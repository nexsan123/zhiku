use sqlx::SqlitePool;
use tauri::State;

use crate::models::news::ApiStatus;

/// Get status of all tracked API services.
/// Frontend: invoke('get_api_status')
#[tauri::command]
pub async fn get_api_status(pool: State<'_, SqlitePool>) -> Result<Vec<ApiStatus>, String> {
    let rows = sqlx::query_as::<_, ApiStatus>(
        "SELECT service, status, last_check, last_error, response_ms
         FROM api_status ORDER BY service",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| format!("[DB_ERR] Failed to fetch API status: {}", e))?;

    Ok(rows)
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
