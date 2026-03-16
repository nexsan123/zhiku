use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::services::indicator_engine;

/// A financial alert triggered by threshold rules against cycle indicators.
///
/// Persisted to the `signals` table and emitted to frontend via `alerts-triggered` event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub id: String,
    pub severity: String,
    pub category: String,
    pub title: String,
    pub detail: String,
    pub indicator_value: f64,
    pub threshold: f64,
    pub created_at: String,
}

/// Check all alert rules against current indicators. Returns new alerts (if any).
///
/// Called periodically from poll_loop (every 30 minutes).
/// Uses a cooldown mechanism: same alert category won't fire twice within 6 hours.
pub async fn check_alerts(pool: &SqlitePool) -> Result<Vec<Alert>, AppError> {
    let indicators = indicator_engine::calculate_cycle_indicators(pool).await?;
    let mut alerts = Vec::new();
    let now = Utc::now();

    // --- Rule 1: Extreme Fear (F&G < 15) ---
    if indicators.sentiment.fear_greed < 15.0 {
        alerts.push(Alert {
            id: Uuid::new_v4().to_string(),
            severity: "critical".to_string(),
            category: "sentiment".to_string(),
            title: "极度恐惧".to_string(),
            detail: format!(
                "恐惧贪婪指数降至{:.0}，市场处于极度恐惧状态",
                indicators.sentiment.fear_greed
            ),
            indicator_value: indicators.sentiment.fear_greed,
            threshold: 15.0,
            created_at: now.to_rfc3339(),
        });
    }
    // Rule 1b: Extreme Greed (F&G > 85)
    else if indicators.sentiment.fear_greed > 85.0 {
        alerts.push(Alert {
            id: Uuid::new_v4().to_string(),
            severity: "warning".to_string(),
            category: "sentiment".to_string(),
            title: "极度贪婪".to_string(),
            detail: format!(
                "恐惧贪婪指数升至{:.0}，市场处于极度贪婪状态（逆向信号）",
                indicators.sentiment.fear_greed
            ),
            indicator_value: indicators.sentiment.fear_greed,
            threshold: 85.0,
            created_at: now.to_rfc3339(),
        });
    }

    // --- Rule 2: VIX Spike (> 30) ---
    if indicators.market.vix_level > 30.0 {
        alerts.push(Alert {
            id: Uuid::new_v4().to_string(),
            severity: "critical".to_string(),
            category: "volatility".to_string(),
            title: "VIX 飙升".to_string(),
            detail: format!(
                "VIX波动率指数达{:.1}，超过30关键阈值",
                indicators.market.vix_level
            ),
            indicator_value: indicators.market.vix_level,
            threshold: 30.0,
            created_at: now.to_rfc3339(),
        });
    }

    // --- Rule 3: Yield curve inversion (spread < -0.2) ---
    if indicators.credit.credit_spread < -0.2 {
        alerts.push(Alert {
            id: Uuid::new_v4().to_string(),
            severity: "warning".to_string(),
            category: "credit".to_string(),
            title: "收益率曲线倒挂".to_string(),
            detail: format!(
                "10Y-2Y利差{:.2}pp，收益率曲线倒挂是衰退领先指标",
                indicators.credit.credit_spread
            ),
            indicator_value: indicators.credit.credit_spread,
            threshold: -0.2,
            created_at: now.to_rfc3339(),
        });
    }

    // --- Rule 4: Geopolitical critical ---
    if indicators.geopolitical.risk_level == "critical" {
        alerts.push(Alert {
            id: Uuid::new_v4().to_string(),
            severity: "critical".to_string(),
            category: "geopolitical".to_string(),
            title: "地缘风险极高".to_string(),
            detail: format!(
                "过去24小时{}条地缘政治新闻，风险等级critical",
                indicators.geopolitical.event_count
            ),
            indicator_value: indicators.geopolitical.event_count as f64,
            threshold: 10.0,
            created_at: now.to_rfc3339(),
        });
    }

    // --- Rule 5: S&P 500 daily drop > 3% ---
    if indicators.market.sp500_trend < -3.0 {
        alerts.push(Alert {
            id: Uuid::new_v4().to_string(),
            severity: "critical".to_string(),
            category: "volatility".to_string(),
            title: "标普500大跌".to_string(),
            detail: format!(
                "标普500过去24小时跌{:.1}%",
                indicators.market.sp500_trend
            ),
            indicator_value: indicators.market.sp500_trend,
            threshold: -3.0,
            created_at: now.to_rfc3339(),
        });
    }

    // --- Rule 6: CPI inflation > 5% ---
    if indicators.economic.cpi_inflation > 5.0 {
        alerts.push(Alert {
            id: Uuid::new_v4().to_string(),
            severity: "warning".to_string(),
            category: "macro".to_string(),
            title: "通胀过高".to_string(),
            detail: format!(
                "CPI同比通胀{:.1}%，超过5%警戒线",
                indicators.economic.cpi_inflation
            ),
            indicator_value: indicators.economic.cpi_inflation,
            threshold: 5.0,
            created_at: now.to_rfc3339(),
        });
    }

    // Apply cooldown: skip alerts of same category that fired within last 6 hours
    let mut filtered = Vec::new();
    for alert in alerts {
        let recent_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM signals WHERE signal_type = ?1 AND created_at >= datetime('now', '-6 hours')",
        )
        .bind(&alert.category)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        if recent_count == 0 {
            filtered.push(alert);
        }
    }

    // Persist new alerts to signals table
    for alert in &filtered {
        let data_json = serde_json::json!({
            "indicator_value": alert.indicator_value,
            "threshold": alert.threshold,
        })
        .to_string();

        sqlx::query(
            "INSERT INTO signals (id, signal_type, severity, title, summary, data, source_urls, ai_confidence, ai_model, created_at, pushed_to_qt)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, NULL, ?7, 0)",
        )
        .bind(&alert.id)
        .bind(&alert.category)
        .bind(&alert.severity)
        .bind(&alert.title)
        .bind(&alert.detail)
        .bind(&data_json)
        .bind(&alert.created_at)
        .execute(pool)
        .await
        .ok(); // Best effort -- don't fail if insert fails
    }

    Ok(filtered)
}

/// Get recent alerts (last 20, ordered by most recent first).
pub async fn get_recent_alerts(pool: &SqlitePool) -> Result<Vec<Alert>, AppError> {
    let rows: Vec<(String, String, String, String, Option<String>, Option<String>, String)> =
        sqlx::query_as(
            "SELECT id, signal_type, severity, title, summary, data, created_at \
             FROM signals ORDER BY created_at DESC LIMIT 20",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("Query alerts: {}", e)))?;

    let alerts = rows
        .into_iter()
        .map(
            |(id, category, severity, title, detail, data, created_at)| {
                let (indicator_value, threshold) = data
                    .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
                    .map(|v| {
                        (
                            v.get("indicator_value")
                                .and_then(|x| x.as_f64())
                                .unwrap_or(0.0),
                            v.get("threshold")
                                .and_then(|x| x.as_f64())
                                .unwrap_or(0.0),
                        )
                    })
                    .unwrap_or((0.0, 0.0));

                Alert {
                    id,
                    severity,
                    category,
                    title,
                    detail: detail.unwrap_or_default(),
                    indicator_value,
                    threshold,
                    created_at,
                }
            },
        )
        .collect();

    Ok(alerts)
}
