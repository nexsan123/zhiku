use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// News Cluster — Rust-computed grouping of related news
// ---------------------------------------------------------------------------

/// A cluster of related news articles (same region + shared entities).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsCluster {
    /// Unique cluster identifier.
    pub cluster_id: String,
    /// Auto-generated topic hint from shared entities/regions.
    pub topic_hint: String,
    /// News article IDs in this cluster.
    pub news_ids: Vec<String>,
    /// Regions involved across all news in cluster.
    pub regions: Vec<String>,
    /// Entities involved across all news in cluster.
    pub entities: Vec<String>,
    /// Number of news in this cluster.
    pub news_count: usize,
    /// ISO 8601 timestamp when cluster was created.
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Deep Analysis — Claude second-pass output
// ---------------------------------------------------------------------------

/// Deep motive analysis for a news cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepMotiveAnalysis {
    /// Primary motive behind the cluster of events.
    pub primary_motive: String,
    /// Secondary/hidden motive (if detected).
    pub secondary_motive: String,
    /// Political bias warning: flags if source news had detectable political bias.
    #[serde(default)]
    pub bias_warning: String,
    /// Confidence in this analysis (0.0-1.0).
    pub confidence: f64,
    /// Confidence grade: "high", "reasonable", "speculative".
    pub confidence_grade: String,
}

/// Impact assessment across the five-layer world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerImpact {
    /// Physical layer impact (energy, food, demographics).
    pub physical: String,
    /// Credit layer impact (credit cycles, BIS indicators).
    pub credit: String,
    /// Dollar/transmission layer impact (capital flows, DXY).
    pub dollar: String,
    /// Geopolitical layer impact (state actors, alliances).
    pub geopolitical: String,
    /// Sentiment layer impact (market fear/greed, narrative shifts).
    pub sentiment: String,
}

/// Full deep analysis output for a news cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepAnalysis {
    /// Cluster this analysis belongs to.
    pub cluster_id: String,
    /// Topic of the cluster.
    pub cluster_topic: String,
    /// Number of news articles analyzed.
    pub news_count: usize,
    /// Surface-level description (what happened).
    pub surface: String,
    /// Connection between the news articles (why they're related).
    pub connection: String,
    /// Deep motive analysis (why it really happened).
    pub deep_analysis: DeepMotiveAnalysis,
    /// Impact across five layers.
    pub layer_impact: LayerImpact,
    /// Key observation / actionable insight.
    pub key_observation: String,
    /// Source URLs for traceability (ZK-01).
    pub source_urls: Vec<String>,
    /// ISO 8601 timestamp.
    pub analyzed_at: String,
}

/// Confidence grade enum for typed usage.
/// Currently not directly constructed in Rust but reserved for future frontend/API use.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceGrade {
    High,
    Reasonable,
    Speculative,
}

#[allow(dead_code)]
impl ConfidenceGrade {
    pub fn from_score(confidence: f64) -> Self {
        if confidence >= 0.80 {
            Self::High
        } else if confidence >= 0.50 {
            Self::Reasonable
        } else {
            Self::Speculative
        }
    }

    pub fn display_color(&self) -> &'static str {
        match self {
            Self::High => "green",
            Self::Reasonable => "yellow",
            Self::Speculative => "gray",
        }
    }
}
