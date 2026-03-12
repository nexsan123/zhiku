use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::ai::{
    CycleIndicators, CycleReasoning, FiveLayerReasoning, ReasoningStep, TurningSignal,
};
use crate::models::credit::{confidence_grade, GlobalCycleOverview};
use crate::services::ai_config::ResolvedAiConfig;
use crate::services::{ai_router, summarizer};

// ===========================================================================
// Legacy cycle reasoning (kept for backward compat)
// ===========================================================================

/// System prompt for Claude cycle reasoning (legacy).
const CYCLE_SYSTEM_PROMPT: &str = r#"You are a senior financial cycle analyst. Given a set of macroeconomic and market indicators, produce a structured JSON assessment.

IMPORTANT: Credit cycle data is currently unavailable. Base your reasoning on the other 5 indicator categories.

Respond with ONLY a JSON object (no markdown, no explanation) matching this exact structure:
{
  "cyclePosition": "early_expansion|mid_expansion|late_expansion|recession|recovery",
  "monetaryPolicyStage": "hiking|pausing|cutting|qe|qt",
  "sentimentStage": "panic|fear|caution|neutral|optimism|euphoria",
  "turningSignals": [{"signal": "description", "direction": "bullish|bearish", "strength": "weak|moderate|strong"}],
  "sectorRecommendations": ["defensive", "cyclical", "tech", ...],
  "tailRisks": ["risk description", ...],
  "confidence": 0.0 to 1.0,
  "reasoningChain": "Step-by-step explanation of your reasoning",
  "timestamp": "ISO 8601 timestamp"
}

Rules:
- confidence: 0.0 (no confidence) to 1.0 (very high confidence)
- turningSignals: 0-5 signals, only include if evidence is clear
- sectorRecommendations: 2-5 sectors
- tailRisks: 1-3 risks
- reasoningChain: 2-4 sentences explaining your logic
- JSON only, no other text"#;

/// Run legacy cycle reasoning using configured AI provider.
pub async fn reason_cycle(
    indicators: &CycleIndicators,
    config: &ResolvedAiConfig,
    provider: &str,
) -> Result<CycleReasoning, AppError> {
    if provider != "ollama" && config.api_key.is_empty() {
        log::warn!("AI API key not configured — returning default cycle reasoning");
        return Ok(default_reasoning("No API key configured"));
    }

    let indicators_json = serde_json::to_string_pretty(indicators)
        .map_err(|e| AppError::Parse(format!("Failed to serialize indicators: {}", e)))?;

    let user_prompt = format!(
        "Analyze the following financial cycle indicators and provide your structured assessment:\n\n{}",
        indicators_json
    );

    let response = ai_router::reason(&user_prompt, Some(CYCLE_SYSTEM_PROMPT), config, provider)
        .await?;

    if response.is_empty() {
        return Ok(default_reasoning("Claude returned empty response"));
    }

    parse_reasoning_response(&response)
}

/// Persist a CycleReasoning to ai_analysis.
pub async fn persist_reasoning(
    pool: &SqlitePool,
    reasoning: &CycleReasoning,
    model_label: &str,
) -> Result<(), AppError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let output_json = serde_json::to_string(reasoning)
        .map_err(|e| AppError::Parse(format!("Failed to serialize reasoning: {}", e)))?;

    sqlx::query(
        r#"INSERT INTO ai_analysis
           (id, analysis_type, input_ids, output, model, confidence, reasoning_chain, source_urls, created_at)
           VALUES (?1, 'cycle_reasoning', NULL, ?2, ?3, ?4, ?5, NULL, ?6)"#,
    )
    .bind(&id)
    .bind(&output_json)
    .bind(model_label)
    .bind(reasoning.confidence)
    .bind(&reasoning.reasoning_chain)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert cycle_reasoning failed: {}", e)))?;

    log::info!("Cycle reasoning persisted: id={}, model={}, confidence={:.2}", id, model_label, reasoning.confidence);
    Ok(())
}

