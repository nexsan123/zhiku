use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::credit::confidence_grade;
use crate::services::game_map;

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
    provider: &str,
) -> Result<ScenarioMatrix, AppError> {
    if provider != "ollama" && config.api_key.is_empty() {
        log::warn!("AI API key not configured — returning empty scenario matrix");
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

    let response = crate::services::ai_router::reason(&prompt, Some(SCENARIO_SYSTEM_PROMPT), config, provider)
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

const SCENARIO_SYSTEM_PROMPT: &str = r#"你是一位独立的全球地缘博弈策略分析师，专注于政策行为对金融市场的影响。你站在上帝视角，超越一切国家立场。

核心原则：
- 零立场：不替任何国家的政策辩护或批评。美国加关税是博弈行为，中国反制也是博弈行为，一律客观分析其金融影响
- 博弈论思维：每个政策向量都是博弈树的一个节点，分析各方的最优响应和可能演化路径
- 概率校准：概率必须反映真实不确定性，不可因为某个情景"政治正确"就给高概率
- 资产影响必须具体：不说"市场可能波动"，要说"标普500可能跌3-5%因为..."

给定活跃的政策向量和双边关系动态，为每个活跃向量生成 2-3 个情景。

只回复 JSON 数组（无 markdown，无解释）：
[
  {
    "policyVector": "trade",
    "title": "情景标题（中文）",
    "description": "具体会发生什么，通过什么路径影响市场（2-3 句中文）",
    "probability": 0.0 到 1.0,
    "changeReason": "概率变化原因（对比上周）或 '初始评估'",
    "assetImpacts": [
      {"symbol": "^GSPC", "direction": "bearish", "magnitude": "moderate"}
    ],
    "confidenceGrade": "high|reasonable|speculative"
  }
]

规则：
- 用中文回复 title、description、changeReason 字段
- 每个政策向量生成 2-3 个互斥情景，概率之和约为 1.0
- confidenceGrade: 有充分历史先例或多源信息 → "high"，合理推断 → "reasonable"，推测 → "speculative"
- 双边关系作为情景的调节变量（如中美紧张度影响贸易情景概率）
- assetImpacts 必须用真实交易符号，方向和量级要具体
- 不可因政治立场偏好某个情景，概率必须基于证据
- 只输出 JSON 数组，不输出任何其他内容"#;

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
