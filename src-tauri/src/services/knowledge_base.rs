//! Knowledge base constants for AI prompt injection.
//!
//! Static JSON data compiled into the binary via `include_str!()`.
//! Each AI service module picks only the subset it needs.

/// 15-country structural profiles (economy, energy, military, finance, geopolitics).
pub const COUNTRY_PROFILES: &str = include_str!("../data/country_profiles.json");

/// 8 structural causal chains (dollar hegemony, credit cycle, etc.).
pub const POWER_STRUCTURES: &str = include_str!("../data/power_structures.json");

/// 15 bilateral geopolitical edges + 3 meta patterns.
pub const GEOPOLITICAL_GRAPH: &str = include_str!("../data/geopolitical_graph.json");

/// 59 RSS source bias annotations with lean/reliability ratings.
pub const MEDIA_BIAS_REGISTRY: &str = include_str!("../data/media_bias_registry.json");

/// Per-country per-indicator data reliability scores (0.0-1.0).
pub const DATA_RELIABILITY: &str = include_str!("../data/data_reliability.json");

/// US policy decision calendar (FOMC, Treasury reports, trade reviews).
pub const POLICY_CALENDAR: &str = include_str!("../data/policy_calendar.json");

/// 15 geopolitical event trigger templates with probability, transmission paths, and market impacts.
pub const EVENT_TRIGGERS: &str = include_str!("../data/event_triggers.json");

/// 16-country role positioning + control chain positions + behavioral models + transition signals.
pub const COUNTRY_ROLES: &str = include_str!("../data/country_roles.json");

// ---------------------------------------------------------------------------
// Slim media-bias extract (for token-constrained providers like Groq 8K)
// ---------------------------------------------------------------------------

use std::sync::LazyLock;

static MEDIA_BIAS_SLIM_CACHE: LazyLock<String> = LazyLock::new(|| {
    let parsed: serde_json::Value =
        serde_json::from_str(MEDIA_BIAS_REGISTRY).unwrap_or_default();
    let sources = parsed.get("sources").and_then(|s| s.as_object());
    match sources {
        Some(map) => {
            let mut lines: Vec<String> = map
                .iter()
                .map(|(name, v)| {
                    let lean = v.get("lean").and_then(|l| l.as_f64()).unwrap_or(0.0);
                    let label = v
                        .get("lean_label")
                        .and_then(|l| l.as_str())
                        .unwrap_or("unknown");
                    let reliability = v
                        .get("factual_reliability")
                        .and_then(|r| r.as_f64())
                        .unwrap_or(0.0);
                    let ownership = v
                        .get("ownership")
                        .and_then(|o| o.as_str())
                        .unwrap_or("unknown");
                    format!(
                        "{}: lean={:.1}({}), reliability={:.2}, {}",
                        name, lean, label, reliability, ownership
                    )
                })
                .collect();
            lines.sort();
            lines.join("\n")
        }
        None => String::from("(media bias data unavailable)"),
    }
});

/// Return a compact one-line-per-source summary of media bias data.
///
/// Output is ~3 KB vs the full 30 KB JSON -- suitable for Groq 8K context.
pub fn media_bias_slim() -> &'static str {
    &MEDIA_BIAS_SLIM_CACHE
}

// ---------------------------------------------------------------------------
// Slim country-profiles extract (~500 bytes vs 44 KB)
// ---------------------------------------------------------------------------

static COUNTRY_PROFILES_SLIM_CACHE: LazyLock<String> = LazyLock::new(|| {
    let parsed: serde_json::Value =
        serde_json::from_str(COUNTRY_PROFILES).unwrap_or_default();
    let countries = parsed.get("countries").and_then(|c| c.as_object());
    match countries {
        Some(map) => {
            let mut lines: Vec<String> = map
                .iter()
                .map(|(code, v)| {
                    let name_zh = v.get("name_zh").and_then(|n| n.as_str()).unwrap_or(code);
                    let tier = v.get("tier").and_then(|t| t.as_str()).unwrap_or("?");
                    let cred = v.get("stats_credibility").and_then(|c| c.as_f64()).unwrap_or(0.0);

                    let econ = v.get("economic_base").unwrap_or(&serde_json::Value::Null);
                    let gdp = econ.get("gdp_usd").and_then(|g| g.as_f64()).unwrap_or(0.0);
                    let gdp_rank = econ.get("gdp_rank").and_then(|r| r.as_u64()).unwrap_or(0);

                    let energy = v.get("energy").unwrap_or(&serde_json::Value::Null);
                    let oil_prod = energy.get("oil_production_crude_bpd").and_then(|o| o.as_f64()).unwrap_or(0.0);

                    let mil = v.get("military").unwrap_or(&serde_json::Value::Null);
                    let mil_budget = mil.get("budget_usd").and_then(|b| b.as_f64()).unwrap_or(0.0);
                    let mil_pct = mil.get("budget_pct_gdp").and_then(|p| p.as_f64()).unwrap_or(0.0);

                    let fin = v.get("financial_position").unwrap_or(&serde_json::Value::Null);
                    let debt_gdp = fin.get("debt_to_gdp_pct").and_then(|d| d.as_f64()).unwrap_or(0.0);

                    format!(
                        "{}({},{}): GDP=${:.2}T(#{}) | 石油产{:.1}M桶/日 | 军费${:.0}B({:.1}%GDP) | 债务{:.0}%GDP | 信用:{:.1}",
                        code, name_zh, tier,
                        gdp / 1e12, gdp_rank,
                        oil_prod / 1e6,
                        mil_budget / 1e9, mil_pct,
                        debt_gdp, cred
                    )
                })
                .collect();
            lines.sort();
            lines.join("\n")
        }
        None => String::from("(country profiles data unavailable)"),
    }
});

