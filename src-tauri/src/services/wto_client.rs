use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::errors::AppError;

/// WTO Timeseries API base URL.
/// Requires free API key from https://apiportal.wto.org/
const WTO_BASE_URL: &str = "https://api.wto.org/timeseries/v1/data";

/// WTO indicator codes to fetch: (indicator_code, indicator_name, topic).
/// - TP_A_0010: Merchandise exports, annual, world
/// - TP_A_0020: Merchandise imports, annual, world
const WTO_INDICATORS: &[(&str, &str)] = &[
    ("TP_A_0010", "WTO_MERCH_EXPORTS"),
    ("TP_A_0020", "WTO_MERCH_IMPORTS"),
];

/// WTO API response structure (subset).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WtoDataPoint {
    year: Option<String>,
    value: Option<f64>,
}

/// Fetch WTO trade data and insert into `macro_data`.
///
/// Requires a WTO API key (free registration at https://apiportal.wto.org/).
/// If `api_key` is empty, returns Ok(0) with a log warning (graceful degradation).
///
/// # Returns
/// Total count of newly inserted data points, or `AppError` on failure.
pub async fn fetch_wto_data(pool: &SqlitePool, api_key: &str) -> Result<usize, AppError> {
    if api_key.is_empty() {
        log::warn!("WTO API key not configured -- skipping trade data fetch. Register at https://apiportal.wto.org/");
        return Ok(0);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    let mut total: usize = 0;

    for (indicator_code, indicator_name) in WTO_INDICATORS {
        match fetch_single_indicator(&client, pool, api_key, indicator_code, indicator_name).await {
            Ok(count) => {
                log::info!("WTO [{}]: {} new data points", indicator_name, count);
                total += count;
            }
            Err(e) => {
                log::warn!("WTO [{}] failed: {}", indicator_name, e);
            }
        }
    }

    Ok(total)
}

/// Fetch a single WTO indicator.
async fn fetch_single_indicator(
    client: &reqwest::Client,
    pool: &SqlitePool,
    api_key: &str,
    indicator_code: &str,
    indicator_name: &str,
) -> Result<usize, AppError> {
    // WTO API: reporting_economy=000 (World), partner=000 (World)
    // Get last 5 years of data, sorted descending
    let url = format!(
        "{}?i={}&r=000&p=000&ps=2020,2021,2022,2023,2024,2025&fmt=json&mode=full&lang=1&max=10",
        WTO_BASE_URL, indicator_code
    );

    let response = client
        .get(&url)
        .header("User-Agent", "ZhiKu/0.1 (Financial Intelligence)")
        .header("Ocp-Apim-Subscription-Key", api_key)
        .send()
        .await
        .map_err(|e| {
            AppError::Network(format!("WTO request failed for {}: {}", indicator_code, e))
        })?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "WTO API returned status {} for {}",
            response.status(),
            indicator_code
        )));
    }

    let data_points: Vec<WtoDataPoint> = response.json().await.map_err(|e| {
        AppError::Parse(format!("WTO parse error for {}: {}", indicator_code, e))
    })?;

    let now = Utc::now().to_rfc3339();
    let mut inserted: usize = 0;

    for point in &data_points {
        let value = match point.value {
            Some(v) => v,
            None => continue,
        };
        let period = point.year.as_deref().unwrap_or("unknown");

        // Dedup: skip if (indicator, period) already exists
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM macro_data WHERE indicator = ?1 AND period = ?2 AND source = 'wto'",
        )
        .bind(indicator_name)
        .bind(period)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            AppError::Database(format!("Check existing WTO data failed: {}", e))
        })?;

        if exists > 0 {
            continue;
        }

        sqlx::query(
            r#"INSERT INTO macro_data (indicator, value, period, source, fetched_at)
               VALUES (?1, ?2, ?3, 'wto', ?4)"#,
        )
        .bind(indicator_name)
        .bind(value)
        .bind(period)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::Database(format!("Insert WTO data failed: {}", e))
        })?;

        inserted += 1;
    }

    Ok(inserted)
}
