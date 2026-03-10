use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::ai::{CycleIndicators, CycleReasoning};
use crate::services::{claude_client, summarizer};

/// System prompt for Claude cycle reasoning.
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

/// Run cycle reasoning using Claude API.
///
/// Takes computed indicators, sends to Claude for analysis, returns structured reasoning.
/// If `claude_api_key` is empty or Claude is unavailable, returns a default CycleReasoning
/// with confidence=0.
pub async fn reason_cycle(
    indicators: &CycleIndicators,
    claude_api_key: &str,
) -> Result<CycleReasoning, AppError> {
    if claude_api_key.is_empty() {
        log::warn!("Claude API key not configured — returning default cycle reasoning");
        return Ok(default_reasoning("No API key configured"));
    }

    let indicators_json = serde_json::to_string_pretty(indicators)
        .map_err(|e| AppError::Parse(format!("Failed to serialize indicators: {}", e)))?;

    let user_prompt = format!(
        "Analyze the following financial cycle indicators and provide your structured assessment:\n\n{}",
        indicators_json
    );

    let response = claude_client::analyze(
        &user_prompt,
        Some(CYCLE_SYSTEM_PROMPT),
        claude_api_key,
    )
    .await?;

    if response.is_empty() {
        log::warn!("Claude returned empty response for cycle reasoning");
        return Ok(default_reasoning("Claude returned empty response"));
    }

    parse_reasoning_response(&response)
}

/// Persist a CycleReasoning result to the `ai_analysis` table.
pub async fn persist_reasoning(
    pool: &SqlitePool,
    reasoning: &CycleReasoning,
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
    .bind("claude:claude-sonnet-4-20250514")
    .bind(reasoning.confidence)
    .bind(&reasoning.reasoning_chain)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert cycle_reasoning failed: {}", e)))?;

    log::info!(
        "Cycle reasoning persisted: id={}, confidence={:.2}",
        id,
        reasoning.confidence
    );
    Ok(())
}

/// Fetch the latest CycleReasoning from `ai_analysis` table.
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

/// Parse Claude's response text into a CycleReasoning.
fn parse_reasoning_response(response: &str) -> Result<CycleReasoning, AppError> {
    let trimmed = response.trim();

    // Try direct parse
    if let Ok(reasoning) = serde_json::from_str::<CycleReasoning>(trimmed) {
        return Ok(reasoning);
    }

    // Try stripping markdown code block markers
    let stripped = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(reasoning) = serde_json::from_str::<CycleReasoning>(stripped) {
        return Ok(reasoning);
    }

    // Try extracting first JSON object (reuse summarizer's utility)
    if let Some(json_str) = summarizer::extract_json_object(trimmed) {
        if let Ok(reasoning) = serde_json::from_str::<CycleReasoning>(&json_str) {
            return Ok(reasoning);
        }
    }

    Err(AppError::Parse(format!(
        "Failed to parse Claude cycle reasoning response: {}",
        &trimmed[..trimmed.len().min(200)]
    )))
}

/// Construct a default CycleReasoning when Claude is unavailable.
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
