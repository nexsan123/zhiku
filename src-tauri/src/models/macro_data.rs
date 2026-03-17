use serde::{Deserialize, Serialize};

/// Macroeconomic data point from FRED or other sources.
/// Aligns with SQL table `macro_data`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MacroData {
    pub id: i64,
    pub indicator: String,
    pub value: f64,
    pub period: Option<String>,
    pub source: String,
    pub fetched_at: String,
}

/// FRED API response structures for deserialization.
#[derive(Debug, Deserialize)]
pub struct FredResponse {
    pub observations: Vec<FredObservation>,
}

#[derive(Debug, Deserialize)]
pub struct FredObservation {
    pub date: String,
    pub value: String,
}