/// Return a compact one-line-per-country summary of structural profiles.
///
/// Output is ~500 bytes vs the full 44 KB JSON -- suitable for token-constrained providers.
pub fn country_profiles_slim() -> &'static str {
    &COUNTRY_PROFILES_SLIM_CACHE
}

// ---------------------------------------------------------------------------
// Slim power-structures extract (~800 bytes vs 28 KB)
// ---------------------------------------------------------------------------

static POWER_STRUCTURES_SLIM_CACHE: LazyLock<String> = LazyLock::new(|| {
    let parsed: serde_json::Value =
        serde_json::from_str(POWER_STRUCTURES).unwrap_or_default();
    let chains = parsed.get("chains").and_then(|c| c.as_array());
    match chains {
        Some(arr) => {
            let lines: Vec<String> = arr
                .iter()
                .filter_map(|chain| {
                    let id = chain.get("id").and_then(|i| i.as_str()).unwrap_or("?");
                    let name = chain.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                    let desc = chain.get("description").and_then(|d| d.as_str()).unwrap_or("");
                    let desc_short: String = desc.chars().take(60).collect();
                    let path = chain.get("transmission_path").and_then(|p| p.as_str()).unwrap_or("");

                    // Collect key metrics from nodes
                    let metrics_summary: String = chain
                        .get("nodes")
                        .and_then(|n| n.as_array())
                        .map(|nodes| {
                            nodes.iter()
                                .filter_map(|node| {
                                    node.get("metrics").and_then(|m| m.as_object()).map(|m| {
                                        m.iter()
                                            .filter(|(k, _)| k.contains("pct") || k.contains("share"))
                                            .map(|(k, v)| {
                                                let val = v.as_f64()
                                                    .map(|f| format!("{:.1}%", f))
                                                    .unwrap_or_else(|| v.as_str().unwrap_or("?").to_string());
                                                format!("{}={}", k.replace("_pct", "%").replace("_share", ""), val)
                                            })
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    })
                                })
                                .filter(|s| !s.is_empty())
                                .collect::<Vec<_>>()
                                .join("; ")
                        })
                        .unwrap_or_default();

                    Some(format!(
                        "{}({}): {} | 传导:{} | {}",
                        id, name, desc_short, path, metrics_summary
                    ))
                })
                .collect();
            lines.join("\n")
        }
        None => String::from("(power structures data unavailable)"),
    }
});

/// Return a compact one-line-per-chain summary of structural causal chains.
///
/// Output is ~800 bytes vs the full 28 KB JSON -- suitable for token-constrained providers.
pub fn power_structures_slim() -> &'static str {
    &POWER_STRUCTURES_SLIM_CACHE
}

// ---------------------------------------------------------------------------
// Slim geopolitical-graph extract (~600 bytes vs 22 KB)
// ---------------------------------------------------------------------------

static GEOPOLITICAL_GRAPH_SLIM_CACHE: LazyLock<String> = LazyLock::new(|| {
    let parsed: serde_json::Value =
        serde_json::from_str(GEOPOLITICAL_GRAPH).unwrap_or_default();
    let edges = parsed.get("edges").and_then(|e| e.as_array());
    match edges {
        Some(arr) => {
            let lines: Vec<String> = arr
                .iter()
                .filter_map(|edge| {
                    let id = edge.get("id").and_then(|i| i.as_str()).unwrap_or("?");
                    let label = edge.get("label").and_then(|l| l.as_str()).unwrap_or("?");
                    let score = edge.get("cooperation_score").and_then(|s| s.as_f64()).unwrap_or(0.0);
                    let rel_type = edge.get("relationship_type").and_then(|r| r.as_str()).unwrap_or("?");

                    let channels: String = edge
                        .get("financial_transmission")
                        .and_then(|ft| ft.get("channels"))
                        .and_then(|c| c.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();

                    Some(format!(
                        "{}({}): score={:.1}, {} | 传导:{}",
                        id, label, score, rel_type, channels
                    ))
                })
                .collect();
            lines.join("\n")
        }
        None => String::from("(geopolitical graph data unavailable)"),
    }
});

/// Return a compact one-line-per-edge summary of geopolitical relationships.
///
/// Output is ~600 bytes vs the full 22 KB JSON -- suitable for token-constrained providers.
pub fn geopolitical_graph_slim() -> &'static str {
    &GEOPOLITICAL_GRAPH_SLIM_CACHE
}

