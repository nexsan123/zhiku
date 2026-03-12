use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::services::{groq_client, ollama_client};

/// Internal result of AI news summarization.
#[derive(Debug, Deserialize)]
pub struct NewsSummary {
    pub summary: String,
    pub sentiment: f64,
    pub keywords: Vec<String>,
    pub category: String,
    /// Regions involved (e.g. "east_asia", "middle_east", "north_america", "europe", "global").
    #[serde(default)]
    pub region: Vec<String>,
    /// Key entities mentioned (countries, orgs, companies, people).
    #[serde(default)]
    pub entities: Vec<String>,
    /// Political bias detected in source: "none", "pro_west", "pro_east", "nationalist", "other".
    #[serde(default = "default_bias")]
    pub political_bias: String,
}

fn default_bias() -> String {
    "none".to_string()
}

/// Default Ollama model for batch summarization.
const OLLAMA_MODEL: &str = "llama3.1:8b";

/// Prompt template for news summarization.
/// Requests structured JSON output with summary, sentiment, keywords, category.
const SUMMARIZE_PROMPT_TEMPLATE: &str = r#"你是一位独立的金融新闻分析师。你不带任何政治立场，只提取事实和金融影响。

分析以下新闻，只回复 JSON 对象（无 markdown，无解释）：

标题: {title}
内容: {description}

格式：
{"summary":"2-3句中文摘要，提取核心事实","sentiment":0.0,"keywords":["关键词1","关键词2"],"category":"market","region":["east_asia"],"entities":["中国","央行"],"politicalBias":"none"}

规则：
- summary: 2-3 句中文，只陈述事实和金融影响，剥离政治叙事和情绪化用词
- sentiment: -1.0（极度负面）到 1.0（极度正面），基于对金融市场的实际影响而非政治立场
- keywords: 3-5 个金融相关关键词（中文）
- category: macro_policy, market, geopolitical, central_bank, trade, crypto, energy, supply_chain 之一
- region: 1-3 个区域: east_asia, southeast_asia, south_asia, middle_east, europe, north_america, latin_america, africa, oceania, global
- entities: 1-5 个关键实体（国家、机构、公司、人物）
- politicalBias: 如果原文有明显政治倾向，标注 "pro_west"/"pro_east"/"nationalist"/"other"，否则 "none"

只输出 JSON，不输出任何其他内容:"#;

/// Summarize a single news article using AI.
///
/// Strategy: Ollama first -> Groq fallback -> error if both fail.
///
/// # Arguments
/// * `title` - News article title
/// * `description` - News article content/snippet
/// * `groq_api_key` - Groq API key for fallback (empty = skip Groq)
///
/// # Returns
/// Parsed `NewsSummary` with summary, sentiment, keywords, category.
pub async fn summarize_news(
    title: &str,
    description: &str,
    groq_api_key: &str,
) -> Result<(NewsSummary, String), AppError> {
    let prompt = SUMMARIZE_PROMPT_TEMPLATE
        .replace("{title}", title)
        .replace("{description}", description);

    // Try Groq first (primary), Ollama as fallback (when available)
    if !groq_api_key.is_empty() {
        let groq_response = groq_client::chat_completion(&prompt, groq_api_key).await;
        match groq_response {
            Ok(ref text) if !text.is_empty() => {
                if let Ok(summary) = parse_ai_json(text) {
                    let model = "groq:llama-3.1-8b-instant".to_string();
                    return Ok((summary, model));
                }
                log::warn!("Groq returned unparseable response, trying Ollama fallback");
            }
            Ok(_) => {
                log::warn!("Groq returned empty response, trying Ollama fallback");
            }
            Err(e) => {
                log::warn!("Groq failed: {}, trying Ollama fallback", e);
            }
        }
    }

    // Fallback to Ollama
    match ollama_client::generate_completion(&prompt, OLLAMA_MODEL).await {
        Ok(response) => {
            if let Ok(summary) = parse_ai_json(&response) {
                let model = format!("ollama:{}", OLLAMA_MODEL);
                return Ok((summary, model));
            }
            Err(AppError::Network(
                "All AI engines returned unparseable responses".to_string(),
            ))
        }
        Err(e) => Err(AppError::Network(
            format!("All AI engines unavailable: {}", e),
        )),
    }
}

