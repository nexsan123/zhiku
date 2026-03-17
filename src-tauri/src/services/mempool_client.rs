use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::errors::AppError;

/// mempool.space public API base URL. Free, no API key required.
const MEMPOOL_BASE: &str = "https://mempool.space/api";

/// Hashrate response from /api/v1/mining/hashrate/3d.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct HashrateResponse {
    current_hashrate: f64,
    current_difficulty: f64,
}

/// Recommended fees from /api/v1/fees/recommended.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct FeesResponse {
    fastest_fee: u64,
    half_hour_fee: u64,
    hour_fee: u64,
    economy_fee: u64,
    minimum_fee: u64,
}

/// Difficulty adjustment from /api/v1/difficulty-adjustment.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct DifficultyAdjustment {
    progress_percent: f64,
    difficulty_change: f64,
    estimated_retarget_date: u64,
    remaining_blocks: u64,
    remaining_time: f64,
    previous_retarget: f64,
    next_retarget_height: u64,
    time_avg: f64,
    time_offset: f64,
}

/// Fetch BTC network data from mempool.space and insert into `macro_data`.
///
/// No API key required. Fetches:
/// - BTC_HASHRATE: current network hashrate (EH/s)
/// - BTC_FEE_MEDIUM: recommended medium-priority fee (sat/vB)
/// - BTC_DIFFICULTY_PROGRESS: difficulty adjustment progress (%)
///
/// Uses upsert pattern: updates today's value if it already exists,
/// since these are point-in-time values that change frequently.
///
/// # Returns
/// Count of newly inserted/updated data points, or `AppError` on failure.
pub async fn fetch_mempool_data(pool: &SqlitePool) -> Result<usize, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    let now = Utc::now().to_rfc3339();
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let mut total: usize = 0;

    // 1. Hashrate
    match fetch_hashrate(&client).await {
        Ok(hr) => {
            // Convert to EH/s (exahashes) for readability
            let hashrate_eh = hr.current_hashrate / 1e18;
            total += upsert_indicator(pool, "BTC_HASHRATE", hashrate_eh, &today, &now).await?;
            log::info!("mempool: hashrate = {:.2} EH/s", hashrate_eh);
        }
        Err(e) => {
            log::warn!("mempool [hashrate] failed: {}", e);
        }
    }

    // 2. Recommended fees
    match fetch_fees(&client).await {
        Ok(fees) => {
            total += upsert_indicator(
                pool,
                "BTC_FEE_MEDIUM",
                fees.hour_fee as f64,
                &today,
                &now,
            )
            .await?;
            total += upsert_indicator(
                pool,
                "BTC_FEE_FAST",
                fees.fastest_fee as f64,
                &today,
                &now,
            )
            .await?;
            log::info!(
                "mempool: fees fast={} medium={} economy={} sat/vB",
                fees.fastest_fee,
                fees.hour_fee,
                fees.economy_fee
            );
        }
        Err(e) => {
            log::warn!("mempool [fees] failed: {}", e);
        }
    }

    // 3. Difficulty adjustment
    match fetch_difficulty(&client).await {
        Ok(diff) => {
            total += upsert_indicator(
                pool,
                "BTC_DIFFICULTY_PROGRESS",
                diff.progress_percent,
                &today,
                &now,
            )
            .await?;
            total += upsert_indicator(
                pool,
                "BTC_DIFFICULTY_CHANGE",
                diff.difficulty_change,
                &today,
                &now,
            )
            .await?;
            log::info!(
                "mempool: difficulty progress={:.1}%, change={:.2}%",
                diff.progress_percent,
                diff.difficulty_change
            );
        }
        Err(e) => {
            log::warn!("mempool [difficulty] failed: {}", e);
        }
    }

    log::info!("mempool: {} indicators updated", total);
    Ok(total)
}

/// Fetch hashrate from mempool.space.
async fn fetch_hashrate(client: &reqwest::Client) -> Result<HashrateResponse, AppError> {
    let url = format!("{}/v1/mining/hashrate/3d", MEMPOOL_BASE);

    let response = client
        .get(&url)
        .header("User-Agent", "ZhiKu/0.1 (Financial Intelligence)")
        .send()
        .await
        .map_err(|e| AppError::Network(format!("mempool hashrate request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "mempool hashrate API returned status {}",
            response.status()
        )));
    }

    response.json().await.map_err(|e| {
        AppError::Parse(format!("mempool hashrate parse error: {}", e))
    })
}

/// Fetch recommended fees from mempool.space.
async fn fetch_fees(client: &reqwest::Client) -> Result<FeesResponse, AppError> {
    let url = format!("{}/v1/fees/recommended", MEMPOOL_BASE);

    let response = client
        .get(&url)
        .header("User-Agent", "ZhiKu/0.1 (Financial Intelligence)")
        .send()
        .await
        .map_err(|e| AppError::Network(format!("mempool fees request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "mempool fees API returned status {}",
            response.status()
        )));
    }

    response.json().await.map_err(|e| {
        AppError::Parse(format!("mempool fees parse error: {}", e))
    })
}

/// Fetch difficulty adjustment from mempool.space.
async fn fetch_difficulty(client: &reqwest::Client) -> Result<DifficultyAdjustment, AppError> {
    let url = format!("{}/v1/difficulty-adjustment", MEMPOOL_BASE);

    let response = client
        .get(&url)
        .header("User-Agent", "ZhiKu/0.1 (Financial Intelligence)")
        .send()
        .await
        .map_err(|e| {
            AppError::Network(format!("mempool difficulty request failed: {}", e))
        })?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "mempool difficulty API returned status {}",
            response.status()
        )));
    }

    response.json().await.map_err(|e| {
        AppError::Parse(format!("mempool difficulty parse error: {}", e))
    })
}

/// Upsert a single indicator into `macro_data`.
///
/// If (indicator, period) exists, updates the value.
/// If not, inserts a new row.
///
/// Returns 1 on insert, 0 on update.
async fn upsert_indicator(
    pool: &SqlitePool,
    indicator: &str,
    value: f64,
    period: &str,
    fetched_at: &str,
) -> Result<usize, AppError> {
    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM macro_data WHERE indicator = ?1 AND period = ?2",
    )
    .bind(indicator)
    .bind(period)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        AppError::Database(format!("Check existing mempool data failed: {}", e))
    })?;

    if exists > 0 {
        sqlx::query(
            "UPDATE macro_data SET value = ?1, fetched_at = ?2 WHERE indicator = ?3 AND period = ?4",
        )
        .bind(value)
        .bind(fetched_at)
        .bind(indicator)
        .bind(period)
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::Database(format!("Update mempool data failed: {}", e))
        })?;
        return Ok(0);
    }

    sqlx::query(
        r#"INSERT INTO macro_data (indicator, value, period, source, fetched_at)
           VALUES (?1, ?2, ?3, 'mempool', ?4)"#,
    )
    .bind(indicator)
    .bind(value)
    .bind(period)
    .bind(fetched_at)
    .execute(pool)
    .await
    .map_err(|e| {
        AppError::Database(format!("Insert mempool data failed: {}", e))
    })?;

    Ok(1)
}
