// Models for Phase 3+ — defined now to match SQL schema, used later.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Intelligence signal for QuantTerminal integration.
/// Aligns with SQL table `signals`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Signal {
    pub id: String,
    pub signal_type: String,
    pub severity: String,
    pub title: String,
    pub summary: Option<String>,
    pub data: Option<String>,
    pub source_urls: Option<String>,
    pub ai_confidence: Option<f64>,
    pub ai_model: Option<String>,
    pub created_at: String,
    pub pushed_to_qt: bool,
}

/// AI analysis record.
/// Aligns with SQL table `ai_analysis`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiAnalysisRow {
    pub id: String,
    pub analysis_type: String,
    pub input_ids: Option<String>,
    pub output: String,
    pub model: String,
    pub confidence: Option<f64>,
    pub reasoning_chain: Option<String>,
    pub source_urls: Option<String>,
    pub created_at: String,
}

/// Market snapshot from Yahoo Finance or similar.
/// Aligns with SQL table `market_snap`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MarketSnap {
    pub id: i64,
    pub symbol: String,
    pub price: f64,
    pub change_pct: Option<f64>,
    pub volume: Option<f64>,
    pub timestamp: String,
    pub source: String,
}