/// Fetch the latest CycleReasoning from ai_analysis.
pub async fn get_latest_reasoning(
    pool: &SqlitePool,
) -> Result<Option<CycleReasoning>, AppError> {
    let row: Option<(String,)> = sqlx::query_as(
        r#"SELECT output FROM ai_analysis
           WHERE analysis_type = 'cycle_reasoning'
           ORDER BY created_at DESC LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query cycle_reasoning failed: {}", e)))?;

    match row {
        Some((json_str,)) => {
            let reasoning: CycleReasoning = serde_json::from_str(&json_str).map_err(|e| {
                AppError::Parse(format!("Failed to parse stored cycle_reasoning: {}", e))
            })?;
            Ok(Some(reasoning))
        }
        None => Ok(None),
    }
}

fn parse_reasoning_response(response: &str) -> Result<CycleReasoning, AppError> {
    let trimmed = response.trim();

    if let Ok(r) = serde_json::from_str::<CycleReasoning>(trimmed) {
        return Ok(r);
    }

    let stripped = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(r) = serde_json::from_str::<CycleReasoning>(stripped) {
        return Ok(r);
    }

    if let Some(json_str) = summarizer::extract_json_object(trimmed) {
        if let Ok(r) = serde_json::from_str::<CycleReasoning>(&json_str) {
            return Ok(r);
        }
    }

    Err(AppError::Parse(format!(
        "Failed to parse cycle reasoning: {}",
        &trimmed[..trimmed.len().min(200)]
    )))
}

fn default_reasoning(reason: &str) -> CycleReasoning {
    CycleReasoning {
        cycle_position: "unknown".to_string(),
        monetary_policy_stage: "unknown".to_string(),
        sentiment_stage: "unknown".to_string(),
        turning_signals: Vec::new(),
        sector_recommendations: Vec::new(),
        tail_risks: Vec::new(),
        confidence: 0.0,
        reasoning_chain: reason.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    }
}

// ===========================================================================
// Five-Layer Reasoning (edict-004)
// ===========================================================================

const FIVE_LAYER_SYSTEM_PROMPT: &str = r#"You are a senior financial cycle analyst with expertise in credit cycles, dollar dynamics, and geopolitical intelligence. Given a comprehensive five-layer data input, produce a structured JSON assessment.

Respond with ONLY a JSON object (no markdown, no explanation) matching this exact structure:
{
  "cyclePosition": "early_expansion|mid_expansion|late_expansion|recession|recovery",
  "monetaryPolicyStage": "hiking|pausing|cutting|qe|qt",
  "sentimentStage": "panic|fear|caution|neutral|optimism|euphoria",
  "reasoningSteps": [
    {"step": 1, "layer": "physical", "finding": "Energy prices stable...", "evidence": ["WTI at $72", "NG flat"], "confidence": 0.8},
    {"step": 2, "layer": "credit", "finding": "Global credit cycle in tightening...", "evidence": ["US gap +5.2pp", "DSR 14.3%"], "confidence": 0.85},
    {"step": 3, "layer": "dollar", "finding": "Dollar ebbing...", "evidence": ["DXY +3.2% 3m", "Fed hiking"], "confidence": 0.7},
    {"step": 4, "layer": "geopolitical", "finding": "Trade tensions elevated...", "evidence": ["5 tariff news this week"], "confidence": 0.6},
    {"step": 5, "layer": "sentiment", "finding": "Market cautious...", "evidence": ["F&G index 38", "VIX 22"], "confidence": 0.75}
  ],
  "turningSignals": [{"signal": "description", "direction": "bullish|bearish", "strength": "weak|moderate|strong"}],
  "sectorRecommendations": ["defensive", "cyclical", "tech"],
  "tailRisks": ["risk description"],
  "narrative": "2-3 paragraph narrative summary connecting all layers for human consumption. Write in a professional intelligence briefing style.",
  "confidence": 0.0 to 1.0,
  "timestamp": "ISO 8601 timestamp"
}

Rules:
- reasoningSteps: EXACTLY 5 steps, one per layer (physical, credit, dollar, geopolitical, sentiment)
- Each step must cite specific data points as evidence
- confidence per step and overall: 0.0-1.0
- narrative: 2-3 paragraphs, professional tone, connect the dots across layers
- turningSignals: 0-5 signals
- sectorRecommendations: 2-5 sectors
- tailRisks: 1-3 risks
- JSON only, no other text"#;

/// Input data for five-layer reasoning.
pub struct FiveLayerInput {
    /// Credit layer: GlobalCycleOverview
    pub cycle_overview: GlobalCycleOverview,
    /// Legacy indicators (monetary, economic, market, sentiment, geopolitical)
    pub indicators: CycleIndicators,
    /// Latest deep analysis summaries (from Phase C)
    pub intelligence_summaries: Vec<String>,
    /// Active scenario titles (from Phase D)
    pub active_scenarios: Vec<String>,
}

/// Run five-layer reasoning using configured AI provider.
pub async fn reason_five_layer(
    _pool: &SqlitePool,
    input: &FiveLayerInput,
    config: &ResolvedAiConfig,
    provider: &str,
) -> Result<FiveLayerReasoning, AppError> {
    if provider != "ollama" && config.api_key.is_empty() {
        log::warn!("AI API key not configured — returning default five-layer reasoning");
        return Ok(default_five_layer(&input.cycle_overview, "No API key configured"));
    }

    let prompt = build_five_layer_prompt(input)?;

    let response =
        ai_router::reason(&prompt, Some(FIVE_LAYER_SYSTEM_PROMPT), config, provider).await?;

    if response.is_empty() {
        return Ok(default_five_layer(&input.cycle_overview, "Claude returned empty response"));
    }

    parse_five_layer_response(&response, &input.cycle_overview)
}

/// Persist five-layer reasoning to ai_analysis.
pub async fn persist_five_layer(
    pool: &SqlitePool,
    reasoning: &FiveLayerReasoning,
    model_label: &str,
) -> Result<(), AppError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let output_json = serde_json::to_string(reasoning)
        .map_err(|e| AppError::Parse(format!("Failed to serialize five-layer reasoning: {}", e)))?;

    sqlx::query(
        r#"INSERT INTO ai_analysis
           (id, analysis_type, input_ids, output, model, confidence, reasoning_chain, source_urls, created_at)
           VALUES (?1, 'five_layer_reasoning', NULL, ?2, ?3, ?4, NULL, NULL, ?5)"#,
    )
    .bind(&id)
    .bind(&output_json)
    .bind(model_label)
    .bind(reasoning.confidence)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert five_layer_reasoning failed: {}", e)))?;

    log::info!(
        "Five-layer reasoning persisted: id={}, model={}, confidence={:.2}",
        id,
        model_label,
        reasoning.confidence
    );
    Ok(())
}

