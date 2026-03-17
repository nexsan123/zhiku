use chrono::Utc;
use sqlx::SqlitePool;

use crate::errors::AppError;
use crate::models::macro_data::{FredObservation, FredResponse};

/// FRED API base URL.
const FRED_BASE_URL: &str = "https://api.stlouisfed.org/fred/series/observations";

/// Key FRED series IDs for US macro monitoring.
pub const FRED_SERIES: &[(&str, &str)] = &[
    ("FEDFUNDS", "Federal Funds Rate"),
    ("CPIAUCSL", "Consumer Price Index"),
    ("UNRATE", "Unemployment Rate"),
    ("GDP", "Gross Domestic Product"),
    ("M2SL", "M2 Money Supply"),
    ("DGS10", "10-Year Treasury Yield"),
    ("DGS2", "2-Year Treasury Yield"),
];

/// Fetch a single FRED series and insert observations into `macro_data`.
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `series_id` - FRED series identifier (e.g., "FEDFUNDS")
/// * `api_key` - FRED API key (from tauri-plugin-store)
///
/// # Returns
/// Count of newly inserted observations, or `AppError` on failure.
/// If api_key is empty, returns Ok(0) with a log warning (graceful degradation).
pub async fn fetch_series(
    pool: &SqlitePool,
    series_id: &str,
    api_key: &str,
) -> Result<usize, AppError> {
    if api_key.is_empty() {
        log::warn!(
            "FRED API key not configured — skipping fetch for {}",
            series_id
        );
        return Ok(0);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    let url = format!(
        "{}?series_id={}&api_key={}&file_type=json&sort_order=desc&limit=10",
        FRED_BASE_URL, series_id, api_key
    );

    let response = client
        .get(&url)
        .header("User-Agent", "ZhiKu/0.1 (Financial Intelligence)")
        .send()
        .await
        .map_err(|e| AppError::Network(format!("FRED request failed for {}: {}", series_id, e)))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "FRED API returned status {} for {}",
            response.status(),
            series_id
        )));
    }

    let fred_data: FredResponse = response.json().await.map_err(|e| {
        AppError::Parse(format!(
            "Failed to parse FRED response for {}: {}",
            series_id, e
        ))
    })?;

    let now = Utc::now().to_rfc3339();
    let mut inserted: usize = 0;

    for obs in &fred_data.observations {
        if let Some(count) = insert_observation(pool, series_id, obs, &now).await? {
            inserted += count;
        }
    }

    log::info!("FRED [{}]: {} new observations", series_id, inserted);
    Ok(inserted)
}

/// Insert a single FRED observation, skipping "." (missing data marker).
async fn insert_observation(
    pool: &SqlitePool,
    series_id: &str,
    obs: &FredObservation,
    fetched_at: &str,
) -> Result<Option<usize>, AppError> {
    // FRED uses "." to indicate missing/unavailable data
    let value: f64 = match obs.value.parse() {
        Ok(v) => v,
        Err(_) => return Ok(None), // Skip non-numeric values (e.g., ".")
    };

    // Check if this exact (indicator, period) already exists to avoid duplicates
    let exists: bool = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM macro_data WHERE indicator = ?1 AND period = ?2",
    )
    .bind(series_id)
    .bind(&obs.date)
    .fetch_one(pool)
    .await
    .map(|count| count > 0)
    .map_err(|e| AppError::Database(format!("Check existing macro_data failed: {}", e)))?;

    if exists {
        return Ok(Some(0));
    }

    sqlx::query(
        r#"INSERT INTO macro_data (indicator, value, period, source, fetched_at)
           VALUES (?1, ?2, ?3, 'fred', ?4)"#,
    )
    .bind(series_id)
    .bind(value)
    .bind(&obs.date)
    .bind(fetched_at)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert macro_data failed: {}", e)))?;

    Ok(Some(1))
}

/// Fetch all configured FRED series.
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `api_key` - FRED API key
///
/// # Returns
/// Total count of newly inserted observations.
pub async fn fetch_all_series(pool: &SqlitePool, api_key: &str) -> Result<usize, AppError> {
    let mut total = 0;

    for (series_id, label) in FRED_SERIES {
        match fetch_series(pool, series_id, api_key).await {
            Ok(count) => total += count,
            Err(e) => {
                log::warn!("FRED [{}] ({}) failed: {}", series_id, label, e);
                // Continue with next series — do not abort
            }
        }
    }

    Ok(total)
}
