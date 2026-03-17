use sqlx::SqlitePool;
use tauri::State;

use crate::models::macro_data::MacroData;
use crate::services::fred_client;

/// Get all macro data from the database, ordered by fetched date descending.
/// Frontend: invoke('get_macro_data')
#[tauri::command]
pub async fn get_macro_data(pool: State<'_, SqlitePool>) -> Result<Vec<MacroData>, String> {
    // Return the latest observation per indicator (not global LIMIT which
    // would be dominated by whichever source was fetched most recently).
    let rows = sqlx::query_as::<_, MacroData>(
        "SELECT m.id, m.indicator, m.value, m.period, m.source, m.fetched_at
         FROM macro_data m
         INNER JOIN (
             SELECT indicator, MAX(period) AS max_period
             FROM macro_data
             GROUP BY indicator
         ) latest ON m.indicator = latest.indicator AND m.period = latest.max_period
         ORDER BY m.source, m.indicator",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| format!("[DB_ERR] Failed to fetch macro data: {}", e))?;

    Ok(rows)
}

/// Trigger FRED data fetch for all configured series.
/// Reads API key from tauri-plugin-store. If not set, returns 0 (graceful degradation).
/// Frontend: invoke('fetch_fred')
#[tauri::command]
pub async fn fetch_fred(
    pool: State<'_, SqlitePool>,
    app_handle: tauri::AppHandle,
) -> Result<usize, String> {
    // Read FRED API key from store
    let api_key = read_fred_api_key(&app_handle);

    fred_client::fetch_all_series(pool.inner(), &api_key)
        .await
        .map_err(|e| e.to_string())
}

/// Read FRED API key from tauri-plugin-store.
/// Returns empty string if not configured (graceful degradation).
fn read_fred_api_key(app_handle: &tauri::AppHandle) -> String {
    use tauri_plugin_store::StoreExt;

    let store = match app_handle.store("settings.json") {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Failed to open settings store: {}", e);
            return String::new();
        }
    };

    store
        .get("fred_api_key")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}
