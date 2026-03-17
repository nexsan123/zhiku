use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::errors::AppError;

/// EIA v2 API base URL.
const EIA_BASE_URL: &str = "https://api.eia.gov/v2/petroleum/pri/spt/data/";

/// Oil series to fetch: (series_facet, indicator_name).
const EIA_SERIES: &[(&str, &str)] = &[
    ("RWTC", "wti_price"),
    ("RBRTE", "brent_price"),
];

/// EIA v2 API response structure (subset we need).
#[derive(Debug, Deserialize)]
struct EiaResponse {
    response: Option<EiaResponseBody>,
}

#[derive(Debug, Deserialize)]
struct EiaResponseBody {
    data: Option<Vec<EiaDataPoint>>,
}

/// EIA v2 data point.
///
/// `value` is `serde_json::Value` because EIA API v2 returns numeric values
/// as JSON strings (e.g. `"72.15"`) rather than bare numbers.
#[derive(Debug, Deserialize)]
struct EiaDataPoint {
    value: Option<serde_json::Value>,
    period: Option<String>,
}

/// Extract an f64 from a serde_json::Value that may be a Number or a String.
///
/// EIA API v2 returns values as JSON strings (e.g. `"72.15"`).
/// This helper handles both representations defensively.
fn extract_f64(val: &serde_json::Value) -> Option<f64> {
    match val {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Fetch oil prices from EIA API and insert into `macro_data`.
///
/// If `api_key` is empty, returns Ok(0) with a log warning (graceful degradation).
/// Per-series failure is logged and skipped.
///
/// # Returns
/// Total count of newly inserted data points.
pub async fn fetch_oil_prices(pool: &SqlitePool, api_key: &str) -> Result<usize, AppError> {
    if api_key.is_empty() {
        log::warn!("EIA API key not configured — skipping oil price fetch");
        return Ok(0);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    let mut total: usize = 0;

    for (series_facet, indicator_name) in EIA_SERIES {
        match fetch_single_series(&client, pool, api_key, series_facet, indicator_name).await {
            Ok(count) => {
                log::info!("EIA [{}]: {} new data points", indicator_name, count);
                total += count;
            }
            Err(e) => {
                log::warn!("EIA [{}] failed: {}", indicator_name, e);
            }
        }
    }

    Ok(total)
}

/// Fetch a single EIA series.
async fn fetch_single_series(
    client: &reqwest::Client,
    pool: &SqlitePool,
    api_key: &str,
    series_facet: &str,
    indicator_name: &str,
) -> Result<usize, AppError> {
    let url = format!(
        "{}?api_key={}&data[]=value&facets[series][]={}&frequency=weekly&sort[0][column]=period&sort[0][direction]=desc&length=1",
        EIA_BASE_URL, api_key, series_facet
    );

    let response = client
        .get(&url)
        .header("User-Agent", "ZhiKu/0.1 (Financial Intelligence)")
        .send()
        .await
        .map_err(|e| AppError::Network(format!("EIA request failed for {}: {}", series_facet, e)))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "EIA API returned status {} for {}",
            response.status(),
            series_facet
        )));
    }

    // Read body as text first for debugging, then parse
    let body = response.text().await.map_err(|e| {
        AppError::Network(format!("EIA response read failed for {}: {}", series_facet, e))
    })?;

    log::debug!("EIA [{}] raw response: {}", series_facet, &body.chars().take(500).collect::<String>());

    let eia_data: EiaResponse = serde_json::from_str(&body).map_err(|e| {
        AppError::Parse(format!(
            "EIA parse error for {}: {} | body preview: {}",
            series_facet,
            e,
            &body.chars().take(300).collect::<String>()
        ))
    })?;

    let data_points = eia_data
        .response
        .and_then(|r| r.data)
        .unwrap_or_default();

    let now = Utc::now().to_rfc3339();
    let mut inserted: usize = 0;

    for point in &data_points {
        let value = match point.value.as_ref().and_then(extract_f64) {
            Some(v) => v,
            None => continue,
        };
        let period = point.period.as_deref().unwrap_or("unknown");

        // Log first successful value extraction for verification
        if inserted == 0 {
            log::info!(
                "EIA [{}] parsed: value={:.2}, period={}",
                indicator_name,
                value,
                period
            );
        }

        // Check for existing entry to avoid duplicates
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM macro_data WHERE indicator = ?1 AND period = ?2 AND source = 'eia'",
        )
        .bind(indicator_name)
        .bind(period)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(format!("Check existing EIA data failed: {}", e)))?;

        if exists > 0 {
            continue;
        }

        sqlx::query(
            r#"INSERT INTO macro_data (indicator, value, period, source, fetched_at)
               VALUES (?1, ?2, ?3, 'eia', ?4)"#,
        )
        .bind(indicator_name)
        .bind(value)
        .bind(period)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Insert EIA data failed: {}", e)))?;

        inserted += 1;
    }

    Ok(inserted)
}
