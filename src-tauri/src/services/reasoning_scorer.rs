use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::ai::CycleReasoning;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A single scorecard entry for frontend display.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScorecardEntry {
    pub id: String,
    pub created_at: String,
    pub predicted_direction: String,
    pub confidence: f64,
    pub contradictions: Vec<String>,
    pub contradiction_count: i32,
    pub actual_sp500_7d: Option<f64>,
    pub actual_sp500_30d: Option<f64>,
    pub direction_correct_7d: Option<i32>,
    pub direction_correct_30d: Option<i32>,
    pub human_verdict: Option<String>,
    pub human_note: Option<String>,
}

/// Aggregated accuracy statistics across all scored records.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccuracyStats {
    pub total_scored: i64,
    pub direction_correct_7d: i64,
    pub direction_correct_30d: i64,
    pub accuracy_7d: f64,
    pub accuracy_30d: f64,
    pub avg_confidence: f64,
    pub contradiction_rate: f64,
}

// ---------------------------------------------------------------------------
// record_scorecard — called after each cycle reasoning persist
// ---------------------------------------------------------------------------

/// Record a scorecard for the latest cycle reasoning.
///
/// Extracts verifiable predictions (direction, signals, sectors, tail risks),
/// runs contradiction detection, and writes to the `reasoning_scorecard` table.
/// Uses Method C: queries the latest `ai_analysis` row for the reasoning_id.
pub async fn record_scorecard(
    pool: &SqlitePool,
    reasoning: &CycleReasoning,
) -> Result<(), AppError> {
    // Fetch the latest cycle_reasoning id from ai_analysis
    let reasoning_id: String = sqlx::query_scalar(
        "SELECT id FROM ai_analysis WHERE analysis_type = 'cycle_reasoning' ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query latest reasoning id failed: {}", e)))?
    .unwrap_or_else(|| "unknown".to_string());

    // Score direction from turning signals
    let direction = score_direction(reasoning);

    // Detect contradictions
    let contradictions = detect_contradictions(reasoning);
    let contradiction_count = contradictions.len() as i32;

    // Serialize lists to JSON
    let signals_json = serde_json::to_string(&reasoning.turning_signals)
        .unwrap_or_else(|_| "[]".to_string());
    let sectors_json = serde_json::to_string(&reasoning.sector_recommendations)
        .unwrap_or_else(|_| "[]".to_string());
    let risks_json = serde_json::to_string(&reasoning.tail_risks)
        .unwrap_or_else(|_| "[]".to_string());
    let contradictions_json = serde_json::to_string(&contradictions)
        .unwrap_or_else(|_| "[]".to_string());

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"INSERT INTO reasoning_scorecard
           (id, reasoning_id, created_at, predicted_direction, predicted_signals,
            predicted_sectors, predicted_tail_risks, confidence,
            contradictions, contradiction_count)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
    )
    .bind(&id)
    .bind(&reasoning_id)
    .bind(&now)
    .bind(&direction)
    .bind(&signals_json)
    .bind(&sectors_json)
    .bind(&risks_json)
    .bind(reasoning.confidence)
    .bind(&contradictions_json)
    .bind(contradiction_count)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert reasoning_scorecard failed: {}", e)))?;

    log::info!(
        "Scorecard recorded: id={}, direction={}, contradictions={}, confidence={:.2}",
        id, direction, contradiction_count, reasoning.confidence
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Direction scoring
// ---------------------------------------------------------------------------

/// Score overall direction from turning signals.
///
/// Bullish signals add points (strong=3, moderate=2, weak=1).
/// Bearish signals subtract points.
/// Total > 0 = "bullish", < 0 = "bearish", = 0 = "neutral".
fn score_direction(reasoning: &CycleReasoning) -> String {
    let mut score: i32 = 0;
    for signal in &reasoning.turning_signals {
        let weight = match signal.strength.as_str() {
            "strong" => 3,
            "moderate" => 2,
            "weak" => 1,
            _ => 1,
        };
        match signal.direction.as_str() {
            "bullish" => score += weight,
            "bearish" => score -= weight,
            _ => {}
        }
    }
    if score > 0 {
        "bullish".to_string()
    } else if score < 0 {
        "bearish".to_string()
    } else {
        "neutral".to_string()
    }
}

// ---------------------------------------------------------------------------
// Contradiction detection (5 rules)
// ---------------------------------------------------------------------------

/// Detect logical contradictions in AI reasoning output.
///
/// Returns a list of human-readable contradiction descriptions.
/// Rules are based on domain knowledge of financial cycle analysis.
fn detect_contradictions(reasoning: &CycleReasoning) -> Vec<String> {
    let mut contradictions = Vec::new();

    // Rule 1: Expansion cycle but panic/fear sentiment
    if reasoning.cycle_position.contains("expansion")
        && (reasoning.sentiment_stage == "panic" || reasoning.sentiment_stage == "fear")
    {
        contradictions.push(
            "情绪与周期矛盾：判断扩张但市场处于恐惧".to_string(),
        );
    }

    // Rule 2: Bearish signals dominate but recommends cyclical/tech (offensive sectors)
    let bullish_count = reasoning
        .turning_signals
        .iter()
        .filter(|s| s.direction == "bullish")
        .count();
    let bearish_count = reasoning
        .turning_signals
        .iter()
        .filter(|s| s.direction == "bearish")
        .count();
    if bearish_count > bullish_count
        && reasoning
            .sector_recommendations
            .iter()
            .any(|s| s == "cyclical" || s == "tech")
    {
        contradictions.push(
            "信号与板块矛盾：bearish信号为主但推荐进攻性板块".to_string(),
        );
    }

    // Rule 3: High confidence but all signals are weak
    if reasoning.confidence >= 0.7
        && !reasoning.turning_signals.is_empty()
        && reasoning
            .turning_signals
            .iter()
            .all(|s| s.strength == "weak")
    {
        contradictions.push(
            "过度自信：置信度>=0.7但所有信号都是weak".to_string(),
        );
    }

    // Rule 4: Recommends defensive but judges expansion
    if reasoning
        .sector_recommendations
        .iter()
        .any(|s| s == "defensive")
        && reasoning.cycle_position.contains("expansion")
    {
        contradictions.push(
            "板块矛盾：推荐防御性但判断扩张期".to_string(),
        );
    }

    // Rule 5: Recession judgment but low confidence (< 0.6)
    if reasoning.cycle_position.contains("recession") && reasoning.confidence < 0.6 {
        contradictions.push(
            "低信心衰退判断：证据不足以支持衰退结论".to_string(),
        );
    }

    contradictions
}

// ---------------------------------------------------------------------------
// backfill_actuals — daily task to fill in actual market results
// ---------------------------------------------------------------------------

/// Backfill actual S&P 500 results for scorecard records that are old enough.
///
/// Checks for records needing 7-day and 30-day results, fetches prices from
/// `market_snap`, calculates percentage change, and scores direction accuracy.
///
/// Returns the number of records updated.
pub async fn backfill_actuals(pool: &SqlitePool) -> Result<usize, AppError> {
    let mut updated = 0usize;

    // 7-day backfill
    let pending_7d: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, created_at FROM reasoning_scorecard \
         WHERE actual_sp500_7d IS NULL \
         AND julianday('now') - julianday(created_at) >= 7.0",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query pending 7d scorecards failed: {}", e)))?;

    for (id, created_at) in &pending_7d {
        let price_then = get_price_at(pool, "^GSPC", created_at).await;
        let price_after = get_price_after(pool, "^GSPC", created_at, 7).await;

        if let (Some(then), Some(after)) = (price_then, price_after) {
            let change_pct = (after - then) / then * 100.0;
            let predicted_dir = get_predicted_direction(pool, &id).await;
            let correct = match predicted_dir.as_deref() {
                Some("bullish") => {
                    if change_pct > 0.0 { 1 } else { 0 }
                }
                Some("bearish") => {
                    if change_pct < 0.0 { 1 } else { 0 }
                }
                _ => -1, // neutral — not scored
            };

            let result = sqlx::query(
                "UPDATE reasoning_scorecard SET actual_sp500_7d = ?1, direction_correct_7d = ?2 WHERE id = ?3",
            )
            .bind(change_pct)
            .bind(correct)
            .bind(&id)
            .execute(pool)
            .await;

            if let Err(e) = result {
                log::warn!("Scorecard backfill 7d failed for {}: {}", id, e);
            } else {
                updated += 1;
            }
        }
    }

    // 30-day backfill
    let pending_30d: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, created_at FROM reasoning_scorecard \
         WHERE actual_sp500_30d IS NULL \
         AND julianday('now') - julianday(created_at) >= 30.0",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query pending 30d scorecards failed: {}", e)))?;

    for (id, created_at) in &pending_30d {
        let price_then = get_price_at(pool, "^GSPC", created_at).await;
        let price_after = get_price_after(pool, "^GSPC", created_at, 30).await;

        if let (Some(then), Some(after)) = (price_then, price_after) {
            let change_pct = (after - then) / then * 100.0;
            let predicted_dir = get_predicted_direction(pool, &id).await;
            let correct = match predicted_dir.as_deref() {
                Some("bullish") => {
                    if change_pct > 0.0 { 1 } else { 0 }
                }
                Some("bearish") => {
                    if change_pct < 0.0 { 1 } else { 0 }
                }
                _ => -1,
            };

            let result = sqlx::query(
                "UPDATE reasoning_scorecard SET actual_sp500_30d = ?1, direction_correct_30d = ?2 WHERE id = ?3",
            )
            .bind(change_pct)
            .bind(correct)
            .bind(&id)
            .execute(pool)
            .await;

            if let Err(e) = result {
                log::warn!("Scorecard backfill 30d failed for {}: {}", id, e);
            } else {
                updated += 1;
            }
        }
    }

    Ok(updated)
}

