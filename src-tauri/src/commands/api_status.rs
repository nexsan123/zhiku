use sqlx::SqlitePool;
use tauri::State;

use crate::models::news::{ApiStatus, ApiStatusResponse};

/// Get status of all tracked API services, enriched with freshness info.
/// Frontend: invoke('get_api_status')
/// Returns: Vec<ApiStatusResponse> with freshness ("live"/"recent"/"aging"/"stale"/"expired"/"unknown")
///          and minutesAgo (minutes since last_check, null if unknown)
#[tauri::command]
pub async fn get_api_status(pool: State<'_, SqlitePool>) -> Result<Vec<ApiStatusResponse>, String> {
    let rows = sqlx::query_as::<_, ApiStatus>(
        "SELECT service, status, last_check, last_error, response_ms
         FROM api_status ORDER BY service",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| format!("[DB_ERR] Failed to fetch API status: {}", e))?;

    Ok(rows.into_iter().map(ApiStatusResponse::from).collect())
}

/// Update the status of a specific API service.
/// Frontend: invoke('update_api_status', { service, status, errorMsg })
#[tauri::command]
pub async fn update_api_status(
    pool: State<'_, SqlitePool>,
    service: String,
    status: String,
    error_msg: Option<String>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"INSERT INTO api_status (service, status, last_check, last_error)
           VALUES (?1, ?2, ?3, ?4)
           ON CONFLICT(service) DO UPDATE SET
             status = excluded.status,
             last_check = excluded.last_check,
             last_error = excluded.last_error"#,
    )
    .bind(&service)
    .bind(&status)
    .bind(&now)
    .bind(&error_msg)
    .execute(pool.inner())
    .await
    .map_err(|e| format!("[DB_ERR] Failed to update API status: {}", e))?;

    Ok(())
}
