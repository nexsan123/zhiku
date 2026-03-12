use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use sqlx::SqlitePool;
use tokio::net::TcpListener;

use crate::services::{
    cycle_reasoner, deep_analyzer, dollar_tide, game_map,
    global_aggregator, indicator_engine, market_radar, scenario_engine,
};

/// Shared state for the REST server.
struct RestState {
    pool: SqlitePool,
}

/// Start the REST API server on port 9601.
/// Non-blocking -- spawns on its own tokio task via RT-001.
pub fn start_rest_server(pool: SqlitePool) {
    let state = Arc::new(RestState { pool });

    tauri::async_runtime::spawn(async move {
        let app = Router::new()
            .route("/api/v1/signals", get(get_signals))
            .route("/api/v1/macro-score", get(get_macro_score))
            .route("/api/v1/market-radar", get(get_market_radar))
            .route("/api/v1/ai-brief", get(get_ai_brief))
            .route("/api/v1/cycle", get(get_cycle))
            // Phase F: new edict-004 endpoints
            .route("/api/v1/credit-cycle", get(get_credit_cycle))
            .route("/api/v1/dollar-tide", get(get_dollar_tide))
            .route("/api/v1/game-map", get(get_game_map))
            .route("/api/v1/intelligence", get(get_intelligence))
            .with_state(state);

        let listener = match TcpListener::bind("127.0.0.1:9601").await {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to bind REST server on :9601: {}", e);
                return;
            }
        };

        log::info!("QuantTerminal REST API listening on http://127.0.0.1:9601");

        if let Err(e) = axum::serve(listener, app).await {
            log::error!("REST server error: {}", e);
        }
    });
}

/// GET /api/v1/signals -- recent AI signal events from news analysis
async fn get_signals(State(state): State<Arc<RestState>>) -> Json<serde_json::Value> {
    let rows: Vec<(String, String, f64, String)> = sqlx::query_as(
        r#"SELECT a.id, a.output, a.confidence, a.created_at
           FROM ai_analysis a
           WHERE a.analysis_type = 'news_summary'
             AND a.created_at >= datetime('now', '-24 hours')
           ORDER BY a.created_at DESC
           LIMIT 50"#,
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let signals: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(id, output, confidence, created_at)| {
            serde_json::json!({
                "id": id,
                "summary": output,
                "confidence": confidence,
                "timestamp": created_at,
            })
        })
        .collect();

    let count = signals.len();
    Json(serde_json::json!({ "signals": signals, "count": count }))
}

/// GET /api/v1/macro-score -- latest macro indicators from FRED
async fn get_macro_score(State(state): State<Arc<RestState>>) -> Json<serde_json::Value> {
    let rows: Vec<(String, f64, String)> = sqlx::query_as(
        r#"SELECT indicator, value, fetched_at
           FROM macro_data
           WHERE source = 'FRED'
           ORDER BY fetched_at DESC"#,
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Deduplicate by indicator (keep latest)
    let mut seen = std::collections::HashSet::new();
    let indicators: Vec<serde_json::Value> = rows
        .into_iter()
        .filter(|(ind, _, _)| seen.insert(ind.clone()))
        .map(|(indicator, value, fetched_at)| {
            serde_json::json!({
                "indicator": indicator,
                "value": value,
                "updatedAt": fetched_at,
            })
        })
        .collect();

    Json(serde_json::json!({ "indicators": indicators }))
}

/// GET /api/v1/market-radar -- 7-signal market radar
async fn get_market_radar(State(state): State<Arc<RestState>>) -> Json<serde_json::Value> {
    match market_radar::compute_radar(&state.pool).await {
        Ok(radar) => Json(serde_json::to_value(&radar).unwrap_or_default()),
        Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// GET /api/v1/ai-brief -- AI brief summaries by category
async fn get_ai_brief(State(state): State<Arc<RestState>>) -> Json<serde_json::Value> {
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
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(category, count, avg_sentiment, summary)| {
            serde_json::json!({
                "category": category,
                "count": count,
                "avgSentiment": avg_sentiment,
                "latestSummary": summary,
            })
        })
        .collect();

    Json(serde_json::json!({ "brief": items }))
}

/// GET /api/v1/cycle -- latest cycle reasoning
async fn get_cycle(State(state): State<Arc<RestState>>) -> Json<serde_json::Value> {
    let indicators = indicator_engine::calculate_cycle_indicators(&state.pool)
        .await
        .ok();

    let reasoning = cycle_reasoner::get_latest_reasoning(&state.pool)
        .await
        .ok()
        .flatten();

    Json(serde_json::json!({
        "indicators": indicators,
        "reasoning": reasoning,
    }))
}

/// GET /api/v1/credit-cycle -- 15-country credit cycle overview + global phase + dollar tide
async fn get_credit_cycle(State(state): State<Arc<RestState>>) -> Json<serde_json::Value> {
    match global_aggregator::compute_global_overview(&state.pool).await {
        Ok(overview) => Json(serde_json::to_value(&overview).unwrap_or_default()),
        Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// GET /api/v1/dollar-tide -- dollar tide state
async fn get_dollar_tide(State(state): State<Arc<RestState>>) -> Json<serde_json::Value> {
    match dollar_tide::compute_dollar_tide(&state.pool).await {
        Ok(tide) => Json(serde_json::to_value(&tide).unwrap_or_default()),
        Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
    }
}

/// GET /api/v1/game-map -- policy vectors + scenarios + decision calendar
async fn get_game_map(State(state): State<Arc<RestState>>) -> Json<serde_json::Value> {
    let vectors = game_map::get_policy_vectors(&state.pool)
        .await
        .unwrap_or_default();

    let bilaterals = game_map::get_bilateral_dynamics(&state.pool)
        .await
        .unwrap_or_default();

    let calendar = game_map::get_calendar_events(90)
        .unwrap_or_default();

    let scenarios = scenario_engine::get_active_scenarios(&state.pool)
        .await
        .unwrap_or_default();

    Json(serde_json::json!({
        "policyVectors": vectors,
        "bilateralDynamics": bilaterals,
        "decisionCalendar": calendar,
        "scenarios": scenarios,
    }))
}

/// GET /api/v1/intelligence -- deep analysis briefs (two-pass intelligence)
async fn get_intelligence(State(state): State<Arc<RestState>>) -> Json<serde_json::Value> {
    let analyses = deep_analyzer::get_recent_analyses(&state.pool, 10)
        .await
        .unwrap_or_default();

    Json(serde_json::json!({
        "analyses": analyses,
        "count": analyses.len(),
    }))
}
