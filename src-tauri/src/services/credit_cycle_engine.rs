use sqlx::SqlitePool;

use crate::errors::AppError;
use crate::models::credit::{
    confidence_grade, CreditCyclePhase, CountryCreditData, CountryCyclePosition, CountryTier,
};

/// Country definitions: (BIS code, human name, tier).
const COUNTRIES: &[(&str, &str, CountryTier)] = &[
    // Core
    ("US", "United States", CountryTier::Core),
    ("XM", "Euro Area", CountryTier::Core),
    ("JP", "Japan", CountryTier::Core),
    ("CN", "China", CountryTier::Core),
    // Important
    ("GB", "United Kingdom", CountryTier::Important),
    ("CA", "Canada", CountryTier::Important),
    ("AU", "Australia", CountryTier::Important),
    ("KR", "South Korea", CountryTier::Important),
    ("IN", "India", CountryTier::Important),
    ("BR", "Brazil", CountryTier::Important),
    // Monitor
    ("TR", "Turkey", CountryTier::Monitor),
    ("AR", "Argentina", CountryTier::Monitor),
    ("ZA", "South Africa", CountryTier::Monitor),
    ("SA", "Saudi Arabia", CountryTier::Monitor),
    ("AE", "UAE", CountryTier::Monitor),
];

