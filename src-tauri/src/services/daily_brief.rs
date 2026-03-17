//! Daily Intelligence Brief generator.
//!
//! Pure Rust rule engine -- no AI model calls. Synthesizes data from
//! indicator_engine, cycle_reasoner, deep_analyzer, scenario_engine,
//! and knowledge_base into a structured `DailyBrief` every 6 hours.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::ai::CycleIndicators;
use crate::models::ai::CycleReasoning;
use crate::services::{
    cycle_reasoner, deep_analyzer, indicator_engine, knowledge_base, scenario_engine,
};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Comprehensive daily intelligence brief.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyBrief {
    pub id: String,
    /// One-sentence headline summarising macro state.
    pub headline: String,
    /// Key contradictory signals detected across layers.
    pub key_contradictions: Vec<String>,
    /// Items requiring attention today.
    pub attention_items: Vec<AttentionItem>,
    /// Suggestion for QuantTerminal position/risk.
    pub qt_suggestion: QtSuggestion,
    /// Snapshot of core data points at generation time.
    pub data_snapshot: DataSnapshot,
    /// ISO 8601 timestamp.
    pub generated_at: String,
    /// Always "rust_rules" (no AI model used).
    pub model: String,
}

/// A single attention item with priority and category.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionItem {
    /// "high" | "medium" | "low"
    pub priority: String,
    /// "policy" | "geopolitical" | "market" | "data"
    pub category: String,
    pub content: String,
    pub reason: String,
}

/// Quantitative suggestion for QuantTerminal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QtSuggestion {
    /// 0 = fully short, 0.5 = neutral, 1 = fully long.
    pub position_bias: f64,
    /// Risk multiplier (fear -> lower).
    pub risk_multiplier: f64,
    /// "normal" | "monitor" | "action" | "emergency"
    pub urgency: String,
    pub sector_adjustments: Vec<SectorAdjustment>,
    pub reasoning: String,
}

/// Per-sector weight adjustment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorAdjustment {
    pub sector: String,
    /// 1.0 = neutral, >1 = overweight, <1 = underweight.
    pub weight: f64,
    pub reason: String,
}

/// Snapshot of core indicators at brief generation time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSnapshot {
    pub cycle_phase: String,
    pub cycle_confidence: f64,
    pub fear_greed: f64,
    pub fed_rate: f64,
    pub cpi_yoy: f64,
    pub gdp_growth: f64,
    pub credit_spread: f64,
    pub sp500_trend: f64,
    pub geopolitical_risk: String,
    pub geopolitical_events: i64,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate a comprehensive daily brief from current data.
///
/// Pure Rust rule engine -- collects data from existing services and
/// synthesises headline, contradictions, attention items, QT suggestion,
/// and a data snapshot. Persists result to `ai_analysis` table.
pub async fn generate_daily_brief(pool: &SqlitePool) -> Result<DailyBrief, AppError> {
    // 1. Collect data
    let indicators = indicator_engine::calculate_cycle_indicators(pool).await?;
    let latest_reasoning = cycle_reasoner::get_latest_reasoning(pool).await?;
    let recent_analyses = deep_analyzer::get_recent_analyses(pool, 3)
        .await
        .unwrap_or_default();
    let scenarios = scenario_engine::get_active_scenarios(pool)
        .await
        .unwrap_or_default();

    // 2. Build each section
    let headline = build_headline(&indicators, &latest_reasoning);
    let contradictions = detect_brief_contradictions(&indicators, &latest_reasoning);
    let attention_items = build_attention_items(&indicators, &recent_analyses, &scenarios);
    let qt_suggestion = build_qt_suggestion(&indicators);
    let snapshot = build_snapshot(&indicators, &latest_reasoning);

    let brief = DailyBrief {
        id: Uuid::new_v4().to_string(),
        headline,
        key_contradictions: contradictions,
        attention_items,
        qt_suggestion,
        data_snapshot: snapshot,
        generated_at: Utc::now().to_rfc3339(),
        model: "rust_rules".to_string(),
    };

    persist_brief(pool, &brief).await?;
    Ok(brief)
}

