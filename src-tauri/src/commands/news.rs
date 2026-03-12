use sqlx::SqlitePool;
use tauri::State;

use crate::models::news::{NewsHeatmapEntry, NewsItem, NewsRow};
use crate::services::{news_heatmap, rss_fetcher};

/// Get all news articles from the database, ordered by published date descending.
/// Aligns with: contracts/api-news.ts :: NewsItem
/// Frontend: invoke('get_news')
#[tauri::command]
pub async fn get_news(pool: State<'_, SqlitePool>) -> Result<Vec<NewsItem>, String> {
    let rows = sqlx::query_as::<_, NewsRow>(
        "SELECT id, url, title, source, source_tier, category, published_at,
                fetched_at, content_snippet, language, sentiment_score,
                ai_summary, source_url
         FROM news ORDER BY published_at DESC LIMIT 200",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| format!("[DB_ERR] Failed to fetch news: {}", e))?;

    Ok(rows.into_iter().map(NewsItem::from).collect())
}

/// Get total count of news articles in the database.
/// Frontend: invoke('get_news_count')
#[tauri::command]
pub async fn get_news_count(pool: State<'_, SqlitePool>) -> Result<i64, String> {
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM news")
        .fetch_one(pool.inner())
        .await
        .map_err(|e| format!("[DB_ERR] Failed to count news: {}", e))?;

    Ok(count)
}

/// Trigger RSS fetch for all configured feeds. Returns count of newly inserted articles.
/// Frontend: invoke('fetch_rss')
#[tauri::command]
pub async fn fetch_rss(pool: State<'_, SqlitePool>) -> Result<usize, String> {
    rss_fetcher::fetch_all_rss(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

/// Get per-country news heatmap aggregation for the map pulse layer.
/// Aggregates news from the last N hours by country keyword matching.
/// Frontend: invoke('get_news_heatmap', { hours: 1 })
#[tauri::command]
pub async fn get_news_heatmap(
    pool: State<'_, SqlitePool>,
    hours: Option<u32>,
) -> Result<Vec<NewsHeatmapEntry>, String> {
    let h = hours.unwrap_or(1);
    news_heatmap::aggregate_news_heatmap(pool.inner(), h)
        .await
        .map_err(|e| e.to_string())
}
