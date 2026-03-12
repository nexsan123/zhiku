use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::credit::confidence_grade;
use crate::services::{claude_client, game_map};

// ---------------------------------------------------------------------------
// Scenario data structures
// ---------------------------------------------------------------------------

/// A single scenario for a policy vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    /// Scenario identifier.
    pub id: String,
    /// Which policy vector this scenario belongs to.
    pub policy_vector: String,
    /// Scenario title.
    pub title: String,
    /// Description of what would happen.
    pub description: String,
    /// Probability estimate (0.0-1.0).
    pub probability: f64,
    /// Previous probability (for showing change).
    pub previous_probability: f64,
    /// Reason for probability change.
    pub change_reason: String,
    /// Assets affected and expected direction.
    pub asset_impacts: Vec<AssetImpact>,
    /// Confidence grade.
    pub confidence_grade: String,
    /// When this scenario was last updated.
    pub updated_at: String,
}

/// Impact on a specific asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImpact {
    pub symbol: String,
    pub direction: String, // "bullish", "bearish", "neutral"
    pub magnitude: String, // "small", "moderate", "large"
}

/// Full scenario matrix output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioMatrix {
    pub scenarios: Vec<Scenario>,
    pub active_vectors: Vec<String>,
    #[serde(default)]
    pub generated_at: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Get active scenarios from the database.
