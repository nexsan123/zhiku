use sqlx::SqlitePool;

use crate::errors::AppError;
use crate::models::credit::{DollarTide, TideState};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute the current dollar tide state from SQLite data.
///
/// Inputs (all from SQLite):
/// - DXY trend: market_snap DX-Y.NYB 3m/6m price trend
/// - Fed policy: macro_data FEDFUNDS direction
/// - M2 growth: macro_data M2SL YoY
/// - Yield spread: macro_data DGS10 - DGS2
///
/// Output: DollarTide struct with tide_state + sub-indicators + confidence.
pub async fn compute_dollar_tide(pool: &SqlitePool) -> Result<DollarTide, AppError> {
    let dxy_trend_3m = dxy_trend(pool, 90).await;
    let dxy_trend_6m = dxy_trend(pool, 180).await;
    let fed_policy = fed_policy_direction(pool).await;
    let m2_growth = m2_yoy_growth(pool).await;
    let yield_spread = yield_curve_spread(pool).await;

    let (tide_state, confidence) =
        determine_tide(dxy_trend_3m, dxy_trend_6m, &fed_policy, m2_growth, yield_spread);

    Ok(DollarTide {
        dxy_trend_3m,
        dxy_trend_6m,
        fed_policy,
        m2_growth,
        yield_spread,
        tide_label: tide_state.display_name_zh().to_string(),
        confidence,
        tide_state,
    })
}

/// Get the dollar tide risk modifier for emerging market countries.
///
/// Returns: +1 (ebbing = EM pressure), 0 (neutral), -1 (rising = EM benefit).
pub fn tide_risk_modifier(tide: &DollarTide) -> i8 {
    match tide.tide_state {
        TideState::Ebbing => 1,
        TideState::Neutral => 0,
        TideState::Rising => -1,
    }
}

// ---------------------------------------------------------------------------
// Sub-indicator computation
// ---------------------------------------------------------------------------

/// DXY percentage trend over N days from market_snap.
async fn dxy_trend(pool: &SqlitePool, days: i32) -> f64 {
    let latest: Option<f64> = sqlx::query_scalar(
        "SELECT price FROM market_snap WHERE symbol = 'DX-Y.NYB' ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let older: Option<f64> = sqlx::query_scalar(
        &format!(
            "SELECT price FROM market_snap WHERE symbol = 'DX-Y.NYB' \
             AND timestamp <= datetime('now', '-{} days') \
             ORDER BY timestamp DESC LIMIT 1",
            days
        ),
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match (latest, older) {
        (Some(l), Some(o)) if o > 0.0 => ((l - o) / o) * 100.0,
        _ => 0.0,
    }
}

/// Federal funds rate direction: "hiking", "pausing", "cutting", "unknown".
async fn fed_policy_direction(pool: &SqlitePool) -> String {
    let rows: Vec<(f64,)> = sqlx::query_as(
        "SELECT value FROM macro_data WHERE indicator = 'FEDFUNDS' ORDER BY period DESC LIMIT 2",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if rows.len() < 2 {
        return "unknown".to_string();
    }

    let diff = rows[0].0 - rows[1].0;
    if diff > 0.10 {
        "hiking".to_string()
    } else if diff < -0.10 {
        "cutting".to_string()
    } else {
        "pausing".to_string()
    }
}

/// M2 money supply year-over-year growth (%).
async fn m2_yoy_growth(pool: &SqlitePool) -> f64 {
    // M2SL is monthly. We need ~12 months apart.
    let rows: Vec<(f64,)> = sqlx::query_as(
        "SELECT value FROM macro_data WHERE indicator = 'M2SL' ORDER BY period DESC LIMIT 13",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if rows.len() < 2 {
        return 0.0;
    }

    let latest = rows[0].0;
    // Use 12th entry back if available, otherwise oldest
    let older_idx = rows.len().min(12);
    let older = rows[older_idx - 1].0;

    if older.abs() < 0.001 {
        return 0.0;
    }

    ((latest - older) / older.abs()) * 100.0
}

/// Yield curve spread: 10Y - 2Y treasury yield (percentage points).
/// Negative = inverted curve.
async fn yield_curve_spread(pool: &SqlitePool) -> f64 {
    let dgs10: Option<f64> = sqlx::query_scalar(
        "SELECT value FROM macro_data WHERE indicator = 'DGS10' ORDER BY period DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let dgs2: Option<f64> = sqlx::query_scalar(
        "SELECT value FROM macro_data WHERE indicator = 'DGS2' ORDER BY period DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match (dgs10, dgs2) {
        (Some(t10), Some(t2)) => t10 - t2,
        _ => 0.0,
    }
}

// ---------------------------------------------------------------------------
// Tide determination logic
// ---------------------------------------------------------------------------

/// Determine dollar tide state from sub-indicators.
///
/// Logic (edict-004):
/// - Ebbing (退潮): DXY rising + hiking + M2 contracting → EM under pressure
/// - Rising (涨潮): DXY falling + cutting + M2 expanding → EM benefiting
/// - Neutral (平潮): mixed signals
///
/// Returns (TideState, confidence).
fn determine_tide(
    dxy_3m: f64,
    dxy_6m: f64,
    fed_policy: &str,
    m2_growth: f64,
    yield_spread: f64,
) -> (TideState, f64) {
    // Score-based system: positive = ebbing (dollar strengthening), negative = rising (dollar weakening)
    let mut score: f64 = 0.0;
    let mut signals: usize = 0;

    // DXY trend (strongest weight)
    let dxy_avg = (dxy_3m + dxy_6m) / 2.0;
    if dxy_avg.abs() > 0.5 {
        score += dxy_avg.signum() * 2.0; // +2 if DXY rising, -2 if falling
        signals += 1;
    }

    // Fed policy
    match fed_policy {
        "hiking" => {
            score += 1.5;
            signals += 1;
        }
        "cutting" => {
            score -= 1.5;
            signals += 1;
        }
        "pausing" => {
            signals += 1; // counts as signal but doesn't move score
        }
        _ => {}
    }

    // M2 growth
    if m2_growth.abs() > 1.0 {
        if m2_growth < 0.0 {
            score += 1.0; // M2 contracting = tightening
        } else {
            score -= 1.0; // M2 expanding = loosening
        }
        signals += 1;
    }

    // Yield spread (inverted curve is a late-cycle tightening signal)
    if yield_spread < -0.5 {
        score += 0.5; // Inverted = tight conditions
        signals += 1;
    } else if yield_spread > 1.5 {
        score -= 0.5; // Steep = accommodative
        signals += 1;
    }

    // Determine state from aggregate score
    let state = if score >= 2.0 {
        TideState::Ebbing
    } else if score <= -2.0 {
        TideState::Rising
    } else {
        TideState::Neutral
    };

    // Confidence based on signal agreement + data availability
    let data_coverage = if signals >= 4 { 1.0 } else { signals as f64 / 4.0 };
    let signal_strength = (score.abs() / 5.0).min(1.0); // normalize to 0-1
    let confidence = (data_coverage * 0.5 + signal_strength * 0.5).min(1.0);

    (state, confidence)
}
