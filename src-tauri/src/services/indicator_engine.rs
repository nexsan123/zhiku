use chrono::Utc;
use sqlx::SqlitePool;

use crate::errors::AppError;
use crate::models::ai::{
    CreditCycle, CycleIndicators, EconomicCycle, GeopoliticalRisk, MarketCycle, MonetaryCycle,
    SentimentCycle,
};

/// Calculate all 6 cycle indicators from SQLite data.
///
/// Each sub-indicator is computed independently. If a database query fails,
/// the sub-indicator returns a safe default value (never panics).
pub async fn calculate_cycle_indicators(
    pool: &SqlitePool,
) -> Result<CycleIndicators, AppError> {
    let monetary = calculate_monetary(pool).await;
    let credit = calculate_credit(pool).await;
    let economic = calculate_economic(pool).await;
    let market = calculate_market(pool).await;
    let sentiment = calculate_sentiment(pool).await;
    let geopolitical = calculate_geopolitical(pool).await;

    Ok(CycleIndicators {
        monetary,
        credit,
        economic,
        market,
        sentiment,
        geopolitical,
        calculated_at: Utc::now().to_rfc3339(),
    })
}

// ---------------------------------------------------------------------------
// Helper: date arithmetic for YoY calculations
// ---------------------------------------------------------------------------

/// Parse "YYYY-MM-DD" (or "YYYY-MM") and return the month difference
/// between two period strings.  Returns `(later - earlier)` in months.
fn months_diff(earlier: &str, later: &str) -> i32 {
    let parse = |s: &str| -> (i32, i32) {
        let parts: Vec<&str> = s.split('-').collect();
        let y = parts.first().and_then(|v| v.parse().ok()).unwrap_or(0);
        let m = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(1);
        (y, m)
    };
    let (ly, lm) = parse(later);
    let (ey, em) = parse(earlier);
    (ly - ey) * 12 + (lm - em)
}

/// Year-over-year growth rate (%) from a sorted (DESC) vec of (value, period).
/// Looks for a row >= 11 months before the latest row.
/// Fallback: annualise from earliest available data.
fn yoy_growth(rows: &[(f64, String)]) -> f64 {
    if rows.len() < 2 {
        return 0.0;
    }
    let latest_val = rows[0].0;
    let latest_period = &rows[0].1;

    // Find value ~12 months ago (first row whose distance >= 11 months)
    let year_ago = rows.iter().find(|(_, p)| months_diff(p, latest_period) >= 11);

    match year_ago {
        Some((ya_val, _)) if *ya_val > 0.0 => (latest_val - ya_val) / ya_val * 100.0,
        _ => {
            // Fallback: annualise from earliest available row
            let earliest = &rows[rows.len() - 1];
            let months = months_diff(&earliest.1, latest_period).max(1) as f64;
            if earliest.0 > 0.0 {
                ((latest_val / earliest.0).powf(12.0 / months) - 1.0) * 100.0
            } else {
                0.0
            }
        }
    }
}