// ---------------------------------------------------------------------------
// Price lookup helpers
// ---------------------------------------------------------------------------

/// Get the closest price at or before the given datetime.
async fn get_price_at(pool: &SqlitePool, symbol: &str, datetime: &str) -> Option<f64> {
    let result: Option<(f64,)> = sqlx::query_as(
        "SELECT price FROM market_snap \
         WHERE symbol = ?1 AND julianday(timestamp) <= julianday(?2) \
         ORDER BY julianday(timestamp) DESC LIMIT 1",
    )
    .bind(symbol)
    .bind(datetime)
    .fetch_optional(pool)
    .await
    .ok()?;

    result.map(|(price,)| price)
}

/// Get the closest price at or after datetime + N days.
async fn get_price_after(
    pool: &SqlitePool,
    symbol: &str,
    datetime: &str,
    days: i32,
) -> Option<f64> {
    let result: Option<(f64,)> = sqlx::query_as(
        "SELECT price FROM market_snap \
         WHERE symbol = ?1 AND julianday(timestamp) >= julianday(?2) + ?3 \
         ORDER BY julianday(timestamp) ASC LIMIT 1",
    )
    .bind(symbol)
    .bind(datetime)
    .bind(days)
    .fetch_optional(pool)
    .await
    .ok()?;

    result.map(|(price,)| price)
}

