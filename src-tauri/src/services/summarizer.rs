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
}

/// Default Ollama model for batch summarization.
const OLLAMA_MODEL: &str = "llama3.1:8b";

/// Prompt template for news summarization.
/// Requests structured JSON output with summary, sentiment, keywords, category.
const SUMMARIZE_PROMPT_TEMPLATE: &str = r#"You are a financial news analyst. Analyze the following news article and respond with ONLY a JSON object (no markdown, no explanation):

Title: {title}
Content: {description}

Respond with exactly this JSON structure:
{"summary":"2-3 sentence summary","sentiment":0.0,"keywords":["keyword1","keyword2","keyword3"],"category":"market"}

Rules:
- summary: 2-3 concise sentences capturing the key point
- sentiment: a float from -1.0 (extremely negative) to 1.0 (extremely positive)
- keywords: 3-5 relevant financial keywords
- category: one of: macro_policy, market, geopolitical, central_bank, trade, crypto

JSON only, no other text:"#;

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

    // Try Ollama first
    match ollama_client::generate_completion(&prompt, OLLAMA_MODEL).await {
        Ok(response) => {
            if let Ok(summary) = parse_ai_json(&response) {
                let model = format!("ollama:{}", OLLAMA_MODEL);
                return Ok((summary, model));
            }
            log::warn!("Ollama returned unparseable response, trying Groq fallback");
        }
        Err(e) => {
            log::warn!("Ollama failed: {}, trying Groq fallback", e);
        }
    }

    // Fallback to Groq
    let groq_response = groq_client::chat_completion(&prompt, groq_api_key).await?;
    if groq_response.is_empty() {
        return Err(AppError::Network(
            "All AI engines unavailable for summarization".to_string(),
        ));
    }

    match parse_ai_json(&groq_response) {
        Ok(summary) => {
            let model = "groq:llama-3.1-8b-instant".to_string();
            Ok((summary, model))
        }
        Err(e) => Err(e),
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
    let keywords_json =
        serde_json::to_string(&summary.keywords).unwrap_or_else(|_| "[]".to_string());

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
    .bind(&keywords_json)
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
