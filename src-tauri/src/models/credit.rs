use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Credit Cycle Phase — six stages (edict-004)
// ---------------------------------------------------------------------------

/// Six-phase credit cycle position, human-readable Chinese naming.
///
/// 放水期 → 加杠杆 → 过热期 → 收水期 → 去杠杆 → 出清期
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CreditCyclePhase {
    /// 放水期: central bank cutting + credit impulse turning positive
    Easing,
    /// 加杠杆: credit accelerating, impulse positive and rising
    Leveraging,
    /// 过热期: credit gap > +10% or DSR at historical high
    Overheating,
    /// 收水期: central bank hiking + credit impulse turning negative
    Tightening,
    /// 去杠杆: credit contracting + DSR rising + defaults
    Deleveraging,
    /// 出清期: credit bottoming + impulse starting to recover
    Clearing,
    /// 数据不足，无法定位
    Unknown,
}

impl CreditCyclePhase {
    /// Chinese display name for frontend.
    pub fn display_name_zh(&self) -> &'static str {
        match self {
            Self::Easing => "放水期",
            Self::Leveraging => "加杠杆",
            Self::Overheating => "过热期",
            Self::Tightening => "收水期",
            Self::Deleveraging => "去杠杆",
            Self::Clearing => "出清期",
            Self::Unknown => "未知",
        }
    }
}

// ---------------------------------------------------------------------------
// Per-country credit indicators
// ---------------------------------------------------------------------------

/// Core credit indicators for a single country, computed from BIS data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountryCreditData {
    /// BIS credit-to-GDP gap (percentage points). >+10 = Basel III warning.
    pub credit_gdp_gap: Option<f64>,
    /// Debt service ratio (% of income spent on debt service).
    pub debt_service_ratio: Option<f64>,
    /// Credit growth year-over-year (%).
    pub credit_growth_yoy: Option<f64>,
    /// Credit impulse: change in new credit / GDP (second derivative). THE core signal.
    pub credit_impulse: Option<f64>,
    /// Property price trend (YoY %, from BIS WS_SPP). Kuznets cycle reference.
    pub property_price_trend: Option<f64>,
    /// BIS policy rate for this country (from WS_CBPOL).
    pub policy_rate: Option<f64>,
    /// Policy rate direction: "hiking", "pausing", "cutting", "unknown".
    pub rate_direction: String,
}

// ---------------------------------------------------------------------------
// Per-country cycle position
// ---------------------------------------------------------------------------

/// Country tier in the monitoring framework.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CountryTier {
    /// 核心圈: US, CN, XM, JP
    Core,
    /// 重要圈: GB, CA, AU, KR, IN, BR
    Important,
    /// 监控圈: TR, AR, ZA, SA, AE
    Monitor,
}

/// Full cycle positioning for one country.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountryCyclePosition {
    /// BIS country/area code (e.g. "US", "CN", "XM").
    pub country_code: String,
    /// Human-readable country name.
    pub country_name: String,
    /// Tier in monitoring framework.
    pub tier: CountryTier,
    /// Computed credit indicators.
    pub indicators: CountryCreditData,
    /// Determined cycle phase.
    pub phase: CreditCyclePhase,
    /// Chinese display name for the phase.
    pub phase_label: String,
    /// Confidence in this positioning (0.0 - 1.0).
    pub confidence: f64,
    /// Confidence grade: "high", "reasonable", "speculative".
    pub confidence_grade: String,
    /// Data reliability score for this country (from config).
    pub reliability: f64,
    /// Whether dollar tide modifies this country's risk level.
    pub dollar_tide_risk_modifier: i8,
    /// Latest BIS data period (e.g. "2025-Q2").
    pub data_period: String,
}

// ---------------------------------------------------------------------------
// Dollar Tide (transmission layer)
// ---------------------------------------------------------------------------

/// Dollar tide state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TideState {
    /// 涨潮: dollar weakening, EM benefiting
    Rising,
    /// 平潮: mixed signals
    Neutral,
    /// 退潮: dollar strengthening, EM under pressure
    Ebbing,
}

impl TideState {
    pub fn display_name_zh(&self) -> &'static str {
        match self {
            Self::Rising => "涨潮",
            Self::Neutral => "平潮",
            Self::Ebbing => "退潮",
        }
    }
}

/// Dollar tide assessment — how US monetary conditions transmit globally.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DollarTide {
    /// DXY 3-month trend (%).
    pub dxy_trend_3m: f64,
    /// DXY 6-month trend (%).
    pub dxy_trend_6m: f64,
    /// Fed policy direction: "hiking", "pausing", "cutting".
    pub fed_policy: String,
    /// M2 money supply growth YoY (%).
    pub m2_growth: f64,
    /// Yield spread: 10Y - 2Y treasury (percentage points). Negative = inverted.
    pub yield_spread: f64,
    /// Determined tide state.
    pub tide_state: TideState,
    /// Chinese display name.
    pub tide_label: String,
    /// Confidence in this assessment.
    pub confidence: f64,
}

// ---------------------------------------------------------------------------
// Global aggregation
// ---------------------------------------------------------------------------

/// Global credit cycle overview — the main output of the credit cycle engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalCycleOverview {
    /// Per-country cycle positions (15 countries).
    pub countries: Vec<CountryCyclePosition>,
    /// GDP-weighted global cycle phase.
    pub global_phase: CreditCyclePhase,
    /// Chinese label for global phase.
    pub global_phase_label: String,
    /// Global credit gap percentile vs history (0-100).
    pub global_percentile: f64,
    /// Dollar tide assessment.
    pub dollar_tide: DollarTide,
    /// Core tier (US/CN/XM/JP) summary.
    pub core_summary: TierSummary,
    /// Important tier summary.
    pub important_summary: TierSummary,
    /// Monitor tier summary (with dollar tide risk flags).
    pub monitor_summary: TierSummary,
    /// High-risk combinations detected (e.g. "overheating + dollar ebbing").
    pub risk_alerts: Vec<RiskAlert>,
    /// Overall confidence.
    pub confidence: f64,
    /// ISO 8601 timestamp.
    pub calculated_at: String,
    /// Latest BIS data period used.
    pub data_period: String,
}

/// Summary for a country tier group.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierSummary {
    pub tier: CountryTier,
    /// GDP-weighted average phase for this tier.
    pub dominant_phase: CreditCyclePhase,
    pub dominant_phase_label: String,
    /// Average credit gap for this tier.
    pub avg_credit_gap: f64,
    /// Number of countries in warning zone (credit gap > +10).
    pub warning_count: usize,
}

/// A specific risk alert when dangerous combinations are detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskAlert {
    /// Country code (or "global").
    pub country_code: String,
    /// Alert description.
    pub alert: String,
    /// Severity: "warning", "danger", "critical".
    pub severity: String,
    /// Confidence grade.
    pub confidence_grade: String,
}

// ---------------------------------------------------------------------------
// Confidence grading utility
// ---------------------------------------------------------------------------

/// Compute confidence grade from a numeric confidence value.
pub fn confidence_grade(confidence: f64) -> &'static str {
    if confidence >= 0.80 {
        "high"
    } else if confidence >= 0.50 {
        "reasonable"
    } else {
        "speculative"
    }
}

/// Compute confidence grade in Chinese for display.
pub fn confidence_grade_zh(confidence: f64) -> &'static str {
    if confidence >= 0.80 {
        "高确信"
    } else if confidence >= 0.50 {
        "合理推断"
    } else {
        "推测性"
    }
}