pub async fn get_active_scenarios(pool: &SqlitePool) -> Result<ScenarioMatrix, AppError> {
    // Try to load from ai_analysis
    let row: Option<(String,)> = sqlx::query_as(
        r#"SELECT output FROM ai_analysis
           WHERE analysis_type = 'scenario'
           ORDER BY created_at DESC LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query scenarios failed: {}", e)))?;

    match row {
        Some((json_str,)) => {
            let matrix: ScenarioMatrix = serde_json::from_str(&json_str).map_err(|e| {
                AppError::Parse(format!("Failed to parse stored scenarios: {}", e))
            })?;
            Ok(matrix)
        }
        None => Ok(ScenarioMatrix {
            scenarios: Vec::new(),
            active_vectors: Vec::new(),
            generated_at: Utc::now().to_rfc3339(),
        }),
    }
}

/// Generate/update scenarios using Claude.
///
/// Reads current policy vector activity, feeds to Claude for scenario generation,
/// persists results to ai_analysis.
pub async fn update_scenarios(
    pool: &SqlitePool,
    config: &crate::services::ai_config::ResolvedAiConfig,
) -> Result<ScenarioMatrix, AppError> {
    if config.api_key.is_empty() {
        log::warn!("Claude API key not configured — returning empty scenario matrix");
        return Ok(ScenarioMatrix {
            scenarios: Vec::new(),
            active_vectors: Vec::new(),
            generated_at: Utc::now().to_rfc3339(),
        });
    }

    // Get current policy vectors
    let vectors = game_map::get_policy_vectors(pool).await?;
    let dynamics = game_map::get_bilateral_dynamics(pool).await?;

    // Only generate scenarios for active vectors (activity >= 0.4)
    let active: Vec<&game_map::PolicyVector> =
        vectors.iter().filter(|v| v.activity >= 0.4).collect();

    if active.is_empty() {
        return Ok(ScenarioMatrix {
            scenarios: Vec::new(),
            active_vectors: Vec::new(),
            generated_at: Utc::now().to_rfc3339(),
        });
    }

    let active_ids: Vec<String> = active.iter().map(|v| v.id.clone()).collect();

    // Load previous scenarios for comparison
    let previous = get_active_scenarios(pool).await.ok();

    // Build prompt
    let prompt = build_scenario_prompt(&active, &dynamics, &previous);

    let response = claude_client::analyze(&prompt, Some(SCENARIO_SYSTEM_PROMPT), config)
        .await?;

    if response.is_empty() {
        log::warn!("Claude returned empty response for scenario update");
        return get_active_scenarios(pool).await;
    }

    let scenarios = parse_scenario_response(&response, &active_ids, &previous)?;

    let matrix = ScenarioMatrix {
        scenarios,
        active_vectors: active_ids,
        generated_at: Utc::now().to_rfc3339(),
    };

    // Persist
    persist_scenarios(pool, &matrix).await?;

    Ok(matrix)
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

const SCENARIO_SYSTEM_PROMPT: &str = r#"You are a senior geopolitical strategist specializing in US policy analysis for financial markets. Given active policy vectors and bilateral dynamics, generate 2-3 scenarios per active vector.

Respond with ONLY a JSON array (no markdown, no explanation):
[
  {
    "policyVector": "trade",
    "title": "Scenario title",
    "description": "What would happen (2-3 sentences)",
    "probability": 0.0 to 1.0,
    "changeReason": "Why probability changed from last week (or 'initial assessment')",
    "assetImpacts": [
      {"symbol": "^GSPC", "direction": "bearish", "magnitude": "moderate"}
    ],
    "confidenceGrade": "high|reasonable|speculative"
  }
]

Rules:
- Generate 2-3 scenarios per active policy vector
- Probabilities within each vector should sum to roughly 1.0 (mutually exclusive scenarios)
- confidenceGrade: >= 0.8 probability or well-established → "high", 0.5-0.79 → "reasonable", < 0.5 → "speculative"
- Consider bilateral dynamics as modifiers (e.g., US-China tension affects trade scenarios)
- Be specific about asset impacts (use real symbols)
- JSON array only, no other text"#;

fn build_scenario_prompt(
    active: &[&game_map::PolicyVector],
    dynamics: &[game_map::BilateralDynamic],
    previous: &Option<ScenarioMatrix>,
) -> String {
    let mut prompt = String::from("Generate scenarios for the following active US policy vectors:\n\n");

    for v in active {
        prompt.push_str(&format!(
            "Vector: {} ({})\n  Activity: {:.0}% ({})\n  Latest: {}\n  7d news count: {}\n\n",
            v.name, v.name_zh, v.activity * 100.0, v.activity_label, v.latest_headline, v.news_count_7d
        ));
    }

    prompt.push_str("Bilateral dynamics context:\n");
    for d in dynamics {
        prompt.push_str(&format!(
            "  {} ({}): tension={:.0}% ({}), 7d news={}\n",
            d.name, d.name_zh, d.tension * 100.0, d.tension_label, d.news_count_7d
        ));
    }

    if let Some(prev) = previous {
        if !prev.scenarios.is_empty() {
            prompt.push_str("\nPrevious scenarios (update probabilities if warranted):\n");
            for s in &prev.scenarios {
                prompt.push_str(&format!(
                    "  [{}] {} — prev prob: {:.0}%\n",
                    s.policy_vector, s.title, s.probability * 100.0
                ));
            }
        }
    }

    prompt.push_str("\nGenerate updated scenario matrix as JSON array:");
    prompt
}

fn parse_scenario_response(
    response: &str,
    active_ids: &[String],
    previous: &Option<ScenarioMatrix>,
) -> Result<Vec<Scenario>, AppError> {
    let trimmed = response.trim();

    let parsed: Vec<serde_json::Value> = if let Ok(arr) = serde_json::from_str(trimmed) {
        arr
    } else {
        // Try stripping markdown
        let stripped = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        if let Ok(arr) = serde_json::from_str(stripped) {
            arr
        } else {
            // Try extracting JSON array
            if let Some(start) = trimmed.find('[') {
                if let Some(end) = trimmed.rfind(']') {
                    let slice = &trimmed[start..=end];
                    serde_json::from_str(slice).map_err(|e| {
                        AppError::Parse(format!("Scenario JSON parse failed: {}", e))
                    })?
                } else {
                    return Err(AppError::Parse("No JSON array found in scenario response".to_string()));
                }
            } else {
                return Err(AppError::Parse("No JSON array found in scenario response".to_string()));
            }
        }
    };

    let now = Utc::now().to_rfc3339();
    let mut scenarios = Vec::new();

    for item in parsed {
        let vector = item
            .get("policyVector")
            .or_else(|| item.get("policy_vector"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Only keep scenarios for active vectors
        if !active_ids.iter().any(|id| id == &vector) {
            continue;
        }

        let prob = item
            .get("probability")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.3);

        let asset_impacts: Vec<AssetImpact> = item
            .get("assetImpacts")
            .or_else(|| item.get("asset_impacts"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| serde_json::from_value(a.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();

        // Match previous scenario by policy_vector + title to carry forward old probability
        let prev_prob = previous
            .as_ref()
            .and_then(|m| {
                m.scenarios
                    .iter()
                    .find(|s| s.policy_vector == vector && s.title == title)
                    .map(|s| s.probability)
            })
            .unwrap_or(0.0);

        scenarios.push(Scenario {
            id: Uuid::new_v4().to_string(),
            policy_vector: vector,
            title,
            description: item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            probability: prob,
            previous_probability: prev_prob,
            change_reason: item
                .get("changeReason")
                .or_else(|| item.get("change_reason"))
                .and_then(|v| v.as_str())
                .unwrap_or("initial assessment")
                .to_string(),
            asset_impacts,
            confidence_grade: confidence_grade(prob).to_string(),
            updated_at: now.clone(),
        });
    }

    Ok(scenarios)
}

async fn persist_scenarios(pool: &SqlitePool, matrix: &ScenarioMatrix) -> Result<(), AppError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let output_json = serde_json::to_string(matrix)
        .map_err(|e| AppError::Parse(format!("Failed to serialize scenarios: {}", e)))?;

    sqlx::query(
        r#"INSERT INTO ai_analysis
           (id, analysis_type, input_ids, output, model, confidence, reasoning_chain, source_urls, created_at)
           VALUES (?1, 'scenario', NULL, ?2, 'claude:claude-sonnet-4-20250514', NULL, NULL, NULL, ?3)"#,
    )
    .bind(&id)
    .bind(&output_json)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert scenario failed: {}", e)))?;

    log::info!(
        "Scenario matrix persisted: {} scenarios for {} vectors",
        matrix.scenarios.len(),
        matrix.active_vectors.len()
    );

    Ok(())
}