/// Fetch the latest five-layer reasoning.
pub async fn get_latest_five_layer(
    pool: &SqlitePool,
) -> Result<Option<FiveLayerReasoning>, AppError> {
    let row: Option<(String,)> = sqlx::query_as(
        r#"SELECT output FROM ai_analysis
           WHERE analysis_type = 'five_layer_reasoning'
           ORDER BY created_at DESC LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query five_layer_reasoning failed: {}", e)))?;

    match row {
        Some((json_str,)) => {
            let reasoning: FiveLayerReasoning =
                serde_json::from_str(&json_str).map_err(|e| {
                    AppError::Parse(format!("Failed to parse five_layer_reasoning: {}", e))
                })?;
            Ok(Some(reasoning))
        }
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Five-layer internals
// ---------------------------------------------------------------------------

fn build_five_layer_prompt(input: &FiveLayerInput) -> Result<String, AppError> {
    let mut prompt = String::from("=== FIVE-LAYER DATA INPUT ===\n\n");

    // Layer 1: Physical
    prompt.push_str("--- LAYER 1: PHYSICAL ---\n");
    prompt.push_str(&format!(
        "WTI Crude: trend {}%\nVIX: {}\n",
        input.indicators.market.sp500_trend, // reuse as proxy
        input.indicators.market.vix_level
    ));
    prompt.push_str("\n");

    // Layer 2: Credit
    prompt.push_str("--- LAYER 2: CREDIT ---\n");
    prompt.push_str(&format!(
        "Global Phase: {} ({})\n",
        input.cycle_overview.global_phase_label,
        serde_json::to_string(&input.cycle_overview.global_phase).unwrap_or_default()
    ));
    prompt.push_str(&format!(
        "Global Percentile: {:.0}%\nConfidence: {:.2}\n",
        input.cycle_overview.global_percentile, input.cycle_overview.confidence
    ));

    // Core countries summary
    for c in input
        .cycle_overview
        .countries
        .iter()
        .filter(|c| c.tier == crate::models::credit::CountryTier::Core)
    {
        prompt.push_str(&format!(
            "  {} ({}): {} [gap={}, dsr={}, impulse={}]\n",
            c.country_name,
            c.country_code,
            c.phase_label,
            c.indicators.credit_gdp_gap.map_or("N/A".to_string(), |v| format!("{:.1}", v)),
            c.indicators.debt_service_ratio.map_or("N/A".to_string(), |v| format!("{:.1}", v)),
            c.indicators.credit_impulse.map_or("N/A".to_string(), |v| format!("{:.2}", v)),
        ));
    }

    // Risk alerts
    if !input.cycle_overview.risk_alerts.is_empty() {
        prompt.push_str("Risk Alerts:\n");
        for alert in &input.cycle_overview.risk_alerts {
            prompt.push_str(&format!("  [{}] {}\n", alert.severity, alert.alert));
        }
    }
    prompt.push_str("\n");

    // Layer 3: Dollar Tide
    prompt.push_str("--- LAYER 3: DOLLAR TIDE ---\n");
    let tide = &input.cycle_overview.dollar_tide;
    prompt.push_str(&format!(
        "State: {} ({})\nDXY 3m: {:.2}%, 6m: {:.2}%\nFed: {}\nM2 growth: {:.2}%\nYield spread: {:.2}pp\n\n",
        tide.tide_label,
        serde_json::to_string(&tide.tide_state).unwrap_or_default(),
        tide.dxy_trend_3m, tide.dxy_trend_6m, tide.fed_policy, tide.m2_growth, tide.yield_spread
    ));

    // Layer 4: Geopolitical + Intelligence
    prompt.push_str("--- LAYER 4: GEOPOLITICAL ---\n");
    prompt.push_str(&format!(
        "Risk level: {}\nEvents (24h): {}\n",
        input.indicators.geopolitical.risk_level, input.indicators.geopolitical.event_count
    ));
    if !input.intelligence_summaries.is_empty() {
        prompt.push_str("Recent intelligence:\n");
        for (i, s) in input.intelligence_summaries.iter().take(5).enumerate() {
            prompt.push_str(&format!("  [{}] {}\n", i + 1, s));
        }
    }
    if !input.active_scenarios.is_empty() {
        prompt.push_str("Active scenarios:\n");
        for s in input.active_scenarios.iter().take(5) {
            prompt.push_str(&format!("  - {}\n", s));
        }
    }
    prompt.push_str("\n");

    // Layer 5: Sentiment
    prompt.push_str("--- LAYER 5: SENTIMENT ---\n");
    prompt.push_str(&format!(
        "Fear & Greed: {:.0}\nNews sentiment avg: {:.2}\nPhase: {}\n",
        input.indicators.sentiment.fear_greed,
        input.indicators.sentiment.news_sentiment_avg,
        input.indicators.sentiment.phase
    ));

    // Monetary context
    prompt.push_str(&format!(
        "\nFed Rate: {:.2}%\nM2 Growth: {:.2}%\nRate Direction: {}\nPolicy Stance: {}\n",
        input.indicators.monetary.fed_rate,
        input.indicators.monetary.m2_growth,
        input.indicators.monetary.rate_direction,
        input.indicators.monetary.policy_stance
    ));

    prompt.push_str("\n=== END DATA ===\n\nProvide your five-layer assessment as JSON:");
    Ok(prompt)
}

fn parse_five_layer_response(
    response: &str,
    overview: &GlobalCycleOverview,
) -> Result<FiveLayerReasoning, AppError> {
    let trimmed = response.trim();

    let parsed: serde_json::Value = if let Ok(v) = serde_json::from_str(trimmed) {
        v
    } else {
        let stripped = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        if let Ok(v) = serde_json::from_str(stripped) {
            v
        } else if let Some(json_str) = summarizer::extract_json_object(trimmed) {
            serde_json::from_str(&json_str).map_err(|e| {
                AppError::Parse(format!("Five-layer JSON parse failed: {}", e))
            })?
        } else {
            return Err(AppError::Parse(format!(
                "Failed to extract JSON from five-layer response: {}",
                &trimmed[..trimmed.len().min(200)]
            )));
        }
    };

    let conf = parsed.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.3);

    // Parse reasoning steps
    let reasoning_steps: Vec<ReasoningStep> = parsed
        .get("reasoningSteps")
        .or_else(|| parsed.get("reasoning_steps"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    Some(ReasoningStep {
                        step: s.get("step").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                        layer: s.get("layer").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                        finding: s.get("finding").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        evidence: s.get("evidence")
                            .and_then(|v| v.as_array())
                            .map(|a| a.iter().filter_map(|e| e.as_str().map(|s| s.to_string())).collect())
                            .unwrap_or_default(),
                        confidence: s.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Parse turning signals
    let turning_signals: Vec<TurningSignal> = parsed
        .get("turningSignals")
        .or_else(|| parsed.get("turning_signals"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    // Parse string arrays
    let sector_recs: Vec<String> = parsed
        .get("sectorRecommendations")
        .or_else(|| parsed.get("sector_recommendations"))
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|s| s.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let tail_risks: Vec<String> = parsed
        .get("tailRisks")
        .or_else(|| parsed.get("tail_risks"))
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|s| s.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let risk_alerts: Vec<String> = overview
        .risk_alerts
        .iter()
        .map(|a| format!("[{}] {}", a.severity, a.alert))
        .collect();

    Ok(FiveLayerReasoning {
        global_cycle_phase: serde_json::to_string(&overview.global_phase).unwrap_or_default(),
        global_cycle_phase_zh: overview.global_phase_label.clone(),
        dollar_tide_state: serde_json::to_string(&overview.dollar_tide.tide_state)
            .unwrap_or_default(),
        dollar_tide_label: overview.dollar_tide.tide_label.clone(),
        cycle_position: parsed.get("cyclePosition")
            .or_else(|| parsed.get("cycle_position"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        monetary_policy_stage: parsed.get("monetaryPolicyStage")
            .or_else(|| parsed.get("monetary_policy_stage"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        sentiment_stage: parsed.get("sentimentStage")
            .or_else(|| parsed.get("sentiment_stage"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        reasoning_steps,
        turning_signals,
        sector_recommendations: sector_recs,
        tail_risks,
        risk_alerts,
        confidence: conf,
        confidence_grade: confidence_grade(conf).to_string(),
        narrative: parsed.get("narrative")
            .and_then(|v| v.as_str())
            .unwrap_or("Narrative unavailable")
            .to_string(),
        timestamp: Utc::now().to_rfc3339(),
    })
}

fn default_five_layer(overview: &GlobalCycleOverview, reason: &str) -> FiveLayerReasoning {
    FiveLayerReasoning {
        global_cycle_phase: serde_json::to_string(&overview.global_phase).unwrap_or_default(),
        global_cycle_phase_zh: overview.global_phase_label.clone(),
        dollar_tide_state: serde_json::to_string(&overview.dollar_tide.tide_state)
            .unwrap_or_default(),
        dollar_tide_label: overview.dollar_tide.tide_label.clone(),
        cycle_position: "unknown".to_string(),
        monetary_policy_stage: "unknown".to_string(),
        sentiment_stage: "unknown".to_string(),
        reasoning_steps: Vec::new(),
        turning_signals: Vec::new(),
        sector_recommendations: Vec::new(),
        tail_risks: Vec::new(),
        risk_alerts: overview
            .risk_alerts
            .iter()
            .map(|a| format!("[{}] {}", a.severity, a.alert))
            .collect(),
        confidence: 0.0,
        confidence_grade: "speculative".to_string(),
        narrative: reason.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    }
}
