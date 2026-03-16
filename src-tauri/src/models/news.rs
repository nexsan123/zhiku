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

/// API status enriched with runtime-computed freshness.
/// Extends ApiStatus with freshness label and minutes_ago.
/// Fields: 7 (service, status, last_check, last_error, response_ms, freshness, minutes_ago)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStatusResponse {
    pub service: String,
    pub status: String,
    pub last_check: Option<String>,
    pub last_error: Option<String>,
    pub response_ms: Option<i64>,
    pub freshness: String,
    pub minutes_ago: Option<i64>,
}

/// Compute freshness label and minutes_ago from an optional RFC3339 timestamp.
/// Returns ("unknown", None) if last_check is None or unparseable.
fn compute_freshness(last_check: &Option<String>) -> (String, Option<i64>) {
    let ts = match last_check {
        Some(s) => s,
        None => return ("unknown".to_string(), None),
    };
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        let minutes = (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_minutes();
        let label = if minutes < 5 {
            "live"
        } else if minutes < 30 {
            "recent"
        } else if minutes < 120 {
            "aging"
        } else if minutes < 1440 {
            "stale"
        } else {
            "expired"
        };
        (label.to_string(), Some(minutes))
    } else {
        ("unknown".to_string(), None)
    }
}

impl From<ApiStatus> for ApiStatusResponse {
    fn from(row: ApiStatus) -> Self {
        let (freshness, minutes_ago) = compute_freshness(&row.last_check);
        ApiStatusResponse {
            service: row.service,
            status: row.status,
            last_check: row.last_check,
            last_error: row.last_error,
            response_ms: row.response_ms,
            freshness,
            minutes_ago,
        }
    }
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
