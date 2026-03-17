use sqlx::SqlitePool;
use tauri::{Emitter, State};

use crate::models::ai::{AiBriefItem, CycleIndicators, CycleReasoning, FiveLayerReasoning};
use crate::services::{
    company_intel, cycle_reasoner, deep_analyzer, global_aggregator, indicator_engine,
    scenario_engine, summarizer,
};

/// Summarize all pending (unsummarized) news articles using AI.
///
/// Routes through ai_router using batch-optimized provider priority
/// (groq > ollama > others via ai_config::resolve_batch_config).
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
    let (config, provider) = crate::services::ai_config::resolve_batch_config(&app);

    summarizer::summarize_pending_batch(pool.inner(), &config, &provider)
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
            // New format: {"keywords":[...],"region":[...],"entities":[...]}
            if let Ok(meta) = serde_json::from_str::<serde_json::Value>(kw_json) {
                if let Some(kws) = meta.get("keywords").and_then(|v| v.as_array()) {
                    all_keywords.extend(
                        kws.iter().filter_map(|v| v.as_str().map(|s| s.to_string())),
                    );
                }
            }
            // Legacy format: ["keyword1","keyword2"] (backward compat)
            else if let Ok(kws) = serde_json::from_str::<Vec<String>>(kw_json) {
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
    let (ai_config, provider) = crate::services::ai_config::resolve_reasoning_config(&app);

    // Layer 2: compute indicators
    let indicators = indicator_engine::calculate_cycle_indicators(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Layer 3: AI reasoning (uses user-configured provider/model/endpoint)
    let reasoning = cycle_reasoner::reason_cycle(pool.inner(), &indicators, &ai_config, &provider)
        .await
        .map_err(|e| e.to_string())?;

    // Layer 4: persist with model label
    let model_label = ai_config.model_label(&provider);
    cycle_reasoner::persist_reasoning(pool.inner(), &reasoning, &model_label)
        .await
        .map_err(|e| e.to_string())?;

    // Notify frontend
    let _ = app.emit("cycle-reasoning-updated", &reasoning);

    Ok(reasoning)
}

/// Get the latest five-layer reasoning.
///
/// Frontend: invoke('get_five_layer_reasoning')
#[tauri::command]
pub async fn get_five_layer_reasoning(
    pool: State<'_, SqlitePool>,
) -> Result<Option<FiveLayerReasoning>, String> {
    cycle_reasoner::get_latest_five_layer(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

/// Trigger a full five-layer reasoning pass.
///
/// Gathers data from all layers (credit cycle, dollar tide, indicators,
/// deep analysis, scenarios) and sends to Claude for integrated reasoning.
///
/// Frontend: invoke('trigger_five_layer_reasoning')
#[tauri::command]
pub async fn trigger_five_layer_reasoning(
    pool: State<'_, SqlitePool>,
    app: tauri::AppHandle,
) -> Result<FiveLayerReasoning, String> {
    let (ai_config, provider) = crate::services::ai_config::resolve_reasoning_config(&app);

    // Gather all five layers
    let cycle_overview = global_aggregator::compute_global_overview(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let indicators = indicator_engine::calculate_cycle_indicators(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Deep analysis summaries (latest 5)
    let deep_analyses = deep_analyzer::get_recent_analyses(pool.inner(), 5)
        .await
        .unwrap_or_default();
    let intelligence_summaries: Vec<String> = deep_analyses
        .iter()
        .map(|a| format!("{}: {}", a.cluster_topic, a.key_observation))
        .collect();

    // Active scenarios
    let scenarios = scenario_engine::get_active_scenarios(pool.inner())
        .await
        .unwrap_or_default();
    let active_scenarios: Vec<String> = scenarios
        .scenarios
        .iter()
        .map(|s| format!("[{}] {} (p={:.0}%)", s.policy_vector, s.title, s.probability * 100.0))
        .collect();

    let input = cycle_reasoner::FiveLayerInput {
        cycle_overview,
        indicators,
        intelligence_summaries,
        active_scenarios,
    };

    let reasoning = cycle_reasoner::reason_five_layer(pool.inner(), &input, &ai_config, &provider)
        .await
        .map_err(|e| e.to_string())?;

    let model_label = ai_config.model_label(&provider);
    cycle_reasoner::persist_five_layer(pool.inner(), &reasoning, &model_label)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app.emit("five-layer-reasoning-updated", &reasoning);

    Ok(reasoning)
}

/// Get recent deep analyses (intelligence briefing).
///
/// Frontend: invoke('get_deep_analyses', { limit })
#[tauri::command]
pub async fn get_deep_analyses(
    pool: State<'_, SqlitePool>,
    limit: Option<i64>,
) -> Result<Vec<crate::models::intelligence::DeepAnalysis>, String> {
    deep_analyzer::get_recent_analyses(pool.inner(), limit.unwrap_or(10))
        .await
        .map_err(|e| e.to_string())
}

/// Get the latest daily intelligence brief.
///
/// Returns the most recent DailyBrief generated by the rule engine,
/// or null if none has been generated yet.
///
/// Frontend: invoke('get_daily_brief')
#[tauri::command]
pub async fn get_daily_brief(
    pool: State<'_, SqlitePool>,
) -> Result<Option<crate::services::daily_brief::DailyBrief>, String> {
    crate::services::daily_brief::get_latest_brief(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

/// Get recent financial alerts (last 20, most recent first).
///
/// Returns alerts triggered by threshold rules against cycle indicators,
/// persisted in the signals table.
///
/// Frontend: invoke('get_alerts')
#[tauri::command]
pub async fn get_alerts(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<crate::services::alert_engine::Alert>, String> {
    crate::services::alert_engine::get_recent_alerts(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

/// Get trend data for a specific indicator over the last N days.
///
/// Returns time-series data points from indicator_history table.
/// Default: 30 days if `days` not specified.
///
/// Frontend: invoke('get_indicator_trend', { indicator, days })
#[tauri::command]
pub async fn get_indicator_trend(
    pool: State<'_, SqlitePool>,
    indicator: String,
    days: Option<i64>,
) -> Result<Vec<crate::services::trend_tracker::TrendPoint>, String> {
    crate::services::trend_tracker::get_trend(pool.inner(), &indicator, days.unwrap_or(30))
        .await
        .map_err(|e| e.to_string())
}

/// Get all available indicator names with latest values and data point counts.
///
/// Frontend: invoke('get_available_indicators')
#[tauri::command]
pub async fn get_available_indicators(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<crate::services::trend_tracker::IndicatorSummary>, String> {
    crate::services::trend_tracker::get_available_indicators(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

/// Analyze a company by searching news and generating AI investment analysis.
///
/// Searches the news table for articles matching the query (company name or ticker),
/// aggregates results, and optionally generates AI-powered investment analysis.
///
/// Frontend: invoke('analyze_company', { query })
#[tauri::command]
pub async fn analyze_company(
    pool: State<'_, SqlitePool>,
    app: tauri::AppHandle,
    query: String,
) -> Result<company_intel::CompanyIntel, String> {
    company_intel::analyze_company(pool.inner(), &query, &app)
        .await
        .map_err(|e| e.to_string())
}

