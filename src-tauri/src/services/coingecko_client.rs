use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::errors::AppError;

/// CoinGecko simple price API (no key required).
const COINGECKO_URL: &str = "https://api.coingecko.com/api/v3/simple/price";

/// Coins to fetch.
const COIN_IDS: &str = "bitcoin,ethereum,tether,usd-coin,dai";

/// Stablecoins to check for depeg.
const STABLECOINS: &[&str] = &["tether", "usd-coin", "dai"];

/// CoinGecko price entry.
#[derive(Debug, Deserialize)]
struct CoinPrice {
    usd: Option<f64>,
    #[serde(rename = "usd_24h_change")]
    usd_24h_change: Option<f64>,
    #[serde(rename = "usd_market_cap")]
    usd_market_cap: Option<f64>,
}

/// Depeg severity levels.
#[derive(Debug)]
enum DepegStatus {
    Normal,
    SlightDepeg,
    Depegged,
}

impl DepegStatus {
    fn as_str(&self) -> &'static str {
        match self {
            DepegStatus::Normal => "NORMAL",
            DepegStatus::SlightDepeg => "SLIGHT_DEPEG",
            DepegStatus::Depegged => "DEPEGGED",
        }
    }
}

/// Assess stablecoin depeg status.
fn assess_depeg(price: f64) -> DepegStatus {
    let deviation = (price - 1.0).abs();
    if deviation > 0.01 {
        DepegStatus::Depegged
    } else if deviation > 0.005 {
        DepegStatus::SlightDepeg
    } else {
        DepegStatus::Normal
    }
}

/// Map CoinGecko id to a ticker symbol for market_snap.
fn coin_to_symbol(coin_id: &str) -> &str {
    match coin_id {
        "bitcoin" => "BTC-CG",
        "ethereum" => "ETH-CG",
        "tether" => "USDT-CG",
        "usd-coin" => "USDC-CG",
        "dai" => "DAI-CG",
        other => other,
    }
}

/// Fetch crypto prices from CoinGecko and insert into `market_snap`.
/// Also checks stablecoin depeg and stores status in `macro_data`.
///
/// No API key required. Rate limit: 10-50 req/min on free tier.
/// Uses a single batch request to minimize API calls.
///
/// # Returns
/// Total count of newly inserted market snapshots.
pub async fn fetch_crypto_prices(pool: &SqlitePool) -> Result<usize, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    let url = format!(
        "{}?ids={}&vs_currencies=usd&include_24hr_change=true&include_market_cap=true",
        COINGECKO_URL, COIN_IDS
    );

    let response = client
        .get(&url)
        .header("User-Agent", "ZhiKu/0.1 (Financial Intelligence)")
        .send()
        .await
        .map_err(|e| AppError::Network(format!("CoinGecko request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "CoinGecko API returned status {}",
            response.status()
        )));
    }

    let prices: HashMap<String, CoinPrice> = response.json().await.map_err(|e| {
        AppError::Parse(format!("CoinGecko parse error: {}", e))
    })?;

    let now = Utc::now().to_rfc3339();
    let mut inserted: usize = 0;

    for (coin_id, price_data) in &prices {
        let price = match price_data.usd {
            Some(p) => p,
            None => continue,
        };

        let symbol = coin_to_symbol(coin_id);

        sqlx::query(
            r#"INSERT INTO market_snap (symbol, price, change_pct, volume, timestamp, source)
               VALUES (?1, ?2, ?3, ?4, ?5, 'coingecko')"#,
        )
        .bind(symbol)
        .bind(price)
        .bind(price_data.usd_24h_change)
        .bind(price_data.usd_market_cap) // Store market cap in volume field
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::Database(format!("Insert CoinGecko market_snap failed for {}: {}", coin_id, e))
        })?;

        inserted += 1;

        // Check stablecoin depeg
        if STABLECOINS.contains(&coin_id.as_str()) {
            let depeg = assess_depeg(price);
            let indicator = format!("{}_depeg", coin_id.replace('-', "_"));

            // Store depeg status as a numeric code: 0=NORMAL, 1=SLIGHT_DEPEG, 2=DEPEGGED
            let depeg_value: f64 = match depeg {
                DepegStatus::Normal => 0.0,
                DepegStatus::SlightDepeg => 1.0,
                DepegStatus::Depegged => 2.0,
            };

            let today = Utc::now().format("%Y-%m-%d").to_string();

            // Upsert: update if today's reading exists, insert otherwise
            let exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM macro_data WHERE indicator = ?1 AND period = ?2",
            )
            .bind(&indicator)
            .bind(&today)
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(format!("Check depeg data failed: {}", e)))?;

            if exists > 0 {
                sqlx::query(
                    "UPDATE macro_data SET value = ?1, fetched_at = ?2 WHERE indicator = ?3 AND period = ?4",
                )
                .bind(depeg_value)
                .bind(&now)
                .bind(&indicator)
                .bind(&today)
                .execute(pool)
                .await
                .map_err(|e| AppError::Database(format!("Update depeg data failed: {}", e)))?;
            } else {
                sqlx::query(
                    r#"INSERT INTO macro_data (indicator, value, period, source, fetched_at)
                       VALUES (?1, ?2, ?3, 'coingecko', ?4)"#,
                )
                .bind(&indicator)
                .bind(depeg_value)
                .bind(&today)
                .bind(&now)
                .execute(pool)
                .await
                .map_err(|e| AppError::Database(format!("Insert depeg data failed: {}", e)))?;
            }

            log::info!(
                "Stablecoin {}: price={:.4}, depeg={}",
                coin_id,
                price,
                depeg.as_str()
            );
        }
    }

    log::info!("CoinGecko: {} snapshots inserted", inserted);
    Ok(inserted)
}