/// Get the latest value for a FRED indicator from macro_data.
async fn latest_macro_value(pool: &SqlitePool, indicator: &str) -> f64 {
    sqlx::query_scalar(
        "SELECT value FROM macro_data WHERE indicator = ?1 ORDER BY period DESC LIMIT 1",
    )
    .bind(indicator)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// Monetary cycle
// ---------------------------------------------------------------------------

/// Monetary cycle: FEDFUNDS rate + M2 YoY growth + direction + stance.
async fn calculate_monetary(pool: &SqlitePool) -> MonetaryCycle {
    // Latest federal funds rate
    let fed_rate: f64 = sqlx::query_scalar(
        "SELECT value FROM macro_data WHERE indicator = 'FEDFUNDS' ORDER BY period DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0.0);

    // M2 year-over-year growth rate (%)
    // Fetch up to 24 months of M2SL data — enough for YoY with margin
    let m2_rows: Vec<(f64, String)> = sqlx::query_as(
        "SELECT value, period FROM macro_data WHERE indicator = 'M2SL' ORDER BY period DESC LIMIT 24",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let m2_growth = yoy_growth(&m2_rows);

    // Rate direction: compare latest two FEDFUNDS values
    let rate_rows: Vec<(f64,)> = sqlx::query_as(
        "SELECT value FROM macro_data WHERE indicator = 'FEDFUNDS' ORDER BY period DESC LIMIT 2",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let rate_direction = if rate_rows.len() >= 2 {
        if rate_rows[0].0 > rate_rows[1].0 {
            "hiking"
        } else if rate_rows[0].0 < rate_rows[1].0 {
            "cutting"
        } else {
            "pausing"
        }
    } else {
        "unknown"
    }
    .to_string();

    // Policy stance heuristic
    let policy_stance = if fed_rate > 4.0 {
        "hawkish"
    } else if fed_rate < 2.0 {
        "dovish"
    } else {
        "neutral"
    }
    .to_string();

    MonetaryCycle {
        fed_rate,
        m2_growth,
        rate_direction,
        policy_stance,
    }
}

// ---------------------------------------------------------------------------
// Credit cycle — real DGS10/DGS2 spread
// ---------------------------------------------------------------------------

/// Credit cycle: yield-curve spread (DGS10 - DGS2) from macro_data.
async fn calculate_credit(pool: &SqlitePool) -> CreditCycle {
    let dgs10 = latest_macro_value(pool, "DGS10").await;
    let dgs2 = latest_macro_value(pool, "DGS2").await;

    let credit_spread = if dgs10 > 0.0 && dgs2 > 0.0 {
        dgs10 - dgs2
    } else {
        0.0
    };

    let yield_curve = if credit_spread < -0.2 {
        "inverted"
    } else if credit_spread < 0.2 {
        "flat"
    } else {
        "normal"
    }
    .to_string();

    let phase = determine_credit_phase(credit_spread);

    CreditCycle {
        credit_spread,
        yield_curve,
        phase,
    }
}

/// Determine credit-cycle phase from the 10Y-2Y spread.
fn determine_credit_phase(spread: f64) -> String {
    if spread < -0.2 {
        "tightening"
    } else if spread < 0.2 {
        "neutral"
    } else if spread <= 1.0 {
        "easing"
    } else {
        "accommodative"
    }
    .to_string()
}

// ---------------------------------------------------------------------------
// Economic cycle
// ---------------------------------------------------------------------------

/// Economic cycle: GDP QoQ-annualised growth + unemployment + CPI YoY inflation + phase.
async fn calculate_economic(pool: &SqlitePool) -> EconomicCycle {
    // GDP quarter-over-quarter annualised growth rate (%)
    // GDP is quarterly in macro_data; take latest two quarters
    let gdp_rows: Vec<(f64,)> = sqlx::query_as(
        "SELECT value FROM macro_data WHERE indicator = 'GDP' ORDER BY period DESC LIMIT 2",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let gdp_growth = if gdp_rows.len() >= 2 && gdp_rows[1].0 > 0.0 {
        ((gdp_rows[0].0 / gdp_rows[1].0).powf(4.0) - 1.0) * 100.0
    } else {
        0.0
    };

    // Unemployment rate — already a percentage, use as-is
    let unemployment: f64 = sqlx::query_scalar(
        "SELECT value FROM macro_data WHERE indicator = 'UNRATE' ORDER BY period DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0.0);

    // CPI year-over-year inflation rate (%)
    // Fetch up to 24 months of CPIAUCSL — enough for YoY with margin
    let cpi_rows: Vec<(f64, String)> = sqlx::query_as(
        "SELECT value, period FROM macro_data WHERE indicator = 'CPIAUCSL' ORDER BY period DESC LIMIT 24",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let cpi_inflation = yoy_growth(&cpi_rows);

    let phase = determine_economic_phase(gdp_growth, unemployment, cpi_inflation);

    EconomicCycle {
        gdp_growth,
        unemployment,
        cpi_inflation,
        phase,
    }
}

/// Determine economic cycle phase from GDP growth %, unemployment %, CPI inflation %.
fn determine_economic_phase(gdp: f64, unemployment: f64, cpi: f64) -> String {
    if gdp < 0.0 {
        "recession".to_string()
    } else if gdp > 0.0 && gdp < 2.0 && unemployment > 6.0 {
        "recovery".to_string()
    } else if gdp > 0.0 && unemployment < 5.0 && cpi < 3.0 {
        "early_expansion".to_string()
    } else if gdp > 0.0 && cpi > 4.0 {
        "late_expansion".to_string()
    } else if gdp > 0.0 {
        "mid_expansion".to_string()
    } else {
        "unknown".to_string()
    }
}

// ---------------------------------------------------------------------------
// Market cycle
// ---------------------------------------------------------------------------

/// Market cycle: S&P 500 trend + VIX + DXY + phase.
async fn calculate_market(pool: &SqlitePool) -> MarketCycle {
    let sp500_trend = calculate_price_trend(pool, "^GSPC").await;
    let vix_level = latest_price(pool, "^VIX").await;
    let dxy_trend = calculate_price_trend(pool, "DX-Y.NYB").await;

    let phase = determine_market_phase(sp500_trend, vix_level);

    MarketCycle {
        sp500_trend,
        vix_level,
        dxy_trend,
        phase,
    }
}

/// Get the latest price for a symbol from market_snap.
async fn latest_price(pool: &SqlitePool, symbol: &str) -> f64 {
    sqlx::query_scalar(
        "SELECT price FROM market_snap WHERE symbol = ?1 ORDER BY timestamp DESC LIMIT 1",
    )
    .bind(symbol)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0.0)
}

/// Calculate price trend as percentage change (latest vs 24h ago).
async fn calculate_price_trend(pool: &SqlitePool, symbol: &str) -> f64 {
    // Latest price
    let latest = latest_price(pool, symbol).await;
    if latest == 0.0 {
        return 0.0;
    }

    // Price ~24h ago (closest row before 24h cutoff)
    let older: Option<f64> = sqlx::query_scalar(
        r#"SELECT price FROM market_snap
           WHERE symbol = ?1
             AND timestamp <= datetime('now', '-24 hours')
           ORDER BY timestamp DESC LIMIT 1"#,
    )
    .bind(symbol)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match older {
        Some(old) if old > 0.0 => ((latest - old) / old) * 100.0,
        _ => 0.0,
    }
}

/// Determine market phase from S&P 500 trend and VIX level.
fn determine_market_phase(sp500_trend: f64, vix: f64) -> String {
    if sp500_trend < -20.0 {
        "bear".to_string()
    } else if sp500_trend < -10.0 || vix > 30.0 {
        "correction".to_string()
    } else if sp500_trend > 0.0 && vix < 20.0 {
        "bull".to_string()
    } else {
        "recovery".to_string()
    }
}

// ---------------------------------------------------------------------------
// Sentiment cycle
// ---------------------------------------------------------------------------

/// Sentiment cycle: Fear & Greed index + news sentiment average + phase.
async fn calculate_sentiment(pool: &SqlitePool) -> SentimentCycle {
    // Fear & Greed index from macro_data
    let fear_greed: f64 = sqlx::query_scalar(
        "SELECT value FROM macro_data WHERE indicator = 'fear_greed_index' ORDER BY fetched_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(50.0);

    // Average news sentiment from ai_analysis (last 24h, news_summary type)
    let news_sentiment_avg: f64 = sqlx::query_scalar(
        r#"SELECT COALESCE(AVG(confidence), 0.0) FROM ai_analysis
           WHERE analysis_type = 'news_summary'
             AND created_at >= datetime('now', '-24 hours')"#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0.0);

    let phase = determine_sentiment_phase(fear_greed);

    SentimentCycle {
        fear_greed,
        news_sentiment_avg,
        phase,
    }
}

/// Determine sentiment phase from Fear & Greed index.
fn determine_sentiment_phase(fear_greed: f64) -> String {
    if fear_greed < 20.0 {
        "panic"
    } else if fear_greed < 35.0 {
        "fear"
    } else if fear_greed < 45.0 {
        "caution"
    } else if fear_greed < 55.0 {
        "neutral"
    } else if fear_greed < 75.0 {
        "optimism"
    } else {
        "euphoria"
    }
    .to_string()
}

// ---------------------------------------------------------------------------
// Geopolitical risk
// ---------------------------------------------------------------------------

/// Geopolitical risk: count + titles of geopolitical news in last 24h.
async fn calculate_geopolitical(pool: &SqlitePool) -> GeopoliticalRisk {
    // Count geopolitical news in last 24h
    let event_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM news
           WHERE category = 'geopolitical'
             AND published_at >= datetime('now', '-24 hours')"#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Top 5 geopolitical news titles
    let title_rows: Vec<(String,)> = sqlx::query_as(
        r#"SELECT title FROM news
           WHERE category = 'geopolitical'
             AND published_at >= datetime('now', '-24 hours')
           ORDER BY published_at DESC LIMIT 5"#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let key_events: Vec<String> = title_rows.into_iter().map(|(t,)| t).collect();

    let risk_level = determine_risk_level(event_count);

    GeopoliticalRisk {
        risk_level,
        key_events,
        event_count,
    }
}

/// Determine geopolitical risk level from event count.
fn determine_risk_level(count: i64) -> String {
    if count == 0 {
        "low"
    } else if count <= 2 {
        "moderate"
    } else if count <= 5 {
        "elevated"
    } else if count <= 10 {
        "high"
    } else {
        "critical"
    }
    .to_string()
}
