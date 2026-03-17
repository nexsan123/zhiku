use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::errors::AppError;

/// Yahoo Finance v8 chart API base URL.
const YAHOO_V8_BASE: &str = "https://query1.finance.yahoo.com/v8/finance/chart";

/// Symbols to fetch: (yahoo_symbol, human_label).
/// Covers indices, forex, crypto, and commodities.
pub const YAHOO_SYMBOLS: &[(&str, &str)] = &[
    // Indices
    ("^GSPC", "S&P 500"),
    ("^DJI", "Dow Jones"),
    ("^IXIC", "NASDAQ"),
    ("^HSI", "Hang Seng"),
    ("000001.SS", "Shanghai Composite"),
    // Forex
    ("EURUSD=X", "EUR/USD"),
    ("USDJPY=X", "USD/JPY"),
    ("GBPUSD=X", "GBP/USD"),
    ("USDCNY=X", "USD/CNY"),
    // Crypto
    ("BTC-USD", "Bitcoin"),
    ("ETH-USD", "Ethereum"),
    // Commodities
    ("GC=F", "Gold"),
    ("CL=F", "WTI Crude Oil"),
    ("SI=F", "Silver"),
    ("NG=F", "Natural Gas"),
    ("HG=F", "Copper"),
    // Dollar index (for dollar tide calculation)
    ("DX-Y.NYB", "US Dollar Index"),
];

/// Yahoo v8 chart response (only fields we need).
#[derive(Debug, Deserialize)]
struct YahooChartResponse {
    chart: Option<YahooChart>,
}

#[derive(Debug, Deserialize)]
struct YahooChart {
    result: Option<Vec<YahooChartResult>>,
}

#[derive(Debug, Deserialize)]
struct YahooChartResult {
    meta: Option<YahooMeta>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct YahooMeta {
    regular_market_price: Option<f64>,
    #[serde(default)]
    regular_market_volume: Option<f64>,
    #[serde(default)]
    previous_close: Option<f64>,
}

/// Build a shared HTTP client with required headers for Yahoo Finance.
fn build_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))
}

/// Fetch all configured Yahoo Finance symbols and insert into `market_snap`.
///
/// Single symbol failure is logged and skipped (no abort).
///
/// # Returns
/// Total count of newly inserted market snapshots.
pub async fn fetch_all_quotes(pool: &SqlitePool) -> Result<usize, AppError> {
    let client = build_client()?;
    let mut total: usize = 0;

    for (symbol, label) in YAHOO_SYMBOLS {
        match fetch_single_quote(&client, pool, symbol, label).await {
            Ok(count) => {
                log::info!("Yahoo [{}] ({}): {} inserted", symbol, label, count);
                total += count;
            }
            Err(e) => {
                log::warn!("Yahoo [{}] ({}) failed: {}", symbol, label, e);
                // Continue with next symbol
            }
        }
    }

    Ok(total)
}

/// Fetch a single symbol from Yahoo v8 chart API.
async fn fetch_single_quote(
    client: &reqwest::Client,
    pool: &SqlitePool,
    symbol: &str,
    _label: &str,
) -> Result<usize, AppError> {
    let url = format!("{}/{}?range=1d&interval=1d", YAHOO_V8_BASE, symbol);

    let response = client
        .get(&url)
        .header("User-Agent", "ZhiKu/0.1 (Financial Intelligence)")
        .header("Cookie", "consent=yes")
        .send()
        .await
        .map_err(|e| AppError::Network(format!("Yahoo request failed for {}: {}", symbol, e)))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "Yahoo API returned status {} for {}",
            response.status(),
            symbol
        )));
    }

    let text = response.text().await.map_err(|e| {
        AppError::Network(format!("Yahoo response read failed for {}: {}", symbol, e))
    })?;

    let parsed: YahooChartResponse = serde_json::from_str(&text).map_err(|e| {
        // Log first 500 chars for debugging if parse fails
        let preview = if text.len() > 500 { &text[..500] } else { &text };
        log::warn!("Yahoo parse failed for {}. Preview: {}", symbol, preview);
        AppError::Parse(format!("Yahoo parse error for {}: {}", symbol, e))
    })?;

    let meta = parsed
        .chart
        .and_then(|c| c.result)
        .and_then(|r| r.into_iter().next())
        .and_then(|r| r.meta)
        .ok_or_else(|| AppError::Parse(format!("Yahoo: no data in response for {}", symbol)))?;

    let price = meta
        .regular_market_price
        .ok_or_else(|| AppError::Parse(format!("Yahoo: no price for {}", symbol)))?;

    // Calculate change percent from previous close if available
    let change_pct = meta.previous_close.map(|prev| {
        if prev > 0.0 {
            ((price - prev) / prev) * 100.0
        } else {
            0.0
        }
    });

    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"INSERT INTO market_snap (symbol, price, change_pct, volume, timestamp, source)
           VALUES (?1, ?2, ?3, ?4, ?5, 'yahoo')"#,
    )
    .bind(symbol)
    .bind(price)
    .bind(change_pct)
    .bind(meta.regular_market_volume)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert market_snap failed for {}: {}", symbol, e)))?;

    Ok(1)
}
