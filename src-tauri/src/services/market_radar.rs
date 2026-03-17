use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::errors::AppError;

/// A single signal in the 7-signal market radar.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RadarSignal {
    pub name: String,
    pub bullish: Option<bool>,
    pub detail: String,
}

/// Aggregated market radar result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketRadar {
    pub signals: Vec<RadarSignal>,
    pub verdict: String,
    pub bullish_pct: f64,
    pub timestamp: String,
}

/// Compute the 7-signal market radar from database data.
///
/// Each signal returns `Option<bool>` — `None` means insufficient data.
/// Verdict: >= 57% of known signals bullish -> "BUY", else "CASH".
///
/// Signals:
/// 1. Liquidity (JPY/USD 30d change)
/// 2. Flow Structure (BTC vs QQQ 5d return)
/// 3. Macro Regime (QQQ vs XLP 20d ROC)
/// 4. Technical Trend (BTC > SMA50)
/// 5. Hash Rate (BTC hash rate growth)
/// 6. Mining Cost (BTC > $60K)
/// 7. Fear & Greed (index > 50)
pub async fn compute_radar(pool: &SqlitePool) -> Result<MarketRadar, AppError> {
    let signals = vec![
        compute_liquidity(pool).await,
        compute_flow_structure(pool).await,
        compute_macro_regime(pool).await,
        compute_technical_trend(pool).await,
        compute_hash_rate(pool).await,
        compute_mining_cost(pool).await,
        compute_fear_greed(pool).await,
    ];

    // Calculate verdict
    let known_signals: Vec<&RadarSignal> = signals.iter().filter(|s| s.bullish.is_some()).collect();
    let bullish_count = known_signals.iter().filter(|s| s.bullish == Some(true)).count();

    let bullish_pct = if known_signals.is_empty() {
        0.0
    } else {
        (bullish_count as f64 / known_signals.len() as f64) * 100.0
    };

    let verdict = if bullish_pct >= 57.0 {
        "BUY".to_string()
    } else {
        "CASH".to_string()
    };

    Ok(MarketRadar {
        signals,
        verdict,
        bullish_pct,
        timestamp: Utc::now().to_rfc3339(),
    })
}

