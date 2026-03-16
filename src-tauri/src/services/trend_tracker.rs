use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::errors::AppError;
use crate::services::{cycle_reasoner, indicator_engine};

/// A single trend data point for charting.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    pub value: f64,
    pub label: Option<String>,
    pub timestamp: String,
}

/// Summary of an available indicator with latest value and data point count.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndicatorSummary {
    pub indicator: String,
    pub latest_value: f64,
    pub label: Option<String>,
    pub last_updated: String,
    pub data_points: i64,
}

/// Take a snapshot of all computed indicators and persist to indicator_history.
///
/// Called from poll_loop every 6 hours (aligned with cycle reasoning).
/// Returns the number of successfully inserted rows.
pub async fn snapshot_indicators(pool: &SqlitePool) -> Result<usize, AppError> {
    let indicators = indicator_engine::calculate_cycle_indicators(pool).await?;
    let reasoning = cycle_reasoner::get_latest_reasoning(pool).await?;

    let now = Utc::now().to_rfc3339();
    let mut count = 0usize;

    // Cycle phase as numeric encoding
    let phase_num = match indicators.economic.phase.as_str() {
        "recession" => 1.0,
        "recovery" => 2.0,
        "early_expansion" => 3.0,
        "mid_expansion" => 4.0,
        "late_expansion" => 5.0,
        _ => 0.0,
    };
    if insert_snapshot(pool, "cycle_phase", phase_num, Some(&indicators.economic.phase), &now).await
    {
        count += 1;
    }

    // Cycle confidence from latest reasoning
    let confidence = reasoning.as_ref().map(|r| r.confidence).unwrap_or(0.0);
    if insert_snapshot(pool, "cycle_confidence", confidence, None, &now).await {
        count += 1;
    }

    // Fear & Greed index
    if insert_snapshot(pool, "fear_greed", indicators.sentiment.fear_greed, None, &now).await {
        count += 1;
    }

    // Position bias (same logic as qt_rest::handle_adjustment_factors)
    let base_position: f64 = match indicators.economic.phase.as_str() {
        "recession" => 0.2,
        "recovery" => 0.5,
        "early_expansion" => 0.7,
        "mid_expansion" => 0.75,
        "late_expansion" => 0.6,
        _ => 0.5,
    };
    let sentiment_adj = if indicators.sentiment.fear_greed < 20.0 {
        -0.15
    } else if indicators.sentiment.fear_greed < 35.0 {
        -0.05
    } else if indicators.sentiment.fear_greed > 80.0 {
        -0.1
    } else {
        0.0
    };
    let position_bias = (base_position + sentiment_adj).clamp(0.1, 0.95);
    if insert_snapshot(pool, "position_bias", position_bias, None, &now).await {
        count += 1;
    }

    // Risk multiplier (same logic as qt_rest::handle_adjustment_factors)
    let vix_factor: f64 = if indicators.market.vix_level > 30.0 {
        0.5
    } else if indicators.market.vix_level > 25.0 {
        0.7
    } else if indicators.market.vix_level > 20.0 {
        0.85
    } else {
        1.0
    };
    let geo_factor = match indicators.geopolitical.risk_level.as_str() {
        "critical" => 0.6,
        "high" => 0.75,
        "elevated" => 0.9,
        _ => 1.0,
    };
    let risk_multiplier = (vix_factor * geo_factor).clamp(0.3, 1.0);
    if insert_snapshot(pool, "risk_multiplier", risk_multiplier, None, &now).await {
        count += 1;
    }

    // Raw macro indicators
    if insert_snapshot(pool, "cpi_yoy", indicators.economic.cpi_inflation, None, &now).await {
        count += 1;
    }
    if insert_snapshot(pool, "gdp_growth", indicators.economic.gdp_growth, None, &now).await {
        count += 1;
    }
    if insert_snapshot(pool, "credit_spread", indicators.credit.credit_spread, None, &now).await {
        count += 1;
    }
    if insert_snapshot(pool, "fed_rate", indicators.monetary.fed_rate, None, &now).await {
        count += 1;
    }
    if insert_snapshot(pool, "vix", indicators.market.vix_level, None, &now).await {
        count += 1;
    }

    // S&P 500 latest price from market_snap
    let sp500: f64 = sqlx::query_scalar(
        "SELECT price FROM market_snap WHERE symbol = '^GSPC' ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0.0);
    if insert_snapshot(pool, "sp500_price", sp500, None, &now).await {
        count += 1;
    }

    log::info!("TrendTracker: snapshot {} indicators", count);
    Ok(count)
}

/// Insert a single snapshot row. Returns true on success, false on failure.
/// Failures are logged as warnings (best-effort, does not propagate errors).
async fn insert_snapshot(
    pool: &SqlitePool,
    indicator: &str,
    value: f64,
    label: Option<&str>,
    snapshot_at: &str,
) -> bool {
    match sqlx::query(
        "INSERT INTO indicator_history (indicator, value, label, snapshot_at) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(indicator)
    .bind(value)
    .bind(label)
    .bind(snapshot_at)
    .execute(pool)
    .await
    {
        Ok(_) => true,
        Err(e) => {
            log::warn!("TrendTracker: failed to insert {}: {}", indicator, e);
            false
        }
    }
}

/// Query trend data for a specific indicator over the last N days.
pub async fn get_trend(
    pool: &SqlitePool,
    indicator: &str,
    days: i64,
) -> Result<Vec<TrendPoint>, AppError> {
    let cutoff = format!("-{} days", days);
    let rows: Vec<(f64, Option<String>, String)> = sqlx::query_as(
        "SELECT value, label, snapshot_at FROM indicator_history \
         WHERE indicator = ?1 AND snapshot_at >= datetime('now', ?2) \
         ORDER BY snapshot_at ASC",
    )
    .bind(indicator)
    .bind(&cutoff)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query trend: {}", e)))?;

    Ok(rows
        .into_iter()
        .map(|(value, label, timestamp)| TrendPoint {
            value,
            label,
            timestamp,
        })
        .collect())
}

/// Query all available indicator names with their latest values.
pub async fn get_available_indicators(
    pool: &SqlitePool,
) -> Result<Vec<IndicatorSummary>, AppError> {
    let rows: Vec<(String, f64, Option<String>, String, i64)> = sqlx::query_as(
        "SELECT ih.indicator, ih.value, ih.label, ih.snapshot_at, \
                (SELECT COUNT(*) FROM indicator_history WHERE indicator = ih.indicator) as point_count \
         FROM indicator_history ih \
         INNER JOIN (SELECT indicator, MAX(snapshot_at) as max_ts FROM indicator_history GROUP BY indicator) latest \
         ON ih.indicator = latest.indicator AND ih.snapshot_at = latest.max_ts \
         ORDER BY ih.indicator",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query available indicators: {}", e)))?;

    Ok(rows
        .into_iter()
        .map(
            |(indicator, latest_value, label, last_updated, data_points)| IndicatorSummary {
                indicator,
                latest_value,
                label,
                last_updated,
                data_points,
            },
        )
        .collect())
}
