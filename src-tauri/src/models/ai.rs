use serde::{Deserialize, Serialize};

/// AI Brief item for the AI Brief panel.
/// Aggregates recent AI analysis by category for frontend display.
///
/// Used by: commands/ai.rs :: get_ai_brief
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiBriefItem {
    pub category: String,
    pub count: i64,
    pub avg_sentiment: f64,
    pub top_keywords: Vec<String>,
    pub latest_summary: String,
}

// ---------------------------------------------------------------------------
// Cycle Indicators (Layer 2) — Rust-computed from SQLite data
// ---------------------------------------------------------------------------

/// Monetary cycle indicators derived from FRED data (FEDFUNDS, M2SL).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonetaryCycle {
    pub fed_rate: f64,
    pub m2_growth: f64,
    pub rate_direction: String,
    pub policy_stance: String,
}

/// Credit cycle indicators (placeholder — awaiting data source).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditCycle {
    pub credit_spread: f64,
    pub yield_curve: String,
    pub phase: String,
}

/// Economic cycle indicators derived from FRED data (GDP, UNRATE, CPIAUCSL).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EconomicCycle {
    pub gdp_growth: f64,
    pub unemployment: f64,
    pub cpi_inflation: f64,
    pub phase: String,
}

/// Market cycle indicators derived from Yahoo Finance snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketCycle {
    pub sp500_trend: f64,
    pub vix_level: f64,
    pub dxy_trend: f64,
    pub phase: String,
}

/// Sentiment cycle indicators from Fear & Greed + AI news sentiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentimentCycle {
    pub fear_greed: f64,
    pub news_sentiment_avg: f64,
    pub phase: String,
}

/// Geopolitical risk assessment from news categorization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeopoliticalRisk {
    pub risk_level: String,
    pub key_events: Vec<String>,
    pub event_count: i64,
}

/// Commodity cycle indicators from Yahoo Finance (CL=F, GC=F, HG=F, NG=F).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommodityCycle {
    pub oil_price: f64,
    pub oil_trend: f64,
    pub gold_price: f64,
    pub gold_trend: f64,
    pub copper_price: f64,
    pub copper_trend: f64,
    pub natgas_price: f64,
    pub natgas_trend: f64,
    pub phase: String,
}

/// Crypto liquidity signal from CoinGecko (BTC-CG).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoSignal {
    pub btc_price: f64,
    pub btc_trend: f64,
    pub phase: String,
}

/// IMF fiscal snapshot for US and CN (debt/GDP, fiscal balance, GDP growth).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiscalSnapshot {
    pub us_debt_gdp: f64,
    pub cn_debt_gdp: f64,
    pub us_fiscal_balance: f64,
    pub cn_fiscal_balance: f64,
    pub us_gdp_growth: f64,
    pub cn_gdp_growth: f64,
}

/// EIA energy price data (WTI, Brent, spread).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnergyData {
    pub wti_price: f64,
    pub brent_price: f64,
    pub spread: f64,
}

/// Aggregated cycle indicators (Layer 2 output).
/// Computed locally in Rust from SQLite data — no external API call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CycleIndicators {
    pub monetary: MonetaryCycle,
    pub credit: CreditCycle,
    pub economic: EconomicCycle,
    pub market: MarketCycle,
    pub sentiment: SentimentCycle,
    pub geopolitical: GeopoliticalRisk,
    pub commodities: CommodityCycle,
    pub crypto: CryptoSignal,
    pub fiscal: FiscalSnapshot,
    pub energy: EnergyData,
    pub calculated_at: String,
}

// ---------------------------------------------------------------------------
// Cycle Reasoning (Layer 3 → Layer 4) — Claude API output
// ---------------------------------------------------------------------------

/// A single turning signal detected by the AI reasoner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurningSignal {
    pub signal: String,
    pub direction: String,
    pub strength: String,
}

/// Structured cycle reasoning output from Claude API.
/// Persisted in `ai_analysis` table with analysis_type = "cycle_reasoning".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CycleReasoning {
    pub cycle_position: String,
    pub monetary_policy_stage: String,
    pub sentiment_stage: String,
    pub turning_signals: Vec<TurningSignal>,
    pub sector_recommendations: Vec<String>,
    pub tail_risks: Vec<String>,
    pub confidence: f64,
    pub reasoning_chain: String,
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// Five-Layer Reasoning (edict-004 enhanced output)
// ---------------------------------------------------------------------------

/// A single step in the five-layer reasoning chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningStep {
    pub step: u8,
    pub layer: String,
    pub finding: String,
    pub evidence: Vec<String>,
    pub confidence: f64,
}

/// Five-layer reasoning output — the enhanced replacement for CycleReasoning.
/// Persisted in `ai_analysis` with analysis_type = "five_layer_reasoning".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveLayerReasoning {
    /// Global credit cycle position (from GlobalCycleOverview).
    pub global_cycle_phase: String,
    pub global_cycle_phase_zh: String,
    /// Dollar tide state.
    pub dollar_tide_state: String,
    pub dollar_tide_label: String,
    /// Overall cycle position assessment.
    pub cycle_position: String,
    /// Monetary policy stage.
    pub monetary_policy_stage: String,
    /// Sentiment stage.
    pub sentiment_stage: String,
    /// Structured reasoning chain — one step per layer.
    pub reasoning_steps: Vec<ReasoningStep>,
    /// Turning signals detected.
    pub turning_signals: Vec<TurningSignal>,
    /// Sector recommendations.
    pub sector_recommendations: Vec<String>,
    /// Tail risks.
    pub tail_risks: Vec<String>,
    /// Risk alerts from credit engine.
    pub risk_alerts: Vec<String>,
    /// Overall confidence.
    pub confidence: f64,
    pub confidence_grade: String,
    /// Narrative summary (2-3 paragraphs for human consumption).
    pub narrative: String,
    pub timestamp: String,
}
