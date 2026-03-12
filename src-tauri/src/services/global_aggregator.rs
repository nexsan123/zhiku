use chrono::Utc;
use sqlx::SqlitePool;

use crate::errors::AppError;
use crate::models::credit::{
    confidence_grade, CreditCyclePhase, CountryCyclePosition, CountryTier, DollarTide,
    GlobalCycleOverview, RiskAlert, TierSummary,
};
use crate::services::credit_cycle_engine;
use crate::services::dollar_tide::{self, tide_risk_modifier};

/// GDP weights for 15 countries (approximate share of world GDP, 2024 IMF data).
/// Used for weighted-average phase aggregation.
fn gdp_weight(code: &str) -> f64 {
    match code {
        "US" => 0.260,
        "CN" => 0.170,
        "XM" => 0.140, // Euro Area aggregate
        "JP" => 0.040,
        "GB" => 0.033,
        "CA" => 0.021,
        "AU" => 0.017,
        "KR" => 0.018,
        "IN" => 0.037,
        "BR" => 0.021,
        "TR" => 0.011,
        "AR" => 0.006,
        "ZA" => 0.004,
        "SA" => 0.011,
        "AE" => 0.005,
        _ => 0.005,
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute the full global credit cycle overview.
///
/// This is the main entry point: it calls credit_cycle_engine + dollar_tide,
/// then aggregates into GlobalCycleOverview.
pub async fn compute_global_overview(
    pool: &SqlitePool,
) -> Result<GlobalCycleOverview, AppError> {
    // Step 1: Compute dollar tide
    let tide = dollar_tide::compute_dollar_tide(pool).await?;
    let risk_mod = tide_risk_modifier(&tide);

    // Step 2: Compute per-country positions (with dollar tide risk modifier)
    let countries = credit_cycle_engine::compute_all_positions(pool, risk_mod).await?;

    // Step 3: Aggregate by tier
    let core_countries: Vec<&CountryCyclePosition> = countries
        .iter()
        .filter(|c| c.tier == CountryTier::Core)
        .collect();
    let important_countries: Vec<&CountryCyclePosition> = countries
        .iter()
        .filter(|c| c.tier == CountryTier::Important)
        .collect();
    let monitor_countries: Vec<&CountryCyclePosition> = countries
        .iter()
        .filter(|c| c.tier == CountryTier::Monitor)
        .collect();

    let core_summary = build_tier_summary(CountryTier::Core, &core_countries);
    let important_summary = build_tier_summary(CountryTier::Important, &important_countries);
    let monitor_summary = build_tier_summary(CountryTier::Monitor, &monitor_countries);

    // Step 4: GDP-weighted global phase
    let (global_phase, global_confidence) = weighted_global_phase(&countries);

    // Step 5: Historical percentile for global credit gap
    let global_percentile = compute_global_percentile(&countries);

    // Step 6: Risk alerts
    let risk_alerts = detect_risk_alerts(&countries, &tide);

    // Step 7: Latest data period
    let data_period = countries
        .iter()
        .find(|c| c.country_code == "US")
        .map(|c| c.data_period.clone())
        .unwrap_or_else(|| "N/A".to_string());

    Ok(GlobalCycleOverview {
        countries,
        global_phase_label: global_phase.display_name_zh().to_string(),
        global_phase,
        global_percentile,
        dollar_tide: tide,
        core_summary,
        important_summary,
        monitor_summary,
        risk_alerts,
        confidence: global_confidence,
        calculated_at: Utc::now().to_rfc3339(),
        data_period,
    })
}

// ---------------------------------------------------------------------------
// Tier summary
// ---------------------------------------------------------------------------

fn build_tier_summary(tier: CountryTier, countries: &[&CountryCyclePosition]) -> TierSummary {
    if countries.is_empty() {
        return TierSummary {
            tier,
            dominant_phase: CreditCyclePhase::Unknown,
            dominant_phase_label: "未知".to_string(),
            avg_credit_gap: 0.0,
            warning_count: 0,
        };
    }

    // GDP-weighted dominant phase
    let dominant_phase = weighted_phase(countries);

    // Average credit gap
    let gaps: Vec<f64> = countries
        .iter()
        .filter_map(|c| c.indicators.credit_gdp_gap)
        .collect();
    let avg_credit_gap = if gaps.is_empty() {
        0.0
    } else {
        gaps.iter().sum::<f64>() / gaps.len() as f64
    };

    // Warning count: credit gap > +10
    let warning_count = countries
        .iter()
        .filter(|c| c.indicators.credit_gdp_gap.map_or(false, |g| g > 10.0))
        .count();

    TierSummary {
        tier,
        dominant_phase_label: dominant_phase.display_name_zh().to_string(),
        dominant_phase,
        avg_credit_gap,
        warning_count,
    }
}

// ---------------------------------------------------------------------------
// GDP-weighted phase aggregation
// ---------------------------------------------------------------------------

/// Map phase to a numeric score for weighted averaging.
fn phase_score(phase: &CreditCyclePhase) -> f64 {
    match phase {
        CreditCyclePhase::Easing => 1.0,
        CreditCyclePhase::Leveraging => 2.0,
        CreditCyclePhase::Overheating => 3.0,
        CreditCyclePhase::Tightening => 4.0,
        CreditCyclePhase::Deleveraging => 5.0,
        CreditCyclePhase::Clearing => 6.0,
        CreditCyclePhase::Unknown => 3.5, // neutral middle
    }
}

/// Score back to nearest phase.
fn score_to_phase(score: f64) -> CreditCyclePhase {
    if score < 1.5 {
        CreditCyclePhase::Easing
    } else if score < 2.5 {
        CreditCyclePhase::Leveraging
    } else if score < 3.5 {
        CreditCyclePhase::Overheating
    } else if score < 4.5 {
        CreditCyclePhase::Tightening
    } else if score < 5.5 {
        CreditCyclePhase::Deleveraging
    } else {
        CreditCyclePhase::Clearing
    }
}

/// Compute GDP-weighted global phase from all 15 countries.
fn weighted_global_phase(countries: &[CountryCyclePosition]) -> (CreditCyclePhase, f64) {
    let mut total_weight = 0.0;
    let mut weighted_score = 0.0;
    let mut weighted_confidence = 0.0;

    for c in countries {
        if c.phase == CreditCyclePhase::Unknown {
            continue;
        }
        let w = gdp_weight(&c.country_code);
        weighted_score += phase_score(&c.phase) * w;
        weighted_confidence += c.confidence * w;
        total_weight += w;
    }

    if total_weight < 0.01 {
        return (CreditCyclePhase::Unknown, 0.0);
    }

    let avg_score = weighted_score / total_weight;
    let avg_confidence = weighted_confidence / total_weight;

    (score_to_phase(avg_score), avg_confidence)
}

/// Compute GDP-weighted phase for a tier subset.
fn weighted_phase(countries: &[&CountryCyclePosition]) -> CreditCyclePhase {
    let mut total_weight = 0.0;
    let mut weighted_score = 0.0;

    for c in countries {
        if c.phase == CreditCyclePhase::Unknown {
            continue;
        }
        let w = gdp_weight(&c.country_code);
        weighted_score += phase_score(&c.phase) * w;
        total_weight += w;
    }

    if total_weight < 0.01 {
        return CreditCyclePhase::Unknown;
    }

    score_to_phase(weighted_score / total_weight)
}

// ---------------------------------------------------------------------------
// Historical percentile (simplified)
// ---------------------------------------------------------------------------

/// Compute global credit gap percentile (0-100).
///
/// Simplified: maps average credit gap to a percentile using historical ranges.
/// Full implementation would query historical data; for now we use Basel III thresholds.
fn compute_global_percentile(countries: &[CountryCyclePosition]) -> f64 {
    let core_gaps: Vec<f64> = countries
        .iter()
        .filter(|c| c.tier == CountryTier::Core)
        .filter_map(|c| c.indicators.credit_gdp_gap)
        .collect();

    if core_gaps.is_empty() {
        return 50.0; // default to median when no data
    }

    let avg_gap = core_gaps.iter().sum::<f64>() / core_gaps.len() as f64;

    // Map gap to percentile using Basel III research:
    // -10 → ~5th percentile, 0 → ~50th, +10 → ~90th, +20 → ~99th
    let percentile = 50.0 + (avg_gap * 4.0); // linear approximation
    percentile.clamp(0.0, 100.0)
}

// ---------------------------------------------------------------------------
// Risk alert detection
// ---------------------------------------------------------------------------

/// Detect dangerous combinations and generate risk alerts.
fn detect_risk_alerts(
    countries: &[CountryCyclePosition],
    tide: &DollarTide,
) -> Vec<RiskAlert> {
    let mut alerts = Vec::new();

    // Alert 1: Any core country overheating
    for c in countries.iter().filter(|c| c.tier == CountryTier::Core) {
        if c.phase == CreditCyclePhase::Overheating {
            alerts.push(RiskAlert {
                country_code: c.country_code.clone(),
                alert: format!("{} 信用周期处于过热期，信用缺口可能超过安全线", c.country_name),
                severity: "danger".to_string(),
                confidence_grade: confidence_grade(c.confidence).to_string(),
            });
        }
    }

    // Alert 2: Overheating + dollar ebbing (worst combination for EM)
    if tide.tide_state == crate::models::credit::TideState::Ebbing {
        let em_overheating: Vec<&CountryCyclePosition> = countries
            .iter()
            .filter(|c| c.tier == CountryTier::Monitor || c.tier == CountryTier::Important)
            .filter(|c| {
                c.phase == CreditCyclePhase::Overheating
                    || c.phase == CreditCyclePhase::Deleveraging
            })
            .collect();

        if !em_overheating.is_empty() {
            let names: Vec<&str> = em_overheating.iter().map(|c| c.country_name.as_str()).collect();
            alerts.push(RiskAlert {
                country_code: "global".to_string(),
                alert: format!(
                    "美元退潮 + {} 处于高风险阶段，新兴市场债务危机风险升高",
                    names.join("、")
                ),
                severity: "critical".to_string(),
                confidence_grade: confidence_grade(tide.confidence).to_string(),
            });
        }
    }

    // Alert 3: Yield curve inversion (recession predictor)
    if tide.yield_spread < -0.5 {
        alerts.push(RiskAlert {
            country_code: "US".to_string(),
            alert: format!(
                "美国收益率曲线倒挂 ({:.2}pp)，历史上预示衰退概率较高",
                tide.yield_spread
            ),
            severity: "warning".to_string(),
            confidence_grade: "high".to_string(),
        });
    }

    // Alert 4: Multiple core countries in different phases (divergence)
    let core_phases: Vec<&CreditCyclePhase> = countries
        .iter()
        .filter(|c| c.tier == CountryTier::Core && c.phase != CreditCyclePhase::Unknown)
        .map(|c| &c.phase)
        .collect();

    if core_phases.len() >= 3 {
        let has_easing = core_phases.iter().any(|p| **p == CreditCyclePhase::Easing);
        let has_tightening = core_phases
            .iter()
            .any(|p| **p == CreditCyclePhase::Tightening);
        if has_easing && has_tightening {
            alerts.push(RiskAlert {
                country_code: "global".to_string(),
                alert: "核心国家信用周期分化：部分放水、部分收水，全球资本流动不确定性增大".to_string(),
                severity: "warning".to_string(),
                confidence_grade: "reasonable".to_string(),
            });
        }
    }

    alerts
}