/// Process all pending (unsummarized) news articles in batch.
///
/// Queries `news` table for rows where `ai_summary IS NULL`, runs summarization
/// on each, writes results to `ai_analysis` table, and updates `news.ai_summary`
/// + `news.sentiment_score`.
///
/// # Returns
/// Count of successfully summarized articles.
pub async fn summarize_pending_batch(
    pool: &SqlitePool,
    groq_api_key: &str,
) -> Result<usize, AppError> {
    // Fetch pending news (limit to 20 per batch to avoid long blocking)
    let pending: Vec<(String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT id, title, content_snippet, source_url FROM news
         WHERE ai_summary IS NULL
         ORDER BY published_at DESC LIMIT 20",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query pending news failed: {}", e)))?;

    if pending.is_empty() {
        return Ok(0);
    }

    log::info!("Summarizer: {} pending articles to process", pending.len());
    let mut success_count: usize = 0;

    for (news_id, title, snippet, source_url) in &pending {
        let description = snippet.as_deref().unwrap_or("");

        match summarize_news(title, description, groq_api_key).await {
            Ok((summary, model_used)) => {
                // Write to ai_analysis table
                if let Err(e) = write_analysis(
                    pool,
                    news_id,
                    &summary,
                    &model_used,
                    source_url,
                )
                .await
                {
                    log::warn!("Failed to write ai_analysis for {}: {}", news_id, e);
                    continue;
                }

                // Update news table
                if let Err(e) = update_news_summary(pool, news_id, &summary).await {
                    log::warn!("Failed to update news summary for {}: {}", news_id, e);
                    continue;
                }

                success_count += 1;
            }
            Err(e) => {
                log::warn!("Summarization failed for news {}: {}", news_id, e);
                // Continue with next article — do not abort batch
            }
        }
    }

    log::info!("Summarizer: {}/{} articles processed", success_count, pending.len());
    Ok(success_count)
}

/// Write analysis result to `ai_analysis` table.
///
/// Maps to existing schema: input_ids=newsId, output=summary,
/// model=engine name, source_urls=sourceUrl.
async fn write_analysis(
    pool: &SqlitePool,
    news_id: &str,
    summary: &NewsSummary,
    model_used: &str,
    source_url: &str,
) -> Result<(), AppError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    // Store keywords + region + entities as structured JSON in reasoning_chain
    let meta_json = serde_json::json!({
        "keywords": summary.keywords,
        "region": summary.region,
        "entities": summary.entities,
    })
    .to_string();

    sqlx::query(
        r#"INSERT INTO ai_analysis
           (id, analysis_type, input_ids, output, model, confidence, reasoning_chain, source_urls, created_at)
           VALUES (?1, 'news_summary', ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
    )
    .bind(&id)
    .bind(news_id)
    .bind(&summary.summary)
    .bind(model_used)
    .bind(summary.sentiment)
    .bind(&meta_json)
    .bind(source_url)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert ai_analysis failed: {}", e)))?;

    Ok(())
}

/// Update the `news` table with AI summary and sentiment score.
async fn update_news_summary(
    pool: &SqlitePool,
    news_id: &str,
    summary: &NewsSummary,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE news SET ai_summary = ?1, sentiment_score = ?2, category = ?3 WHERE id = ?4",
    )
    .bind(&summary.summary)
    .bind(summary.sentiment)
    .bind(&summary.category)
    .bind(news_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Update news summary failed: {}", e)))?;

    Ok(())
}

/// Parse AI response text into a `NewsSummary`.
///
/// Handles multiple response formats:
/// - Raw JSON: `{"summary": "...", ...}`
/// - Markdown-wrapped: ````json\n{...}\n````
/// - With surrounding text: extracts the first JSON object
fn parse_ai_json(response: &str) -> Result<NewsSummary, AppError> {
    let trimmed = response.trim();

    // Try direct parse first
    if let Ok(summary) = serde_json::from_str::<NewsSummary>(trimmed) {
        return Ok(summary);
    }

    // Try stripping markdown code block markers
    let stripped = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(summary) = serde_json::from_str::<NewsSummary>(stripped) {
        return Ok(summary);
    }

    // Try extracting first JSON object by finding matching braces
    if let Some(json_str) = extract_json_object(trimmed) {
        if let Ok(summary) = serde_json::from_str::<NewsSummary>(&json_str) {
            return Ok(summary);
        }
    }

    Err(AppError::Parse(format!(
        "Failed to parse AI response as NewsSummary: {}",
        &trimmed[..trimmed.len().min(200)]
    )))
}

/// Extract the first top-level JSON object from a string.
/// Finds the first `{` and its matching `}` by counting brace depth.
pub fn extract_json_object(input: &str) -> Option<String> {
    let start = input.find('{')?;
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in input[start..].char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(input[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
    }

    None
}