/// Get the predicted direction for a scorecard record.
async fn get_predicted_direction(pool: &SqlitePool, scorecard_id: &str) -> Option<String> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT predicted_direction FROM reasoning_scorecard WHERE id = ?1",
    )
    .bind(scorecard_id)
    .fetch_optional(pool)
    .await
    .ok()?;

    result.map(|(dir,)| dir)
}

// ---------------------------------------------------------------------------
// Query functions (for frontend)
// ---------------------------------------------------------------------------

/// Get the most recent N scorecard entries.
pub async fn get_recent_scorecards(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<ScorecardEntry>, AppError> {
    let rows: Vec<(
        String,  // id
        String,  // created_at
        String,  // predicted_direction
        f64,     // confidence
        String,  // contradictions (JSON)
        i32,     // contradiction_count
        Option<f64>,  // actual_sp500_7d
        Option<f64>,  // actual_sp500_30d
        Option<i32>,  // direction_correct_7d
        Option<i32>,  // direction_correct_30d
        Option<String>, // human_verdict
        Option<String>, // human_note
    )> = sqlx::query_as(
        r#"SELECT id, created_at, predicted_direction, confidence,
                  contradictions, contradiction_count,
                  actual_sp500_7d, actual_sp500_30d,
                  direction_correct_7d, direction_correct_30d,
                  human_verdict, human_note
           FROM reasoning_scorecard
           ORDER BY created_at DESC
           LIMIT ?1"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query recent scorecards failed: {}", e)))?;

    let entries = rows
        .into_iter()
        .map(|row| {
            let contradictions: Vec<String> =
                serde_json::from_str(&row.4).unwrap_or_default();
            ScorecardEntry {
                id: row.0,
                created_at: row.1,
                predicted_direction: row.2,
                confidence: row.3,
                contradictions,
                contradiction_count: row.5,
                actual_sp500_7d: row.6,
                actual_sp500_30d: row.7,
                direction_correct_7d: row.8,
                direction_correct_30d: row.9,
                human_verdict: row.10,
                human_note: row.11,
            }
        })
        .collect();

    Ok(entries)
}