/// Data reliability scores per country (credit indicator).
/// Sourced from data_reliability.json — hardcoded here for fast lookup.
fn reliability_score(code: &str) -> f64 {
    match code {
        "US" => 0.95,
        "XM" => 0.90,
        "JP" => 0.95,
        "CN" => 0.75, // BIS credit data more reliable than domestic GDP
        "GB" => 0.90,
        "CA" => 0.90,
        "AU" => 0.90,
        "KR" => 0.85,
        "IN" => 0.75,
        "BR" => 0.80,
        "TR" => 0.75, // credit data from BIS more reliable than CPI
        "AR" => 0.70,
        "ZA" => 0.80,
        "SA" => 0.65,
        "AE" => 0.65,
        _ => 0.50,
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute credit cycle positions for all 15 countries.
///
/// Reads BIS indicators from `macro_data` table, applies six-phase positioning
/// rules, attaches confidence and reliability scores.
pub async fn compute_all_positions(
    pool: &SqlitePool,
    dollar_tide_risk: i8,
) -> Result<Vec<CountryCyclePosition>, AppError> {
    let mut positions = Vec::with_capacity(COUNTRIES.len());

    for (code, name, tier) in COUNTRIES {
        let indicators = fetch_country_indicators(pool, code).await;
        let (phase, confidence) = determine_phase(&indicators);
        let reliability = reliability_score(code);
        let period = latest_bis_period(pool, code).await;

        // Adjust confidence by reliability
        let adjusted_confidence = confidence * reliability;

        // Dollar tide risk modifier only applies to EM (Monitor + some Important)
        let risk_mod = match tier {
            CountryTier::Monitor => dollar_tide_risk,
            CountryTier::Important => {
                // Only apply to EM-like Important countries
                match *code {
                    "IN" | "BR" | "KR" => dollar_tide_risk,
                    _ => 0,
                }
            }
            CountryTier::Core => 0,
        };

        positions.push(CountryCyclePosition {
            country_code: code.to_string(),
            country_name: name.to_string(),
            tier: tier.clone(),
            phase_label: phase.display_name_zh().to_string(),
            confidence: adjusted_confidence,
            confidence_grade: confidence_grade(adjusted_confidence).to_string(),
            reliability,
            dollar_tide_risk_modifier: risk_mod,
            data_period: period,
            indicators,
            phase,
        });
    }

    Ok(positions)
}

// ---------------------------------------------------------------------------
// Per-country indicator fetching
// ---------------------------------------------------------------------------

/// Fetch all credit indicators for a single country from macro_data.
async fn fetch_country_indicators(pool: &SqlitePool, code: &str) -> CountryCreditData {
    let credit_gdp_gap = latest_indicator(pool, &format!("BIS_CREDIT_GAP_{}", code)).await;
    let debt_service_ratio = latest_indicator(pool, &format!("BIS_DSR_{}", code)).await;
    let policy_rate = latest_indicator(pool, &format!("BIS_CBPOL_{}", code)).await;

    // Credit growth YoY: computed from two latest BIS_CREDIT observations
    let credit_growth_yoy = compute_yoy_change(pool, &format!("BIS_CREDIT_{}", code)).await;

    // Credit impulse: change in credit growth (second derivative)
    let credit_impulse = compute_credit_impulse(pool, &format!("BIS_CREDIT_{}", code)).await;

    // Property price trend (YoY % from BIS WS_SPP)
    let property_price_trend = latest_indicator(pool, &format!("BIS_SPP_{}", code)).await;

    // Rate direction from two latest policy rate observations
    let rate_direction = compute_rate_direction(pool, &format!("BIS_CBPOL_{}", code)).await;

    CountryCreditData {
        credit_gdp_gap,
        debt_service_ratio,
        credit_growth_yoy,
        credit_impulse,
        property_price_trend,
        policy_rate,
        rate_direction,
    }
}

/// Get the latest value for an indicator from macro_data.
async fn latest_indicator(pool: &SqlitePool, indicator: &str) -> Option<f64> {
    sqlx::query_scalar(
        "SELECT value FROM macro_data WHERE indicator = ?1 ORDER BY period DESC LIMIT 1",
    )
    .bind(indicator)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

/// Compute YoY change from two latest observations of an indicator.
/// Returns percentage change.
async fn compute_yoy_change(pool: &SqlitePool, indicator: &str) -> Option<f64> {
    let rows: Vec<(f64, String)> = sqlx::query_as(
        "SELECT value, period FROM macro_data WHERE indicator = ?1 ORDER BY period DESC LIMIT 5",
    )
    .bind(indicator)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if rows.len() < 2 {
        return None;
    }

    // Latest vs one-year-ago (or oldest available)
    // BIS quarterly data: index 4 = 4 quarters back ≈ 1 year
    let latest = rows[0].0;
    let yoy_idx = if rows.len() >= 5 { 4 } else { rows.len() - 1 };
    let older = rows[yoy_idx].0;

    if older.abs() < 0.001 {
        return None;
    }

    Some(((latest - older) / older.abs()) * 100.0)
}

/// Compute credit impulse (second derivative of credit).
///
/// Credit impulse = change in new credit flow / approximation.
/// We compute: (growth_t - growth_t-1), where growth = quarterly change.
async fn compute_credit_impulse(pool: &SqlitePool, indicator: &str) -> Option<f64> {
    let rows: Vec<(f64,)> = sqlx::query_as(
        "SELECT value FROM macro_data WHERE indicator = ?1 ORDER BY period DESC LIMIT 4",
    )
    .bind(indicator)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if rows.len() < 3 {
        return None;
    }

    // Quarterly changes (flow proxy)
    let flow_t = rows[0].0 - rows[1].0;
    let flow_t1 = rows[1].0 - rows[2].0;

    // Credit impulse = change in flow, normalized by latest level
    if rows[0].0.abs() < 0.001 {
        return None;
    }

    Some(((flow_t - flow_t1) / rows[0].0.abs()) * 100.0)
}

/// Determine rate direction from two latest policy rate observations.
async fn compute_rate_direction(pool: &SqlitePool, indicator: &str) -> String {
    let rows: Vec<(f64,)> = sqlx::query_as(
        "SELECT value FROM macro_data WHERE indicator = ?1 ORDER BY period DESC LIMIT 2",
    )
    .bind(indicator)
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

/// Get the latest BIS data period for a country.
async fn latest_bis_period(pool: &SqlitePool, code: &str) -> String {
    let indicator = format!("BIS_CREDIT_GAP_{}", code);
    let period: Option<String> = sqlx::query_scalar(
        "SELECT period FROM macro_data WHERE indicator = ?1 ORDER BY period DESC LIMIT 1",
    )
    .bind(&indicator)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    period.unwrap_or_else(|| "N/A".to_string())
}

// ---------------------------------------------------------------------------
// Six-phase positioning logic
// ---------------------------------------------------------------------------

/// Determine credit cycle phase from indicators.
///
/// Returns (phase, raw_confidence before reliability adjustment).
///
/// Rules (edict-004):
/// - Easing (放水期): cutting + impulse turning positive + gap low
/// - Leveraging (加杠杆): credit accelerating + impulse positive & rising
/// - Overheating (过热期): gap > +10 or DSR at historical high
/// - Tightening (收水期): hiking + impulse turning negative
/// - Deleveraging (去杠杆): credit contracting + DSR rising
/// - Clearing (出清期): credit bottoming + impulse starting to recover
fn determine_phase(data: &CountryCreditData) -> (CreditCyclePhase, f64) {
    let gap = data.credit_gdp_gap;
    let dsr = data.debt_service_ratio;
    let growth = data.credit_growth_yoy;
    let impulse = data.credit_impulse;
    let rate_dir = data.rate_direction.as_str();

    // Count available indicators for confidence calculation
    let ppt = data.property_price_trend;

    let available = [
        gap.is_some(),
        dsr.is_some(),
        growth.is_some(),
        impulse.is_some(),
        data.policy_rate.is_some(),
        ppt.is_some(),
    ]
    .iter()
    .filter(|&&x| x)
    .count();

    if available < 2 {
        return (CreditCyclePhase::Unknown, 0.20);
    }

    let base_confidence = (available as f64) / 6.0;

    // Rule-based phase determination with priority ordering
    // Priority: Overheating > Deleveraging > Tightening > Leveraging > Easing > Clearing

    // 1. Overheating: gap > +10 or DSR extremely high
    if let Some(g) = gap {
        if g > 10.0 {
            return (CreditCyclePhase::Overheating, base_confidence * 0.95);
        }
    }
    if let Some(d) = dsr {
        if d > 25.0 {
            // DSR > 25% is historically extreme
            return (CreditCyclePhase::Overheating, base_confidence * 0.85);
        }
    }
    // Property price boom + elevated credit gap is classic overheating
    if let (Some(p), Some(g)) = (ppt, gap) {
        if p > 15.0 && g > 4.0 {
            return (CreditCyclePhase::Overheating, base_confidence * 0.80);
        }
    }

    // 2. Deleveraging: credit contracting + DSR still elevated
    if let (Some(g), Some(d)) = (growth, dsr) {
        if g < -2.0 && d > 15.0 {
            return (CreditCyclePhase::Deleveraging, base_confidence * 0.90);
        }
    }
    if let Some(g) = growth {
        if g < -5.0 {
            return (CreditCyclePhase::Deleveraging, base_confidence * 0.80);
        }
    }

    // 3. Tightening: hiking + impulse turning negative
    if rate_dir == "hiking" {
        if let Some(imp) = impulse {
            if imp < -0.5 {
                return (CreditCyclePhase::Tightening, base_confidence * 0.85);
            }
        }
        // Hiking alone is a moderate signal
        if let Some(g) = gap {
            if g > 2.0 {
                return (CreditCyclePhase::Tightening, base_confidence * 0.70);
            }
        }
    }

    // 4. Leveraging: credit accelerating + impulse positive
    if let Some(imp) = impulse {
        if imp > 0.5 {
            if let Some(g) = growth {
                if g > 3.0 {
                    return (CreditCyclePhase::Leveraging, base_confidence * 0.85);
                }
            }
            return (CreditCyclePhase::Leveraging, base_confidence * 0.70);
        }
    }

    // 5. Easing: cutting + impulse turning positive + gap low
    if rate_dir == "cutting" {
        if let Some(imp) = impulse {
            if imp > -0.2 {
                return (CreditCyclePhase::Easing, base_confidence * 0.80);
            }
        }
        return (CreditCyclePhase::Easing, base_confidence * 0.65);
    }

    // 6. Clearing: credit bottoming + impulse starting to recover
    if let Some(imp) = impulse {
        if let Some(g) = growth {
            if g < 0.0 && imp > 0.0 {
                return (CreditCyclePhase::Clearing, base_confidence * 0.75);
            }
        }
    }

    // Fallback: use growth direction as weak signal
    if let Some(g) = growth {
        if g > 0.0 {
            return (CreditCyclePhase::Leveraging, base_confidence * 0.50);
        } else {
            return (CreditCyclePhase::Clearing, base_confidence * 0.45);
        }
    }

    (CreditCyclePhase::Unknown, base_confidence * 0.30)
}
