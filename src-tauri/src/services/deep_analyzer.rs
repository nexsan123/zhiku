use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::credit::confidence_grade;
use crate::models::intelligence::{DeepAnalysis, DeepMotiveAnalysis, LayerImpact, NewsCluster};
use crate::services::summarizer;

/// System prompt for deep analysis (second pass, provider-agnostic).
const DEEP_ANALYSIS_SYSTEM_PROMPT: &str = r#"你是一位独立的全球金融情报深度分析师。你站在上帝视角，超越一切国家、政党、意识形态的立场。

核心原则：
- 零立场分析：不替任何国家说话。所有国家的行为（制裁、关税、补贴、货币政策）一律视为"利益博弈"客观解读
- 剥离政治叙事：新闻中"维护国家安全""自由贸易""公平竞争"等说辞，都要还原为背后的经济利益诉求
- 识别信息战：当新闻来源有明显政治倾向（官方媒体、立场鲜明的机构），标注其倾向性，提取可验证的事实部分
- 多方博弈视角：每个事件至少从两方利益角度分析，理解"谁获益、谁受损、谁在推动"
- 追溯资金流：最终一切地缘事件都通过资金流和信用传导影响金融市场

给定一组相关新闻，深度分析隐性动机和跨层影响。

只回复 JSON 对象（无 markdown，无解释），格式如下：
{
  "surface": "表面事件：发生了什么（1-2 句）",
  "connection": "关联逻辑：这些新闻为什么是一组事件（1-2 句）",
  "deepAnalysis": {
    "primaryMotive": "主要动机：背后的核心利益驱动是什么",
    "secondaryMotive": "隐性动机：不容易看到的第二层博弈（如无则空字符串）",
    "biasWarning": "如果输入新闻含有明显政治倾向，在此标注。如无则空字符串",
    "confidence": 0.0 到 1.0,
    "confidenceGrade": "high|reasonable|speculative"
  },
  "layerImpact": {
    "physical": "对物理层的影响（能源、粮食、供应链）或 'none'",
    "credit": "对信用层的影响（借贷、债务、银行体系）或 'none'",
    "dollar": "对美元/资本流动的影响（DXY、新兴市场资金流）或 'none'",
    "geopolitical": "对地缘格局的影响（联盟、制裁、冲突）或 'none'",
    "sentiment": "对市场情绪的影响（恐惧/贪婪、叙事变化）或 'none'"
  },
  "keyObservation": "最关键的一句话：对金融市场最重要的判断"
}

规则：
- 用中文回复所有文本字段
- confidence: ≥ 0.8 多源验证、0.5-0.79 合理推断、< 0.5 推测性判断
- confidenceGrade 必须与 confidence 数值匹配
- 超越表面叙事，分析经济战、制裁规避、资源争夺、货币霸权等深层博弈
- 不要用"某国是好的/坏的"这种判断，只分析"这个行为的经济影响是什么"
- 只输出 JSON，不输出任何其他内容"#;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Perform deep analysis on a news cluster using configured AI provider.
///
/// Fetches news details from SQLite, builds a prompt with all titles + summaries,
/// sends to AI provider (DeepSeek preferred, Claude fallback), parses the structured response.
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
        log::warn!("AI returned empty response for deep analysis");
        return Ok(default_analysis(cluster));
    }

    let mut analysis = parse_deep_analysis(&response, cluster)?;

    // Attach source URLs for traceability (ZK-01)
    analysis.source_urls = fetch_source_urls(pool, &cluster.news_ids).await;

    Ok(analysis)
}

/// Persist a DeepAnalysis result to the ai_analysis table.
///
/// `model_label` should be e.g. "deepseek:deepseek-chat" or "claude:claude-sonnet-4-20250514",
/// produced by `ResolvedAiConfig::model_label(provider)`.
pub async fn persist_analysis(
    pool: &SqlitePool,
    analysis: &DeepAnalysis,
    model_label: &str,
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
           VALUES (?1, 'deep_analysis', ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
    )
    .bind(&id)
    .bind(&analysis.cluster_id)
    .bind(&output_json)
    .bind(model_label)
    .bind(analysis.deep_analysis.confidence)
    .bind(&analysis.key_observation)
    .bind(&source_urls_json)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert deep_analysis failed: {}", e)))?;

    log::info!(
        "Deep analysis persisted: cluster={}, model={}, confidence={:.2}",
        analysis.cluster_id,
        model_label,
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

/// Build the user prompt for deep analysis.
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

/// Parse AI response into DeepAnalysis.
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
            bias_warning: d
                .get("biasWarning")
                .or_else(|| d.get("bias_warning"))
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
            bias_warning: String::new(),
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

/// Default analysis when no AI provider is available.
fn default_analysis(cluster: &NewsCluster) -> DeepAnalysis {
    DeepAnalysis {
        cluster_id: cluster.cluster_id.clone(),
        cluster_topic: cluster.topic_hint.clone(),
        news_count: cluster.news_count,
        surface: "Analysis unavailable — no AI provider configured".to_string(),
        connection: String::new(),
        deep_analysis: DeepMotiveAnalysis {
            primary_motive: String::new(),
            secondary_motive: String::new(),
            bias_warning: String::new(),
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
        key_observation: "Deep analysis requires an AI provider (DeepSeek/Claude)".to_string(),
        source_urls: Vec::new(),
        analyzed_at: Utc::now().to_rfc3339(),
    }
}
