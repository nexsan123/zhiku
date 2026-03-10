use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::errors::AppError;

/// Alternative.me Fear & Greed Index API.
const FNG_URL: &str = "https://api.alternative.me/fng/?limit=1";

/// API response structure.
#[derive(Debug, Deserialize)]
struct FngResponse {
    data: Option<Vec<FngDataPoint>>,
}

#[derive(Debug, Deserialize)]
struct FngDataPoint {
    value: String,
    value_classification: String,
}

/// Fetch Fear & Greed index and insert into `macro_data`.
///
/// No API key required.
///
/// # Returns
/// Count of newly inserted data points (0 or 1).
pub async fn fetch_fear_greed(pool: &SqlitePool) -> Result<usize, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    let response = client
        .get(FNG_URL)
        .header("User-Agent", "ZhiKu/0.1 (Financial Intelligence)")
        .send()
        .await
        .map_err(|e| AppError::Network(format!("Fear & Greed request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "Fear & Greed API returned status {}",
            response.status()
        )));
    }

    let fng_data: FngResponse = response.json().await.map_err(|e| {
        AppError::Parse(format!("Fear & Greed parse error: {}", e))
    })?;

    let data_points = fng_data.data.unwrap_or_default();
    let point = match data_points.first() {
        Some(p) => p,
        None => {
            log::warn!("Fear & Greed API returned empty data");
            return Ok(0);
        }
    };

    let value: f64 = point.value.parse().map_err(|e| {
        AppError::Parse(format!(
            "Fear & Greed value '{}' is not a number: {}",
            point.value, e
        ))
    })?;

    let now = Utc::now().to_rfc3339();
    let today = Utc::now().format("%Y-%m-%d").to_string();

    // Check if we already have today's reading
    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM macro_data WHERE indicator = 'fear_greed_index' AND period = ?1",
    )
    .bind(&today)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(format!("Check existing F&G data failed: {}", e)))?;

    if exists > 0 {
        // Update today's value instead of inserting duplicate
        sqlx::query(
            "UPDATE macro_data SET value = ?1, fetched_at = ?2 WHERE indicator = 'fear_greed_index' AND period = ?3",
        )
        .bind(value)
        .bind(&now)
        .bind(&today)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Update F&G data failed: {}", e)))?;

        log::info!(
            "Fear & Greed updated: {} ({})",
            value,
            point.value_classification
        );
        return Ok(0);
    }

    sqlx::query(
        r#"INSERT INTO macro_data (indicator, value, period, source, fetched_at)
           VALUES ('fear_greed_index', ?1, ?2, 'alternative.me', ?3)"#,
    )
    .bind(value)
    .bind(&today)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert F&G data failed: {}", e)))?;

    log::info!(
        "Fear & Greed: {} ({})",
        value,
        point.value_classification
    );
    Ok(1)
}