/// Retrieve the most recent daily brief from the database.
pub async fn get_latest_brief(pool: &SqlitePool) -> Result<Option<DailyBrief>, AppError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT output FROM ai_analysis WHERE analysis_type = 'daily_brief' ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query daily_brief: {}", e)))?;

    match row {
        Some((json,)) => {
            let brief: DailyBrief = serde_json::from_str(&json)
                .map_err(|e| AppError::Parse(format!("Parse daily_brief: {}", e)))?;
            Ok(Some(brief))
        }
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Builder helpers
// ---------------------------------------------------------------------------

/// Build a one-line Chinese headline from indicators + reasoning confidence.
fn build_headline(
    indicators: &CycleIndicators,
    reasoning: &Option<CycleReasoning>,
) -> String {
    let phase_zh = match indicators.economic.phase.as_str() {
        "recession" => "衰退期",
        "recovery" => "复苏期",
        "early_expansion" => "扩张早期",
        "mid_expansion" => "扩张中期",
        "late_expansion" => "扩张晚期",
        _ => "未知阶段",
    };

    let sentiment_zh = match indicators.sentiment.phase.as_str() {
        "panic" => "极度恐惧",
        "fear" => "恐惧",
        "caution" => "谨慎",
        "neutral" => "中性",
        "optimism" => "乐观",
        "euphoria" => "亢奋",
        _ => "未知",
    };

    let risk_zh = match indicators.geopolitical.risk_level.as_str() {
        "critical" => "地缘风险极高",
        "high" => "地缘风险较高",
        "elevated" => "地缘风险偏高",
        _ => "",
    };

    let confidence = reasoning.as_ref().map(|r| r.confidence).unwrap_or(0.0);

    if risk_zh.is_empty() {
        format!(
            "宏观{}，市场情绪{}，置信度{:.0}%",
            phase_zh,
            sentiment_zh,
            confidence * 100.0
        )
    } else {
        format!(
            "宏观{}，市场情绪{}，{}，置信度{:.0}%",
            phase_zh,
            sentiment_zh,
            risk_zh,
            confidence * 100.0
        )
    }
}

/// Detect contradictory signals across indicator layers.
fn detect_brief_contradictions(
    indicators: &CycleIndicators,
    _reasoning: &Option<CycleReasoning>,
) -> Vec<String> {
    let mut v = Vec::new();

    // 1. Economic expansion vs extreme fear
    if indicators.economic.phase.contains("expansion")
        && indicators.sentiment.fear_greed < 25.0
    {
        v.push(format!(
            "经济数据显示{}但恐惧贪婪指数仅{:.0}（极度恐惧），宏观基本面与市场情绪严重背离",
            indicators.economic.phase, indicators.sentiment.fear_greed
        ));
    }

    // 2. Cutting rates vs rising inflation
    if indicators.monetary.rate_direction == "cutting"
        && indicators.economic.cpi_inflation > 3.5
    {
        v.push(format!(
            "央行正在降息但CPI通胀达{:.1}%，货币政策与通胀趋势矛盾",
            indicators.economic.cpi_inflation
        ));
    }

    // 3. Yield curve inversion vs expansion
    if indicators.credit.credit_spread < -0.2
        && indicators.economic.phase.contains("expansion")
    {
        v.push(
            "收益率曲线倒挂（衰退信号）但经济仍判断为扩张期，需警惕拐点".to_string(),
        );
    }

    // 4. High VIX vs expansion
    if indicators.market.vix_level > 25.0
        && indicators.economic.phase.contains("expansion")
    {
        v.push(format!(
            "VIX波动率{:.1}偏高但判断扩张期，市场定价的风险高于宏观数据暗示的水平",
            indicators.market.vix_level
        ));
    }

    v
}

/// Build attention items from policy calendar, scenarios, deep analyses, and extreme indicators.
fn build_attention_items(
    indicators: &CycleIndicators,
    analyses: &[crate::models::intelligence::DeepAnalysis],
    scenarios: &scenario_engine::ScenarioMatrix,
) -> Vec<AttentionItem> {
    let mut items = Vec::new();

    // 1. Policy calendar events within 7 days
    let calendar_json = knowledge_base::POLICY_CALENDAR;
    if let Ok(cal) = serde_json::from_str::<serde_json::Value>(calendar_json) {
        if let Some(events) = cal.get("events").and_then(|e| e.as_array()) {
            let now = Utc::now();
            for event in events {
                if let (Some(date_str), Some(title)) = (
                    event.get("date").and_then(|d| d.as_str()),
                    event.get("title").and_then(|t| t.as_str()),
                ) {
                    if let Ok(event_date) =
                        chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    {
                        let days_until = (event_date - now.date_naive()).num_days();
                        if days_until >= 0 && days_until <= 7 {
                            items.push(AttentionItem {
                                priority: if days_until <= 2 {
                                    "high"
                                } else {
                                    "medium"
                                }
                                .to_string(),
                                category: "policy".to_string(),
                                content: title.to_string(),
                                reason: format!("{}天后", days_until),
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. High-probability scenarios
    for scenario in &scenarios.scenarios {
        if scenario.probability >= 0.6 {
            items.push(AttentionItem {
                priority: "high".to_string(),
                category: "geopolitical".to_string(),
                content: scenario.title.clone(),
                reason: format!("概率{:.0}%", scenario.probability * 100.0),
            });
        }
    }

    // 3. Key observations from deep analyses
    for analysis in analyses.iter().take(2) {
        items.push(AttentionItem {
            priority: if analysis.deep_analysis.confidence >= 0.7 {
                "medium"
            } else {
                "low"
            }
            .to_string(),
            category: "market".to_string(),
            content: analysis.key_observation.clone(),
            reason: format!(
                "置信度{:.0}%",
                analysis.deep_analysis.confidence * 100.0
            ),
        });
    }

    // 4. Extreme indicator alerts
    if indicators.sentiment.fear_greed < 15.0 {
        items.push(AttentionItem {
            priority: "high".to_string(),
            category: "market".to_string(),
            content: format!(
                "恐惧贪婪指数{:.0}，处于极度恐惧区间",
                indicators.sentiment.fear_greed
            ),
            reason: "可能触发逆向投资信号".to_string(),
        });
    }

    if indicators.market.vix_level > 30.0 {
        items.push(AttentionItem {
            priority: "high".to_string(),
            category: "market".to_string(),
            content: format!(
                "VIX波动率{:.1}，处于高波动区间",
                indicators.market.vix_level
            ),
            reason: "市场恐慌情绪加剧".to_string(),
        });
    }

    items
}

/// Build QuantTerminal suggestion from indicators using rule-based logic.
fn build_qt_suggestion(indicators: &CycleIndicators) -> QtSuggestion {
    // Position bias: base from cycle phase + sentiment adjustment
    let base_position: f64 = match indicators.economic.phase.as_str() {
        "recession" => 0.2,
        "recovery" => 0.5,
        "early_expansion" => 0.7,
        "mid_expansion" => 0.75,
        "late_expansion" => 0.6,
        _ => 0.5,
    };

    let sentiment_adj = if indicators.sentiment.fear_greed < 20.0 {
        -0.15
    } else if indicators.sentiment.fear_greed < 35.0 {
        -0.05
    } else if indicators.sentiment.fear_greed > 80.0 {
        -0.1 // contrarian: euphoria -> reduce
    } else {
        0.0
    };

    let position_bias: f64 = (base_position + sentiment_adj).clamp(0.1, 0.95);

    // Risk multiplier: VIX + geopolitical
    let vix_factor: f64 = if indicators.market.vix_level > 30.0 {
        0.5
    } else if indicators.market.vix_level > 25.0 {
        0.7
    } else if indicators.market.vix_level > 20.0 {
        0.85
    } else {
        1.0
    };

    let geo_factor = match indicators.geopolitical.risk_level.as_str() {
        "critical" => 0.6,
        "high" => 0.75,
        "elevated" => 0.9,
        _ => 1.0,
    };

    let risk_multiplier: f64 = (vix_factor * geo_factor).clamp(0.3, 1.0);

    // Urgency
    let urgency =
        if indicators.sentiment.fear_greed < 15.0 || indicators.market.vix_level > 35.0 {
            "action"
        } else if indicators.sentiment.fear_greed < 25.0
            || indicators.market.vix_level > 25.0
            || indicators.geopolitical.risk_level == "critical"
        {
            "monitor"
        } else {
            "normal"
        }
        .to_string();

    // Sector adjustments
    let mut sectors = Vec::new();

    if indicators.geopolitical.risk_level == "critical"
        || indicators.geopolitical.risk_level == "high"
    {
        sectors.push(SectorAdjustment {
            sector: "energy".to_string(),
            weight: 1.3,
            reason: format!(
                "地缘风险{}，能源供应风险溢价",
                indicators.geopolitical.risk_level
            ),
        });
    }

    if indicators.sentiment.fear_greed < 30.0 {
        sectors.push(SectorAdjustment {
            sector: "defensive".to_string(),
            weight: 1.2,
            reason: format!(
                "恐惧贪婪{:.0}，偏向防御",
                indicators.sentiment.fear_greed
            ),
        });
    }

    if indicators.monetary.rate_direction == "cutting"
        && indicators.economic.phase.contains("expansion")
    {
        sectors.push(SectorAdjustment {
            sector: "tech".to_string(),
            weight: 1.15,
            reason: "降息周期+经济扩张，利好成长股".to_string(),
        });
    }

    if indicators.economic.phase == "late_expansion" {
        sectors.push(SectorAdjustment {
            sector: "cyclical".to_string(),
            weight: 0.8,
            reason: "晚期扩张，周期股见顶风险".to_string(),
        });
    }

    let reasoning_text = format!(
        "周期{}，情绪{}(F&G={:.0})，VIX={:.1}，地缘{}，利率{}",
        indicators.economic.phase,
        indicators.sentiment.phase,
        indicators.sentiment.fear_greed,
        indicators.market.vix_level,
        indicators.geopolitical.risk_level,
        indicators.monetary.rate_direction,
    );

    QtSuggestion {
        position_bias,
        risk_multiplier,
        urgency,
        sector_adjustments: sectors,
        reasoning: reasoning_text,
    }
}

/// Build data snapshot from current indicators and reasoning.
fn build_snapshot(
    indicators: &CycleIndicators,
    reasoning: &Option<CycleReasoning>,
) -> DataSnapshot {
    DataSnapshot {
        cycle_phase: indicators.economic.phase.clone(),
        cycle_confidence: reasoning.as_ref().map(|r| r.confidence).unwrap_or(0.0),
        fear_greed: indicators.sentiment.fear_greed,
        fed_rate: indicators.monetary.fed_rate,
        cpi_yoy: indicators.economic.cpi_inflation,
        gdp_growth: indicators.economic.gdp_growth,
        credit_spread: indicators.credit.credit_spread,
        sp500_trend: indicators.market.sp500_trend,
        geopolitical_risk: indicators.geopolitical.risk_level.clone(),
        geopolitical_events: indicators.geopolitical.event_count,
    }
}

/// Persist a DailyBrief to the ai_analysis table.
async fn persist_brief(pool: &SqlitePool, brief: &DailyBrief) -> Result<(), AppError> {
    let output = serde_json::to_string(brief)
        .map_err(|e| AppError::Parse(format!("Serialize brief: {}", e)))?;

    sqlx::query(
        "INSERT INTO ai_analysis (id, analysis_type, input_ids, output, model, confidence, reasoning_chain, source_urls, created_at)
         VALUES (?1, 'daily_brief', NULL, ?2, ?3, NULL, NULL, NULL, ?4)",
    )
    .bind(&brief.id)
    .bind(&output)
    .bind(&brief.model)
    .bind(&brief.generated_at)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert daily_brief: {}", e)))?;

    log::info!("Daily brief persisted: id={}, headline={}", brief.id, brief.headline);
    Ok(())
}
