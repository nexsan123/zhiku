use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::intelligence::NewsCluster;

/// Minimum shared entities to cluster two news items.
const MIN_SHARED_ENTITIES: usize = 2;

/// Minimum cluster size (skip clusters with fewer items).
const MIN_CLUSTER_SIZE: usize = 3;

/// Maximum cluster size.
const MAX_CLUSTER_SIZE: usize = 20;

/// Time window for clustering (hours).
const CLUSTER_WINDOW_HOURS: i64 = 48;

/// Metadata extracted from ai_analysis for one news item.
struct NewsMeta {
    news_id: String,
    region: Vec<String>,
    entities: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build news clusters from recently analyzed news in the last 48h.
///
/// Reads region + entities from ai_analysis.reasoning_chain (JSON),
/// groups news by shared entities/regions, returns Vec<NewsCluster>.
pub async fn build_clusters(pool: &SqlitePool) -> Result<Vec<NewsCluster>, AppError> {
    let metas = fetch_news_meta(pool).await?;

    if metas.len() < MIN_CLUSTER_SIZE {
        return Ok(Vec::new());
    }

    let raw_clusters = cluster_by_similarity(&metas);

    let now = Utc::now().to_rfc3339();
    let clusters: Vec<NewsCluster> = raw_clusters
        .into_iter()
        .map(|indices| {
            let news_ids: Vec<String> = indices.iter().map(|&i| metas[i].news_id.clone()).collect();
            let mut all_regions: HashSet<String> = HashSet::new();
            let mut all_entities: HashSet<String> = HashSet::new();

            for &i in &indices {
                for r in &metas[i].region {
                    all_regions.insert(r.clone());
                }
                for e in &metas[i].entities {
                    all_entities.insert(e.clone());
                }
            }

            let regions: Vec<String> = all_regions.into_iter().collect();
            let entities: Vec<String> = all_entities.into_iter().collect();
            let topic_hint = generate_topic_hint(&regions, &entities);

            NewsCluster {
                cluster_id: Uuid::new_v4().to_string(),
                topic_hint,
                news_count: news_ids.len(),
                news_ids,
                regions,
                entities,
                created_at: now.clone(),
            }
        })
        .collect();

    log::info!(
        "News clustering: {} items → {} clusters",
        metas.len(),
        clusters.len()
    );

    Ok(clusters)
}

// ---------------------------------------------------------------------------
// Data fetching
// ---------------------------------------------------------------------------

/// Fetch news metadata (region + entities) from ai_analysis table.
async fn fetch_news_meta(pool: &SqlitePool) -> Result<Vec<NewsMeta>, AppError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        &format!(
            r#"SELECT input_ids, reasoning_chain FROM ai_analysis
               WHERE analysis_type = 'news_summary'
                 AND created_at >= datetime('now', '-{} hours')
                 AND reasoning_chain IS NOT NULL
               ORDER BY created_at DESC"#,
            CLUSTER_WINDOW_HOURS
        ),
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Fetch news meta failed: {}", e)))?;

    let mut metas = Vec::with_capacity(rows.len());

    for (news_id, reasoning_chain) in rows {
        // Parse the JSON metadata stored in reasoning_chain
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&reasoning_chain) {
            let region: Vec<String> = parsed
                .get("region")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                        .collect()
                })
                .unwrap_or_default();

            let entities: Vec<String> = parsed
                .get("entities")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                        .collect()
                })
                .unwrap_or_default();

            if !region.is_empty() || !entities.is_empty() {
                metas.push(NewsMeta {
                    news_id,
                    region,
                    entities,
                });
            }
        }
    }

    Ok(metas)
}

// ---------------------------------------------------------------------------
// Clustering algorithm
// ---------------------------------------------------------------------------

/// Simple entity/region overlap clustering.
///
/// Two news items are "similar" if they share:
/// - >= 2 entities, OR
/// - >= 1 region + >= 1 entity
///
/// Uses union-find for transitive clustering.
fn cluster_by_similarity(metas: &[NewsMeta]) -> Vec<Vec<usize>> {
    let n = metas.len();
    let mut parent: Vec<usize> = (0..n).collect();

    // Union-find helpers
    fn find(parent: &mut [usize], x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }

    fn union(parent: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[ra] = rb;
        }
    }

    // Compare all pairs
    for i in 0..n {
        for j in (i + 1)..n {
            if are_similar(&metas[i], &metas[j]) {
                union(&mut parent, i, j);
            }
        }
    }

    // Group by root
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        groups.entry(root).or_default().push(i);
    }

    // Filter by size
    groups
        .into_values()
        .filter(|g| g.len() >= MIN_CLUSTER_SIZE && g.len() <= MAX_CLUSTER_SIZE)
        .collect()
}

/// Check if two news items are similar enough to cluster.
fn are_similar(a: &NewsMeta, b: &NewsMeta) -> bool {
    let shared_entities = a
        .entities
        .iter()
        .filter(|e| b.entities.contains(e))
        .count();

    if shared_entities >= MIN_SHARED_ENTITIES {
        return true;
    }

    // 1 shared region + 1 shared entity
    let shared_regions = a.region.iter().filter(|r| b.region.contains(r)).count();
    if shared_regions >= 1 && shared_entities >= 1 {
        return true;
    }

    false
}

// ---------------------------------------------------------------------------
// Topic hint generation
// ---------------------------------------------------------------------------

/// Generate a human-readable topic hint from cluster regions and entities.
fn generate_topic_hint(regions: &[String], entities: &[String]) -> String {
    let top_entities: Vec<&str> = entities.iter().take(3).map(|s| s.as_str()).collect();
    let top_regions: Vec<&str> = regions.iter().take(2).map(|s| s.as_str()).collect();

    if !top_entities.is_empty() && !top_regions.is_empty() {
        format!("{} — {}", top_regions.join("/"), top_entities.join(", "))
    } else if !top_entities.is_empty() {
        top_entities.join(", ")
    } else {
        top_regions.join("/")
    }
}