/// Get the latest price for a symbol from market_snap.
async fn latest_price(pool: &SqlitePool, symbol: &str) -> Option<f64> {
    sqlx::query_scalar::<_, f64>(
        "SELECT price FROM market_snap WHERE symbol = ?1 ORDER BY timestamp DESC LIMIT 1",
    )
    .bind(symbol)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

/// Get the price from N days ago (approximate: Nth most recent entry).
async fn price_n_days_ago(pool: &SqlitePool, symbol: &str, n: i64) -> Option<f64> {
    sqlx::query_scalar::<_, f64>(
        "SELECT price FROM market_snap WHERE symbol = ?1 ORDER BY timestamp DESC LIMIT 1 OFFSET ?2",
    )
    .bind(symbol)
    .bind(n)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

/// Get the latest macro_data value for an indicator.
async fn latest_macro(pool: &SqlitePool, indicator: &str) -> Option<f64> {
    sqlx::query_scalar::<_, f64>(
        "SELECT value FROM macro_data WHERE indicator = ?1 ORDER BY fetched_at DESC LIMIT 1",
    )
    .bind(indicator)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

/// Signal 1: Liquidity — JPY/USD 30d change rate > -2% -> bullish
async fn compute_liquidity(pool: &SqlitePool) -> RadarSignal {
    let current = latest_price(pool, "USDJPY=X").await;
    let past = price_n_days_ago(pool, "USDJPY=X", 30).await;

    match (current, past) {
        (Some(now), Some(then)) if then > 0.0 => {
            let change_pct = ((now - then) / then) * 100.0;
            // USDJPY rising = JPY weakening. For JPY/USD we invert.
            let jpy_usd_change = -change_pct;
            let bullish = jpy_usd_change > -2.0;
            RadarSignal {
                name: "Liquidity".to_string(),
                bullish: Some(bullish),
                detail: format!("JPY/USD 30d change: {:.2}%", jpy_usd_change),
            }
        }
        _ => RadarSignal {
            name: "Liquidity".to_string(),
            bullish: None,
            detail: "Insufficient JPY/USD data".to_string(),
        },
    }
}

/// Signal 2: Flow Structure — BTC 5d return vs QQQ 5d return, gap < 5% -> bullish
async fn compute_flow_structure(pool: &SqlitePool) -> RadarSignal {
    let btc_now = latest_price(pool, "BTC-USD").await
        .or(latest_price(pool, "BTC-CG").await);
    let btc_past = price_n_days_ago(pool, "BTC-USD", 5).await
        .or(price_n_days_ago(pool, "BTC-CG", 5).await);

    // QQQ not in our Yahoo symbols — use NASDAQ index as proxy
    let qqq_now = latest_price(pool, "^IXIC").await;
    let qqq_past = price_n_days_ago(pool, "^IXIC", 5).await;

    match (btc_now, btc_past, qqq_now, qqq_past) {
        (Some(bn), Some(bp), Some(qn), Some(qp)) if bp > 0.0 && qp > 0.0 => {
            let btc_ret = ((bn - bp) / bp) * 100.0;
            let qqq_ret = ((qn - qp) / qp) * 100.0;
            let gap: f64 = (btc_ret - qqq_ret).abs();
            let bullish = gap < 5.0;
            RadarSignal {
                name: "Flow Structure".to_string(),
                bullish: Some(bullish),
                detail: format!("BTC 5d: {:.1}%, QQQ 5d: {:.1}%, gap: {:.1}%", btc_ret, qqq_ret, gap),
            }
        }
        _ => RadarSignal {
            name: "Flow Structure".to_string(),
            bullish: None,
            detail: "Insufficient BTC/QQQ data".to_string(),
        },
    }
}

/// Signal 3: Macro Regime — QQQ 20d ROC vs XLP 20d ROC, QQQ leading -> bullish
/// Using NASDAQ (^IXIC) as QQQ proxy, S&P 500 (^GSPC) as XLP proxy.
async fn compute_macro_regime(pool: &SqlitePool) -> RadarSignal {
    let qqq_now = latest_price(pool, "^IXIC").await;
    let qqq_past = price_n_days_ago(pool, "^IXIC", 20).await;
    let xlp_now = latest_price(pool, "^GSPC").await;
    let xlp_past = price_n_days_ago(pool, "^GSPC", 20).await;

    match (qqq_now, qqq_past, xlp_now, xlp_past) {
        (Some(qn), Some(qp), Some(xn), Some(xp)) if qp > 0.0 && xp > 0.0 => {
            let qqq_roc = ((qn - qp) / qp) * 100.0;
            let xlp_roc = ((xn - xp) / xp) * 100.0;
            let bullish = qqq_roc > xlp_roc;
            RadarSignal {
                name: "Macro Regime".to_string(),
                bullish: Some(bullish),
                detail: format!("NASDAQ 20d ROC: {:.1}%, S&P 20d ROC: {:.1}%", qqq_roc, xlp_roc),
            }
        }
        _ => RadarSignal {
            name: "Macro Regime".to_string(),
            bullish: None,
            detail: "Insufficient index data".to_string(),
        },
    }
}

/// Signal 4: Technical Trend — BTC > SMA50 -> bullish
/// SMA50 approximated from average of last 50 price entries.
async fn compute_technical_trend(pool: &SqlitePool) -> RadarSignal {
    let current = latest_price(pool, "BTC-USD").await
        .or(latest_price(pool, "BTC-CG").await);

    let sma50: Option<f64> = sqlx::query_scalar::<_, f64>(
        "SELECT AVG(price) FROM (SELECT price FROM market_snap WHERE symbol IN ('BTC-USD', 'BTC-CG') ORDER BY timestamp DESC LIMIT 50)",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match (current, sma50) {
        (Some(price), Some(sma)) if sma > 0.0 => {
            let bullish = price > sma;
            RadarSignal {
                name: "Technical Trend".to_string(),
                bullish: Some(bullish),
                detail: format!("BTC: ${:.0}, SMA50: ${:.0}", price, sma),
            }
        }
        _ => RadarSignal {
            name: "Technical Trend".to_string(),
            bullish: None,
            detail: "Insufficient BTC price history".to_string(),
        },
    }
}

/// Signal 5: Hash Rate — BTC hash rate growth > 3% -> bullish
/// Reads from macro_data if available (populated by mempool.space in future).
async fn compute_hash_rate(pool: &SqlitePool) -> RadarSignal {
    let hash_rate = latest_macro(pool, "btc_hashrate").await;

    match hash_rate {
        Some(_) => {
            // Would need historical comparison — for now, presence of data = neutral
            RadarSignal {
                name: "Hash Rate".to_string(),
                bullish: None,
                detail: "Hash rate data available but growth calculation pending".to_string(),
            }
        }
        None => RadarSignal {
            name: "Hash Rate".to_string(),
            bullish: None,
            detail: "No BTC hash rate data (mempool.space not yet integrated)".to_string(),
        },
    }
}

/// Signal 6: Mining Cost — BTC price > $60K -> bullish (simplified)
async fn compute_mining_cost(pool: &SqlitePool) -> RadarSignal {
    let btc_price = latest_price(pool, "BTC-USD").await
        .or(latest_price(pool, "BTC-CG").await);

    match btc_price {
        Some(price) => {
            let bullish = price > 60_000.0;
            RadarSignal {
                name: "Mining Cost".to_string(),
                bullish: Some(bullish),
                detail: format!("BTC: ${:.0} (threshold: $60K)", price),
            }
        }
        None => RadarSignal {
            name: "Mining Cost".to_string(),
            bullish: None,
            detail: "No BTC price data".to_string(),
        },
    }
}

/// Signal 7: Fear & Greed — F&G > 50 -> bullish
async fn compute_fear_greed(pool: &SqlitePool) -> RadarSignal {
    let fg_value = latest_macro(pool, "fear_greed_index").await;

    match fg_value {
        Some(value) => {
            let bullish = value > 50.0;
            let label = if value >= 75.0 {
                "Extreme Greed"
            } else if value >= 50.0 {
                "Greed"
            } else if value >= 25.0 {
                "Fear"
            } else {
                "Extreme Fear"
            };
            RadarSignal {
                name: "Fear & Greed".to_string(),
                bullish: Some(bullish),
                detail: format!("F&G: {:.0} ({})", value, label),
            }
        }
        None => RadarSignal {
            name: "Fear & Greed".to_string(),
            bullish: None,
            detail: "No Fear & Greed data".to_string(),
        },
    }
}
