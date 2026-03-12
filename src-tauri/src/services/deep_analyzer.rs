use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::credit::confidence_grade;
use crate::models::intelligence::{DeepAnalysis, DeepMotiveAnalysis, LayerImpact, NewsCluster};
use crate::services::summarizer;

/// System prompt for Claude deep analysis (second pass).
const DEEP_ANALYSIS_SYSTEM_PROMPT: &str = r#"You are a senior geopolitical and financial intelligence analyst. Given a cluster of related news articles, perform deep analysis to uncover hidden motives and cross-layer impacts.

Respond with ONLY a JSON object (no markdown, no explanation) matching this exact structure:
{
  "surface": "What happened on the surface (1-2 sentences)",
  "connection": "Why these news items are related (1-2 sentences)",
  "deepAnalysis": {
    "primaryMotive": "The primary motive behind these events",
    "secondaryMotive": "A hidden or secondary motive (if any, else empty string)",
    "confidence": 0.0 to 1.0,
    "confidenceGrade": "high|reasonable|speculative"
  },
  "layerImpact": {
    "physical": "Impact on physical layer (energy, food, supply chains) or 'none'",
    "credit": "Impact on credit cycles (borrowing, debt, banking) or 'none'",
    "dollar": "Impact on dollar/capital flows (DXY, EM flows) or 'none'",
    "geopolitical": "Impact on geopolitical balance (alliances, sanctions, conflicts) or 'none'",
    "sentiment": "Impact on market sentiment (fear/greed, narratives) or 'none'"
  },
  "keyObservation": "The single most important takeaway for a financial intelligence analyst"
}

Rules:
- confidence: >= 0.8 for well-supported analysis, 0.5-0.79 for reasonable inference, < 0.5 for speculation
- confidenceGrade must match: >= 0.8 → "high", 0.5-0.79 → "reasonable", < 0.5 → "speculative"
- Look beyond surface narratives for hidden motives (economic warfare, sanctions evasion, resource control)
- Consider how events connect to broader US-China dynamics, dollar hegemony, energy transitions
- JSON only, no other text"#;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Perform deep analysis on a news cluster using configured AI provider.
///
/// Fetches news details from SQLite, builds a prompt with all titles + summaries,
/// sends to AI provider, parses the structured response.
pub async fn analyze_cluster(
    pool: &SqlitePool,
    cluster: &NewsCluster,
    config: &crate::services::ai_config::ResolvedAiConfig,
    provider: &str,
) -> Result<DeepAnalysis, AppError> {
    if provider != "ollama" && config.api_key.is_empty() {
        log::warn!("AI API key not configured — returning default deep analysis");
        return Ok(default_analysis(cluster));
    }

    // Fetch news details for the cluster
    let news_details = fetch_cluster_news(pool, &cluster.news_ids).await?;

    let user_prompt = build_prompt(cluster, &news_details);

    let response =
        crate::services::ai_router::reason(&user_prompt, Some(DEEP_ANALYSIS_SYSTEM_PROMPT), config, provider)
            .await?;

    if response.is_empty() {
        log::warn!("Claude returned empty response for deep analysis");
        return Ok(default_analysis(cluster));
    }

    let mut analysis = parse_deep_analysis(&response, cluster)?;

    // Attach source URLs for traceability (ZK-01)
    analysis.source_urls = fetch_source_urls(pool, &cluster.news_ids).await;

    Ok(analysis)
}

/// Persist a DeepAnalysis result to the ai_analysis table.
pub async fn persist_analysis(
    pool: &SqlitePool,
    analysis: &DeepAnalysis,
) -> Result<(), AppError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let output_json = serde_json::to_string(analysis)
        .map_err(|e| AppError::Parse(format!("Failed to serialize deep analysis: {}", e)))?;

    let source_urls_json = serde_json::to_string(&analysis.source_urls)
        .unwrap_or_else(|_| "[]".to_string());

    sqlx::query(
        r#"INSERT INTO ai_analysis
           (id, analysis_type, input_ids, output, model, confidence, reasoning_chain, source_urls, created_at)
           VALUES (?1, 'deep_analysis', ?2, ?3, 'claude:claude-sonnet-4-20250514', ?4, ?5, ?6, ?7)"#,
    )
    .bind(&id)
    .bind(&analysis.cluster_id)
    .bind(&output_json)
    .bind(analysis.deep_analysis.confidence)
    .bind(&analysis.key_observation)
    .bind(&source_urls_json)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert deep_analysis failed: {}", e)))?;

    log::info!(
        "Deep analysis persisted: cluster={}, confidence={:.2}",
        analysis.cluster_id,
        analysis.deep_analysis.confidence
    );
    Ok(())
}

/// Fetch the latest deep analyses from the database.
pub async fn get_recent_analyses(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<DeepAnalysis>, AppError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"SELECT output FROM ai_analysis
           WHERE analysis_type = 'deep_analysis'
           ORDER BY created_at DESC LIMIT ?1"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query deep_analysis failed: {}", e)))?;

    let mut analyses = Vec::new();
    for (json_str,) in rows {
        if let Ok(analysis) = serde_json::from_str::<DeepAnalysis>(&json_str) {
            analyses.push(analysis);
        }
    }
    Ok(analyses)
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Fetch news titles + summaries for building the prompt.
async fn fetch_cluster_news(
    pool: &SqlitePool,
    news_ids: &[String],
) -> Result<Vec<(String, String)>, AppError> {
    let mut results = Vec::new();

    for nid in news_ids {
        let row: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT title, ai_summary FROM news WHERE id = ?1",
        )
        .bind(nid)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(format!("Fetch news {} failed: {}", nid, e)))?;

        if let Some((title, summary)) = row {
            results.push((title, summary.unwrap_or_default()));
        }
    }

    Ok(results)
}

