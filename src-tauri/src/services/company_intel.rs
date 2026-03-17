use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::errors::AppError;
use crate::services::{ai_config, ai_router};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyIntel {
    pub query: String,
    pub matched_news_count: usize,
    pub news_items: Vec<CompanyNewsItem>,
    pub ai_summary: Option<CompanyAnalysis>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyNewsItem {
    pub title: String,
    pub source: String,
    pub published_at: String,
    pub sentiment: Option<f64>,
    pub ai_summary: Option<String>,
    pub category: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyAnalysis {
    pub summary: String,
    pub sentiment_overall: f64,
    pub sentiment_label: String,
    pub key_themes: Vec<String>,
    pub risk_factors: Vec<String>,
    pub opportunity_signals: Vec<String>,
    pub recommendation: String,
    pub confidence: f64,
    pub model: String,
}

/// Search, aggregate, and analyze news for a specific company.
///
/// Flow:
/// 1. Search news table for titles/summaries containing the query (last 30 days)
/// 2. Aggregate matched news
/// 3. If AI API key available, call DeepSeek/Groq for comprehensive analysis
/// 4. Return structured result
pub async fn analyze_company(
    pool: &SqlitePool,
    query: &str,
    app_handle: &tauri::AppHandle,
) -> Result<CompanyIntel, AppError> {
    // 1. Search related news (title or ai_summary contains query, last 30 days, max 50)
    let search_pattern = format!("%{}%", query);
    let news_rows: Vec<(
        String,
        String,
        String,
        Option<f64>,
        Option<String>,
        Option<String>,
        String,
    )> = sqlx::query_as(
        r#"SELECT title, source, published_at, sentiment_score, ai_summary, category, source_url
           FROM news
           WHERE (title LIKE ?1 OR ai_summary LIKE ?1)
             AND published_at >= datetime('now', '-30 days')
           ORDER BY published_at DESC
           LIMIT 50"#,
    )
    .bind(&search_pattern)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Search company news: {}", e)))?;

    let matched_count = news_rows.len();

    // Take top 10 for response (all 50 used for AI analysis)
    let news_items: Vec<CompanyNewsItem> = news_rows
        .iter()
        .take(10)
        .map(
            |(title, source, pub_at, sentiment, summary, category, url)| CompanyNewsItem {
                title: title.clone(),
                source: source.clone(),
                published_at: pub_at.clone(),
                sentiment: *sentiment,
                ai_summary: summary.clone(),
                category: category.clone(),
                url: url.clone(),
            },
        )
        .collect();

    // 2. Skip AI analysis if too few news items
    let ai_summary = if matched_count >= 2 {
        build_ai_analysis(&news_rows, query, matched_count, app_handle).await
    } else {
        None
    };

    Ok(CompanyIntel {
        query: query.to_string(),
        matched_news_count: matched_count,
        news_items,
        ai_summary,
        generated_at: Utc::now().to_rfc3339(),
    })
}

/// Build AI analysis from news rows. Returns None if no AI provider available or on failure.
async fn build_ai_analysis(
    news_rows: &[(
        String,
        String,
        String,
        Option<f64>,
        Option<String>,
        Option<String>,
        String,
    )],
    query: &str,
    matched_count: usize,
    app_handle: &tauri::AppHandle,
) -> Option<CompanyAnalysis> {
    let analysis_count = matched_count.min(20);
    let mut prompt = format!(
        "Analyze the following {} news items related to \"{}\" and provide an investment perspective assessment.\n\n",
        analysis_count, query
    );

    for (i, (title, source, pub_at, sentiment, summary, _, _)) in
        news_rows.iter().take(20).enumerate()
    {
        let date_slice = &pub_at[..pub_at.len().min(10)];
        let summary_text = summary.as_deref().unwrap_or("N/A");
        let sentiment_text = sentiment
            .map(|s| format!("{:.2}", s))
            .unwrap_or_else(|| "N/A".to_string());

        prompt.push_str(&format!(
            "[{}] \"{}\" ({}, {})\nSummary: {}\nSentiment: {}\n\n",
            i + 1,
            title,
            source,
            date_slice,
            summary_text,
            sentiment_text,
        ));
    }

    prompt.push_str(&format!(
        r#"Reply with a JSON object only (no markdown, no explanation):
{{
  "summary": "2-3 sentence summary analyzing {} recent dynamics from an investment perspective (in Chinese)",
  "sentimentOverall": float from -1.0 to 1.0,
  "sentimentLabel": "positive|neutral|negative",
  "keyThemes": ["theme1", "theme2"],
  "riskFactors": ["risk1"],
  "opportunitySignals": ["signal1"],
  "recommendation": "bullish|bearish|neutral",
  "confidence": float from 0.0 to 1.0
}}"#,
        query
    ));

    let system = "You are an independent equity research analyst. Zero bias, analyze based on news facts only. Distinguish facts from opinions. Note information reliability.";

    // Use batch config (groq priority, fast)
    let (config, provider) = ai_config::resolve_batch_config(app_handle);
    if config.api_key.is_empty() && provider != "ollama" {
        return None;
    }

    match ai_router::reason(&prompt, Some(system), &config, &provider).await {
        Ok(response) if !response.is_empty() => {
            parse_company_analysis(&response, &config.model_label(&provider))
        }
        _ => None,
    }
}

/// Parse AI response into CompanyAnalysis, with multi-layer fallback parsing.
fn parse_company_analysis(response: &str, model: &str) -> Option<CompanyAnalysis> {
    let trimmed = response.trim();

    // Try direct parse
    let parsed: serde_json::Value = if let Ok(v) = serde_json::from_str(trimmed) {
        v
    } else {
        // Strip markdown fences
        let stripped = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        if let Ok(v) = serde_json::from_str(stripped) {
            v
        } else {
            // Try to extract JSON object by braces
            let start = trimmed.find('{')?;
            let end = trimmed.rfind('}')?;
            serde_json::from_str(&trimmed[start..=end]).ok()?
        }
    };

    let sentiment = parsed
        .get("sentimentOverall")
        .or_else(|| parsed.get("sentiment_overall"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let label = parsed
        .get("sentimentLabel")
        .or_else(|| parsed.get("sentiment_label"))
        .and_then(|v| v.as_str())
        .unwrap_or(if sentiment > 0.2 {
            "positive"
        } else if sentiment < -0.2 {
            "negative"
        } else {
            "neutral"
        });

    Some(CompanyAnalysis {
        summary: parsed
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        sentiment_overall: sentiment,
        sentiment_label: label.to_string(),
        key_themes: extract_string_array(&parsed, "keyThemes")
            .or_else(|| extract_string_array(&parsed, "key_themes"))
            .unwrap_or_default(),
        risk_factors: extract_string_array(&parsed, "riskFactors")
            .or_else(|| extract_string_array(&parsed, "risk_factors"))
            .unwrap_or_default(),
        opportunity_signals: extract_string_array(&parsed, "opportunitySignals")
            .or_else(|| extract_string_array(&parsed, "opportunity_signals"))
            .unwrap_or_default(),
        recommendation: parsed
            .get("recommendation")
            .and_then(|v| v.as_str())
            .unwrap_or("neutral")
            .to_string(),
        confidence: parsed
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.3),
        model: model.to_string(),
    })
}

fn extract_string_array(v: &serde_json::Value, key: &str) -> Option<Vec<String>> {
    v.get(key).and_then(|a| a.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|s| s.as_str().map(|s| s.to_string()))
            .collect()
    })
}
