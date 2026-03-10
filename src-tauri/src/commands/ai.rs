use sqlx::SqlitePool;
use tauri::{Emitter, State};

use crate::models::ai::{AiBriefItem, CycleIndicators, CycleReasoning};
use crate::services::{cycle_reasoner, indicator_engine, summarizer};

/// Summarize all pending (unsummarized) news articles using AI.
///
/// Strategy: Ollama first -> Groq fallback.
/// Reads groq_api_key from tauri-plugin-store.
///
/// Frontend: invoke('summarize_pending_news')
///
/// # Returns
/// Count of successfully summarized articles.
#[tauri::command]
pub async fn summarize_pending_news(
    pool: State<'_, SqlitePool>,
    app: tauri::AppHandle,
) -> Result<usize, String> {
    let groq_key = read_store_key(&app, "groq_api_key");

    summarizer::summarize_pending_batch(pool.inner(), &groq_key)
        .await
        .map_err(|e| e.to_string())
}

/// Get AI brief: recent analysis grouped by category.
///
/// Queries the last 24 hours of ai_analysis results, aggregated by news category.
/// Returns category, count, average sentiment, top keywords, and latest summary.
///
/// Frontend: invoke('get_ai_brief')
#[tauri::command]
pub async fn get_ai_brief(pool: State<'_, SqlitePool>) -> Result<Vec<AiBriefItem>, String> {
    // Join ai_analysis with news to get category, then aggregate
    let rows: Vec<(String, i64, f64, String)> = sqlx::query_as(
        r#"SELECT
             COALESCE(n.category, 'market') as category,
             COUNT(*) as count,
             AVG(a.confidence) as avg_sentiment,
             a.output as latest_summary
           FROM ai_analysis a
           LEFT JOIN news n ON a.input_ids = n.id
           WHERE a.created_at >= datetime('now', '-24 hours')
             AND a.analysis_type = 'news_summary'
           GROUP BY COALESCE(n.category, 'market')
           ORDER BY count DESC"#,
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| format!("[DB_ERR] Failed to fetch AI brief: {}", e))?;

    let items = rows
        .into_iter()
        .map(|(category, count, avg_sentiment, latest_summary)| {
            // Extract keywords from reasoning_chain field (stored as JSON array)
            AiBriefItem {
                category,
                count,
                avg_sentiment,
                top_keywords: Vec::new(), // Populated below via separate query if needed
                latest_summary,
            }
        })
        .collect::<Vec<_>>();

    // Populate top_keywords per category from reasoning_chain (stored as JSON array of keywords)
    let mut result = Vec::with_capacity(items.len());
    for mut item in items {
        let keywords_rows: Vec<(String,)> = sqlx::query_as(
            r#"SELECT a.reasoning_chain
               FROM ai_analysis a
               LEFT JOIN news n ON a.input_ids = n.id
               WHERE COALESCE(n.category, 'market') = ?1
                 AND a.created_at >= datetime('now', '-24 hours')
                 AND a.analysis_type = 'news_summary'
                 AND a.reasoning_chain IS NOT NULL
               ORDER BY a.created_at DESC
               LIMIT 5"#,
        )
        .bind(&item.category)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| format!("[DB_ERR] Failed to fetch keywords: {}", e))?;

        let mut all_keywords: Vec<String> = Vec::new();
        for (kw_json,) in &keywords_rows {
            if let Ok(kws) = serde_json::from_str::<Vec<String>>(kw_json) {
                all_keywords.extend(kws);
            }
        }
        // Deduplicate and take top 5
        all_keywords.sort();
        all_keywords.dedup();
        all_keywords.truncate(5);
        item.top_keywords = all_keywords;

        result.push(item);
    }

    Ok(result)
}

/// Get current cycle indicators (Layer 2) computed from SQLite data.
///
/// Calculates 6 cycle indicators: monetary, credit, economic, market,
/// sentiment, geopolitical. No external API call needed.
///
/// Frontend: invoke('get_cycle_indicators')
#[tauri::command]
pub async fn get_cycle_indicators(
    pool: State<'_, SqlitePool>,
) -> Result<CycleIndicators, String> {
    indicator_engine::calculate_cycle_indicators(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

/// Get the latest cycle reasoning (Layer 3) from ai_analysis table.
///
/// Returns the most recent CycleReasoning record, or null if none exists.
///
/// Frontend: invoke('get_cycle_reasoning')
#[tauri::command]
pub async fn get_cycle_reasoning(
    pool: State<'_, SqlitePool>,
) -> Result<Option<CycleReasoning>, String> {
    cycle_reasoner::get_latest_reasoning(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

/// Trigger a full cycle reasoning pass: compute indicators -> Claude reasoning -> persist.
///
/// Reads claude_api_key from tauri-plugin-store. If key is missing, returns
/// a default reasoning with confidence=0.
///
/// Frontend: invoke('trigger_cycle_reasoning')
#[tauri::command]
pub async fn trigger_cycle_reasoning(
    pool: State<'_, SqlitePool>,
    app: tauri::AppHandle,
) -> Result<CycleReasoning, String> {
    let claude_key = read_store_key(&app, "claude_api_key");

    // Layer 2: compute indicators
    let indicators = indicator_engine::calculate_cycle_indicators(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Layer 3: Claude reasoning
    let reasoning = cycle_reasoner::reason_cycle(&indicators, &claude_key)
        .await
        .map_err(|e| e.to_string())?;

    // Layer 4: persist
    cycle_reasoner::persist_reasoning(pool.inner(), &reasoning)
        .await
        .map_err(|e| e.to_string())?;

    // Notify frontend
    let _ = app.emit("cycle-reasoning-updated", &reasoning);

    Ok(reasoning)
}

/// Read a key from tauri-plugin-store settings.json.
/// Returns empty string if key not found or store unavailable.
fn read_store_key(app: &tauri::AppHandle, key: &str) -> String {
    use tauri_plugin_store::StoreExt;

    let store = match app.store("settings.json") {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Failed to open settings store for {}: {}", key, e);
            return String::new();
        }
    };

    store
        .get(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}