// ---------------------------------------------------------------------------
// Slim event-triggers extract (~1 KB vs full JSON)
// ---------------------------------------------------------------------------

static EVENT_TRIGGERS_SLIM_CACHE: LazyLock<String> = LazyLock::new(|| {
    let parsed: serde_json::Value =
        serde_json::from_str(EVENT_TRIGGERS).unwrap_or_default();
    let events = parsed.get("events").and_then(|e| e.as_array());
    match events {
        Some(arr) => {
            let lines: Vec<String> = arr
                .iter()
                .filter_map(|evt| {
                    let id = evt.get("id").and_then(|i| i.as_str()).unwrap_or("?");
                    let name = evt.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                    let priority = evt.get("priority").and_then(|p| p.as_str()).unwrap_or("?");

                    let prob_label = evt
                        .get("probability")
                        .and_then(|p| p.get("label"))
                        .and_then(|l| l.as_str())
                        .unwrap_or("?");
                    let prob_window = evt
                        .get("probability")
                        .and_then(|p| p.get("window"))
                        .and_then(|w| w.as_str())
                        .unwrap_or("");

                    let chains: String = evt
                        .get("affected_chains")
                        .and_then(|c| c.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join("+")
                        })
                        .unwrap_or_default();

                    let transmission: String = evt
                        .get("transmission_summary")
                        .and_then(|t| t.as_str())
                        .map(|s| s.chars().take(80).collect())
                        .unwrap_or_default();

                    let impacts: String = evt
                        .get("market_impacts")
                        .and_then(|m| m.as_array())
                        .map(|arr| {
                            arr.iter()
                                .take(3)
                                .filter_map(|imp| {
                                    let asset = imp.get("asset").and_then(|a| a.as_str()).unwrap_or("?");
                                    let dir = imp.get("direction").and_then(|d| d.as_str()).unwrap_or("?");
                                    let mag = imp.get("magnitude").and_then(|m| m.as_str()).unwrap_or("?");
                                    Some(format!("{}:{}{}", asset, if dir == "bullish" { "↑" } else { "↓" }, mag))
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();

                    Some(format!(
                        "{}({},{}) {}({}) | chains:{} | {} | {}",
                        id, name, priority, prob_label, prob_window, chains, impacts, transmission
                    ))
                })
                .collect();
            lines.join("\n")
        }
        None => String::from("(event triggers data unavailable)"),
    }
});

/// Return a compact one-line-per-event summary of geopolitical event triggers.
///
/// Output is ~1.5 KB vs the full JSON -- suitable for token-constrained providers.
pub fn event_triggers_slim() -> &'static str {
    &EVENT_TRIGGERS_SLIM_CACHE
}

// ---------------------------------------------------------------------------
// Slim country-roles extract (~1 KB vs full JSON)
// ---------------------------------------------------------------------------

static COUNTRY_ROLES_SLIM_CACHE: LazyLock<String> = LazyLock::new(|| {
    let parsed: serde_json::Value =
        serde_json::from_str(COUNTRY_ROLES).unwrap_or_default();
    let countries = parsed.get("countries").and_then(|c| c.as_object());
    match countries {
        Some(map) => {
            let mut lines: Vec<String> = map
                .iter()
                .map(|(code, v)| {
                    let role_zh = v.get("role_zh").and_then(|r| r.as_str()).unwrap_or("?");
                    let transition = v.get("transition").and_then(|t| t.as_str()).unwrap_or("?");
                    let behavior: String = v
                        .get("behavior")
                        .and_then(|b| b.as_str())
                        .map(|s| s.chars().take(50).collect())
                        .unwrap_or_default();

                    let chains = v.get("chains").and_then(|c| c.as_object());
                    let chain_str = chains
                        .map(|m| {
                            m.iter()
                                .map(|(k, v)| {
                                    let pos = v.as_str().unwrap_or("?");
                                    let short = match pos {
                                        "controller" => "控",
                                        "producer" => "产",
                                        "gatekeeper" => "门",
                                        "consumer" => "消",
                                        "dependent" => "依",
                                        "builder" => "建",
                                        "follower" => "从",
                                        "neutral" => "中",
                                        _ => "?",
                                    };
                                    format!("{}:{}", k.chars().next().unwrap_or('?'), short)
                                })
                                .collect::<Vec<_>>()
                                .join(",")
                        })
                        .unwrap_or_default();

                    format!(
                        "{}({}): {} | 行为:{} | 链:[{}]",
                        code, role_zh, transition, behavior, chain_str
                    )
                })
                .collect();
            lines.sort();
            lines.join("\n")
        }
        None => String::from("(country roles data unavailable)"),
    }
});

/// Return a compact one-line-per-country summary of roles + chain positions.
///
/// Output is ~1.2 KB vs the full JSON -- suitable for token-constrained providers.
pub fn country_roles_slim() -> &'static str {
    &COUNTRY_ROLES_SLIM_CACHE
}