/// Fetch source URLs for traceability.
async fn fetch_source_urls(pool: &SqlitePool, news_ids: &[String]) -> Vec<String> {
    let mut urls = Vec::new();
    for nid in news_ids {
        let url: Option<String> = sqlx::query_scalar(
            "SELECT source_url FROM news WHERE id = ?1",
        )
        .bind(nid)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        if let Some(u) = url {
            if !u.is_empty() {
                urls.push(u);
            }
        }
    }
    urls
}

/// Build the user prompt for Claude deep analysis.
fn build_prompt(cluster: &NewsCluster, news_details: &[(String, String)]) -> String {
    let mut prompt = format!(
        "Analyze this cluster of {} related news articles.\n\
         Cluster topic hint: {}\n\
         Regions: {}\n\
         Key entities: {}\n\n\
         --- Articles ---\n",
        cluster.news_count,
        cluster.topic_hint,
        cluster.regions.join(", "),
        cluster.entities.join(", ")
    );

    for (i, (title, summary)) in news_details.iter().enumerate() {
        prompt.push_str(&format!(
            "\n[{}] Title: {}\nSummary: {}\n",
            i + 1,
            title,
            if summary.is_empty() {
                "(no summary)"
            } else {
                summary
            }
        ));
    }

    prompt.push_str("\nProvide your deep analysis as JSON:");
    prompt
}

/// Parse Claude's response into DeepAnalysis.
fn parse_deep_analysis(
    response: &str,
    cluster: &NewsCluster,
) -> Result<DeepAnalysis, AppError> {
    let trimmed = response.trim();

    // Try direct parse
    let parsed: serde_json::Value = if let Ok(v) = serde_json::from_str(trimmed) {
        v
    } else {
        // Try stripping markdown
        let stripped = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        if let Ok(v) = serde_json::from_str(stripped) {
            v
        } else if let Some(json_str) = summarizer::extract_json_object(trimmed) {
            serde_json::from_str(&json_str).map_err(|e| {
                AppError::Parse(format!("Deep analysis JSON parse failed: {}", e))
            })?
        } else {
            return Err(AppError::Parse(format!(
                "Failed to extract JSON from deep analysis response: {}",
                &trimmed[..trimmed.len().min(200)]
            )));
        }
    };

    // Extract fields with defaults
    let deep = parsed
        .get("deepAnalysis")
        .or_else(|| parsed.get("deep_analysis"));

    let layer = parsed
        .get("layerImpact")
        .or_else(|| parsed.get("layer_impact"));

    let deep_analysis = if let Some(d) = deep {
        let conf = d
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.3);
        DeepMotiveAnalysis {
            primary_motive: d
                .get("primaryMotive")
                .or_else(|| d.get("primary_motive"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            secondary_motive: d
                .get("secondaryMotive")
                .or_else(|| d.get("secondary_motive"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            confidence: conf,
            confidence_grade: confidence_grade(conf).to_string(),
        }
    } else {
        DeepMotiveAnalysis {
            primary_motive: "analysis unavailable".to_string(),
            secondary_motive: String::new(),
            confidence: 0.0,
            confidence_grade: "speculative".to_string(),
        }
    };

    let layer_impact = if let Some(l) = layer {
        LayerImpact {
            physical: extract_str(l, "physical"),
            credit: extract_str(l, "credit"),
            dollar: extract_str(l, "dollar"),
            geopolitical: extract_str(l, "geopolitical"),
            sentiment: extract_str(l, "sentiment"),
        }
    } else {
        LayerImpact {
            physical: "none".to_string(),
            credit: "none".to_string(),
            dollar: "none".to_string(),
            geopolitical: "none".to_string(),
            sentiment: "none".to_string(),
        }
    };

    Ok(DeepAnalysis {
        cluster_id: cluster.cluster_id.clone(),
        cluster_topic: cluster.topic_hint.clone(),
        news_count: cluster.news_count,
        surface: parsed
            .get("surface")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        connection: parsed
            .get("connection")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        deep_analysis,
        layer_impact,
        key_observation: parsed
            .get("keyObservation")
            .or_else(|| parsed.get("key_observation"))
            .and_then(|v| v.as_str())
            .unwrap_or("no key observation")
            .to_string(),
        source_urls: Vec::new(), // filled by caller
        analyzed_at: Utc::now().to_rfc3339(),
    })
}

fn extract_str(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("none")
        .to_string()
}

/// Default analysis when Claude is unavailable.
fn default_analysis(cluster: &NewsCluster) -> DeepAnalysis {
    DeepAnalysis {
        cluster_id: cluster.cluster_id.clone(),
        cluster_topic: cluster.topic_hint.clone(),
        news_count: cluster.news_count,
        surface: "Analysis unavailable — Claude API key not configured".to_string(),
        connection: String::new(),
        deep_analysis: DeepMotiveAnalysis {
            primary_motive: String::new(),
            secondary_motive: String::new(),
            confidence: 0.0,
            confidence_grade: "speculative".to_string(),
        },
        layer_impact: LayerImpact {
            physical: "none".to_string(),
            credit: "none".to_string(),
            dollar: "none".to_string(),
            geopolitical: "none".to_string(),
            sentiment: "none".to_string(),
        },
        key_observation: "Deep analysis requires Claude API key".to_string(),
        source_urls: Vec::new(),
        analyzed_at: Utc::now().to_rfc3339(),
    }
}
