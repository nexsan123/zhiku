use sqlx::SqlitePool;
use tauri::State;

use crate::models::credit::{CountryCyclePosition, DollarTide, GlobalCycleOverview};
use crate::services::{dollar_tide, global_aggregator};

/// Get the full global credit cycle overview (15 countries + aggregation + dollar tide).
#[tauri::command]
pub async fn get_credit_cycle_overview(
    pool: State<'_, SqlitePool>,
) -> Result<GlobalCycleOverview, String> {
    global_aggregator::compute_global_overview(&pool)
        .await
        .map_err(|e| e.to_string())
}

/// Get the current dollar tide state.
#[tauri::command]
pub async fn get_dollar_tide(pool: State<'_, SqlitePool>) -> Result<DollarTide, String> {
    dollar_tide::compute_dollar_tide(&pool)
        .await
        .map_err(|e| e.to_string())
}

/// Get detailed credit cycle data for a single country.
#[tauri::command]
pub async fn get_country_credit_detail(
    pool: State<'_, SqlitePool>,
    country_code: String,
) -> Result<Option<CountryCyclePosition>, String> {
    let tide = dollar_tide::compute_dollar_tide(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let risk_mod = dollar_tide::tide_risk_modifier(&tide);

    let positions = crate::services::credit_cycle_engine::compute_all_positions(&pool, risk_mod)
        .await
        .map_err(|e| e.to_string())?;

    Ok(positions
        .into_iter()
        .find(|c| c.country_code == country_code))
}
