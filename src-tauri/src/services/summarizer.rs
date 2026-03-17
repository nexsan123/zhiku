use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::services::ai_config::ResolvedAiConfig;
use crate::services::ai_router;
use crate::services::knowledge_base;

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
    #[serde(default)]
    pub political_bias: String,
}

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
- category: 必须严格为以下8个值之一，不可变体：macro_policy | market | geopolitical | central_bank | trade | crypto | energy | supply_chain
  判断标准：
  - geopolitical: 国际关系、制裁、军事、领土争端、外交、战争、联盟
  - macro_policy: 货币政策、财政政策、利率、通胀、GDP、就业数据
  - central_bank: 央行声明、利率决议、QE/QT、央行官员讲话
  - trade: 关税、贸易协定、进出口、WTO、供应链中断
  - energy: 石油、天然气、OPEC、可再生能源、能源价格
  - crypto: 加密货币、区块链、DeFi、稳定币、监管
  - supply_chain: 航运、芯片短缺、物流中断、制造业转移
  - market: 仅用于纯市场行情（股价、指数、IPO、财报）——不要把其他类别的新闻都归为market
- region: 1-3 个区域: east_asia, southeast_asia, south_asia, middle_east, europe, north_america, latin_america, africa, oceania, global
- entities: 1-5 个关键实体（国家、机构、公司、人物）
- politicalBias: 如果原文有明显政治倾向，标注 "pro_west"/"pro_east"/"nationalist"/"other"，否则 "none"
- category 必须是上述 8 个精确拼写之一，不可使用变体(如 geo_political, macros, crypt 等)

只输出 JSON，不输出任何其他内容:"#;

/// Summarize a single news article using AI.
///
/// Routes through `ai_router::reason()` using the resolved batch config
/// (provider priority: groq > ollama > others via ai_config::resolve_batch_config).
///
/// # Arguments
/// * `title` - News article title
/// * `description` - News article content/snippet
/// * `config` - Resolved AI config (api_key, model_name, endpoint_url)
/// * `provider` - Provider name for routing (e.g. "groq", "ollama")
///
/// # Returns
/// Parsed `NewsSummary` with summary, sentiment, keywords, category, plus the model label.
pub async fn summarize_news(
    title: &str,
    description: &str,
    config: &ResolvedAiConfig,
    provider: &str,
) -> Result<(NewsSummary, String), AppError> {
    let prompt = SUMMARIZE_PROMPT_TEMPLATE
        .replace("{title}", title)
        .replace("{description}", description);

    // Inject media bias registry as system prompt for source reliability assessment
    let system_prompt = format!(
        "你是一位独立的金融新闻分析师。你不带任何政治立场，只提取事实和金融影响。\n\n\
         以下是新闻源偏见参考数据，用于评估来源可信度和检测政治偏见：\n{}",
        knowledge_base::media_bias_slim(),
    );

    let response = ai_router::reason(&prompt, Some(&system_prompt), config, provider).await?;

    if response.is_empty() {
        return Err(AppError::Network(
            "AI engine returned empty response (API key may not be configured)".to_string(),
        ));
    }

    match parse_ai_json(&response) {
        Ok(summary) => {
            let model = config.model_label(provider);
            Ok((summary, model))
        }
        Err(e) => Err(AppError::Parse(format!(
            "AI returned unparseable response: {}",
            e
        ))),
    }
}

/// Process all pending (unsummarized) news articles in batch.
///
/// Queries `news` table for rows where `ai_summary IS NULL`, runs summarization
/// on each via `ai_router`, writes results to `ai_analysis` table, and updates
/// `news.ai_summary` + `news.sentiment_score`.
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `config` - Resolved AI config from `ai_config::resolve_batch_config()`
/// * `provider` - Provider name for routing (e.g. "groq", "ollama")
///
/// # Returns
/// Count of successfully summarized articles.
pub async fn summarize_pending_batch(
    pool: &SqlitePool,
    config: &ResolvedAiConfig,
    provider: &str,
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
    let mut fail_count: usize = 0;
    let mut retry_count: usize = 0;

    for (idx, (news_id, title, snippet, source_url)) in pending.iter().enumerate() {
        // Throttle: 3s delay between requests to avoid Groq TPM limit
        if idx > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }

        let description = snippet.as_deref().unwrap_or("");

        let result = match summarize_news(title, description, config, provider).await {
            Ok(ok) => Ok(ok),
            Err(e) => {
                let err_msg = e.to_string().to_lowercase();
                if err_msg.contains("429") || err_msg.contains("rate limit") {
                    retry_count += 1;
                    log::warn!(
                        "Summarizer: rate-limited on news {}, waiting 30s before retry",
                        news_id
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    // Single retry
                    summarize_news(title, description, config, provider).await
                } else {
                    Err(e)
                }
            }
        };

        match result {
            Ok((mut summary, model_used)) => {
                // Normalize category before DB write (defense against LLM variant spellings)
                summary.category = normalize_category(&summary.category);

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
                    fail_count += 1;
                    continue;
                }

                // Update news table
                if let Err(e) = update_news_summary(pool, news_id, &summary).await {
                    log::warn!("Failed to update news summary for {}: {}", news_id, e);
                    fail_count += 1;
                    continue;
                }

                success_count += 1;
            }
            Err(e) => {
                log::warn!("Summarization failed for news {}: {}", news_id, e);
                fail_count += 1;
                // Continue with next article — do not abort batch
            }
        }
    }

    log::info!(
        "Summarizer: {}/{} articles processed (failed: {}, retries: {})",
        success_count,
        pending.len(),
        fail_count,
        retry_count
    );
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
        "politicalBias": summary.political_bias,
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
        &trimmed.chars().take(200).collect::<String>()
    )))
}

/// Normalize AI-generated category to one of 8 canonical values.
///
/// Handles common LLM misspellings and variants (e.g. "geo_political" -> "geopolitical").
/// Falls back to "market" for unrecognized values.
fn normalize_category(raw: &str) -> String {
    let lower = raw.to_lowercase().replace(['-', ' '], "_");
    match lower.as_str() {
        "macro_policy" | "macros" | "macro" | "policy" => "macro_policy",
        "geopolitical" | "geo_political" | "geopolitic" | "geopolitics" => "geopolitical",
        "central_bank" | "centralbank" | "central" => "central_bank",
        "crypto" | "crypt" | "cryptocurrency" | "blockchain" => "crypto",
        "energy" | "oil" | "gas" => "energy",
        "trade" | "tariff" | "tariffs" => "trade",
        "supply_chain" | "supplychain" | "logistics" | "energysupply_chain" => "supply_chain",
        "market" | "markets" | "stock" | "stocks" | "stock_market" | "finance" => "market",
        _ => "market", // fallback
    }
    .to_string()
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