/// Get aggregated accuracy statistics across all scored records.
pub async fn get_accuracy_stats(pool: &SqlitePool) -> Result<AccuracyStats, AppError> {
    // Total records with any actual result (7d or 30d)
    let total_scored: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reasoning_scorecard \
         WHERE direction_correct_7d IS NOT NULL OR direction_correct_30d IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query total_scored failed: {}", e)))?;

    // 7d correct count (only count records where direction_correct_7d = 1)
    let correct_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reasoning_scorecard WHERE direction_correct_7d = 1",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query correct_7d failed: {}", e)))?;

    // 7d scored count (exclude neutral = -1)
    let scored_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reasoning_scorecard \
         WHERE direction_correct_7d IS NOT NULL AND direction_correct_7d >= 0",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query scored_7d failed: {}", e)))?;

    // 30d correct count
    let correct_30d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reasoning_scorecard WHERE direction_correct_30d = 1",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query correct_30d failed: {}", e)))?;

    // 30d scored count
    let scored_30d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reasoning_scorecard \
         WHERE direction_correct_30d IS NOT NULL AND direction_correct_30d >= 0",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query scored_30d failed: {}", e)))?;

    // Average confidence across all scorecards
    let avg_confidence: f64 = sqlx::query_scalar(
        "SELECT COALESCE(AVG(confidence), 0.0) FROM reasoning_scorecard",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query avg_confidence failed: {}", e)))?;

    // Contradiction rate: scorecards with contradiction_count > 0 / total
    let total_all: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reasoning_scorecard",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query total_all failed: {}", e)))?;

    let with_contradictions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reasoning_scorecard WHERE contradiction_count > 0",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query with_contradictions failed: {}", e)))?;

    let accuracy_7d = if scored_7d > 0 {
        correct_7d as f64 / scored_7d as f64
    } else {
        0.0
    };

    let accuracy_30d = if scored_30d > 0 {
        correct_30d as f64 / scored_30d as f64
    } else {
        0.0
    };

    let contradiction_rate = if total_all > 0 {
        with_contradictions as f64 / total_all as f64
    } else {
        0.0
    };

    Ok(AccuracyStats {
        total_scored,
        direction_correct_7d: correct_7d,
        direction_correct_30d: correct_30d,
        accuracy_7d,
        accuracy_30d,
        avg_confidence,
        contradiction_rate,
    })
}
