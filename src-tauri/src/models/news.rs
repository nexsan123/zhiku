use serde::{Deserialize, Serialize};

/// Full database row for the `news` table.
/// Contains all columns including AI-filled fields (nullable).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NewsRow {
    pub id: String,
    pub url: String,
    pub title: String,
    pub source: String,
    pub source_tier: Option<i64>,
    pub category: Option<String>,
    pub published_at: String,
    pub fetched_at: String,
    pub content_snippet: Option<String>,
    pub language: Option<String>,
    pub sentiment_score: Option<f64>,
    pub ai_summary: Option<String>,
    pub source_url: String,
}

/// Contract-aligned NewsItem for Tauri commands.
/// Aligns with: contracts/api-news.ts :: NewsItem (8 fields)
///
/// Field mapping:
///   id: string          -> String
///   title: string       -> String
///   summary: string     -> String       (mapped from content_snippet or ai_summary)
///   sourceUrl: string   -> String
///   category: NewsCategory -> String
///   country: string     -> String       (derived from source, default "us")
///   publishedAt: string -> String
///   fetchedAt: string   -> String
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub source_url: String,
    pub category: String,
    pub country: String,
    pub published_at: String,
    pub fetched_at: String,
    pub sentiment_score: Option<f64>,
    pub ai_summary: Option<String>,
}

impl From<NewsRow> for NewsItem {
    fn from(row: NewsRow) -> Self {
        let ai_sum = row.ai_summary.clone();
        NewsItem {
            id: row.id,
            title: row.title,
            summary: row
                .ai_summary
                .or(row.content_snippet)
                .unwrap_or_default(),
            source_url: row.source_url,
            category: row.category.unwrap_or_else(|| "market".to_string()),
            country: "us".to_string(), // Phase 2: US-only (ZK-04 vertical slice)
            published_at: row.published_at,
            fetched_at: row.fetched_at,
            sentiment_score: row.sentiment_score,
            ai_summary: ai_sum,
        }
    }
}

/// API status for data sources and AI engines.
/// Aligns with SQL table `api_status`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ApiStatus {
    pub service: String,
    pub status: String,
    pub last_check: Option<String>,
    pub last_error: Option<String>,
    pub response_ms: Option<i64>,
}

/// Per-country news heatmap aggregation for the map pulse layer.
/// Used by: get_news_heatmap command
/// Fields: 5 (country_code, news_count, avg_sentiment, top_keywords, latest_title)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsHeatmapEntry {
    pub country_code: String,
    pub news_count: u32,
    pub avg_sentiment: f64,
    pub top_keywords: Vec<String>,
    pub latest_title: String,
}
