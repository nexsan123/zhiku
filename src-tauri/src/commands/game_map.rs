use sqlx::SqlitePool;
use tauri::State;

use crate::services::{game_map, scenario_engine};

/// Get all 6 policy vectors with current activity levels.
#[tauri::command]
pub async fn get_policy_vectors(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<game_map::PolicyVector>, String> {
    game_map::get_policy_vectors(&pool)
        .await
        .map_err(|e| e.to_string())
}

/// Get all 4 bilateral dynamics.
#[tauri::command]
pub async fn get_bilateral_dynamics(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<game_map::BilateralDynamic>, String> {
    game_map::get_bilateral_dynamics(&pool)
        .await
        .map_err(|e| e.to_string())
}

/// Get decision calendar events within the next N days.
#[tauri::command]
pub async fn get_decision_calendar(
    days: Option<i64>,
) -> Result<Vec<game_map::CalendarEvent>, String> {
    let d = days.unwrap_or(90);
    game_map::get_calendar_events(d).map_err(|e| e.to_string())
}

/// Get current active scenarios.
#[tauri::command]
pub async fn get_active_scenarios(
    pool: State<'_, SqlitePool>,
) -> Result<scenario_engine::ScenarioMatrix, String> {
    scenario_engine::get_active_scenarios(&pool)
        .await
        .map_err(|e| e.to_string())
}

/// Trigger Claude to update scenario probabilities.
#[tauri::command]
pub async fn trigger_scenario_update(
    pool: State<'_, SqlitePool>,
    app: tauri::AppHandle,
) -> Result<scenario_engine::ScenarioMatrix, String> {
    // Read reasoning config from store (uses user-configured provider/model/endpoint)
    let (ai_config, provider) = crate::services::ai_config::resolve_reasoning_config(&app);

    scenario_engine::update_scenarios(&pool, &ai_config, &provider)
        .await
        .map_err(|e| e.to_string())
}
