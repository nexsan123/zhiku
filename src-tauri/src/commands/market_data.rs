use sqlx::SqlitePool;
use tauri::State;

use crate::models::signal::MarketSnap;
use crate::services::{market_radar, yahoo_client};
use crate::services::market_radar::MarketRadar;

/// Get latest market snapshots from the database.
/// Returns most recent entry per symbol.
/// Frontend: invoke('get_market_data')
#[tauri::command]
pub async fn get_market_data(pool: State<'_, SqlitePool>) -> Result<Vec<MarketSnap>, String> {
    let rows = sqlx::query_as::<_, MarketSnap>(
        r#"SELECT m.id, m.symbol, m.price, m.change_pct, m.volume, m.timestamp, m.source
           FROM market_snap m
           INNER JOIN (
               SELECT symbol, MAX(timestamp) as max_ts
               FROM market_snap
               GROUP BY symbol
           ) latest ON m.symbol = latest.symbol AND m.timestamp = latest.max_ts
           ORDER BY m.symbol"#,
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| format!("[DB_ERR] Failed to fetch market data: {}", e))?;

    Ok(rows)
}

/// Trigger a manual Yahoo Finance data fetch. Returns count of new snapshots.
/// Frontend: invoke('fetch_market')
#[tauri::command]
pub async fn fetch_market(pool: State<'_, SqlitePool>) -> Result<usize, String> {
    yahoo_client::fetch_all_quotes(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

/// Compute and return the 7-signal market radar.
/// Frontend: invoke('get_market_radar')
#[tauri::command]
pub async fn get_market_radar(pool: State<'_, SqlitePool>) -> Result<MarketRadar, String> {
    market_radar::compute_radar(pool.inner())
        .await
        .map_err(|e| e.to_string())
}
