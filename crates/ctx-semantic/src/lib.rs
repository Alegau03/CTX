use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Features {
    pub semantic_similarity: f64,
    pub keyword_overlap: f64,
    pub recency: f64,
    pub graph_distance_bonus: f64,
    pub failure_bonus: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticBackendKind {
    LocalHash,
    Onnx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingConfig {
    pub backend: SemanticBackendKind,
    pub max_chunks: usize,
    pub adaptive_threshold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkCandidate {
    pub id: String,
    pub text: String,
    pub keyword_hint: String,
    pub recency: f64,
    pub graph_distance: f64,
    pub failure_relevance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedChunk {
    pub id: String,
    pub score: f64,
    pub features: Features,
    pub reason: String,
    pub text: String,
}

pub fn score(features: Features) -> f64 {
    0.40 * features.semantic_similarity
        + 0.20 * features.keyword_overlap
        + 0.15 * features.recency
        + 0.15 * features.graph_distance_bonus
        + 0.10 * features.failure_bonus
}

pub fn rank_chunks_hybrid(
    query: &str,
    candidates: &[ChunkCandidate],
    config: RankingConfig,
) -> Vec<RankedChunk> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let mut seen_fingerprint = HashSet::new();
    let mut ranked = Vec::new();

    for candidate in candidates {
        let fingerprint = normalize_text(&candidate.text);
        if !seen_fingerprint.insert(fingerprint) {
            continue;
        }

        let semantic_similarity = match config.backend {
            SemanticBackendKind::LocalHash | SemanticBackendKind::Onnx => {
                local_hash_similarity(query, &candidate.text)
            }
        };

        let keyword_overlap = jaccard_similarity(query, &candidate.keyword_hint);
        let features = Features {
            semantic_similarity,
            keyword_overlap,
            recency: candidate.recency.clamp(0.0, 1.0),
            graph_distance_bonus: graph_distance_bonus(candidate.graph_distance),
            failure_bonus: candidate.failure_relevance.clamp(0.0, 1.0),
        };

        let total_score = score(features);
        ranked.push(RankedChunk {
            id: candidate.id.clone(),
            score: total_score,
            features,
            reason: format!(
                "semantic={:.3} keyword={:.3} recency={:.3} graph={:.3} failure={:.3}",
                features.semantic_similarity,
                features.keyword_overlap,
                features.recency,
                features.graph_distance_bonus,
                features.failure_bonus
            ),
            text: candidate.text.clone(),
        });
    }

    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let thresholded = if config.adaptive_threshold && !ranked.is_empty() {
        let top = ranked[0].score;
        let threshold = (top * 0.35).max(0.15);
        let mut kept = ranked
            .into_iter()
            .filter(|entry| entry.score >= threshold)
            .collect::<Vec<_>>();

        if kept.len() < 2 && candidates.len() >= 2 {
            // Keep a second option to avoid over-pruning in exploratory queries.
            let mut second_pass = rank_without_threshold(query, candidates, config.backend);
            second_pass.truncate(2);
            kept = dedup_ranked(second_pass);
        }

        kept
    } else {
        ranked
    };

    let mut final_ranked = thresholded;
    final_ranked.truncate(config.max_chunks.max(1));
    final_ranked
}

fn rank_without_threshold(
    query: &str,
    candidates: &[ChunkCandidate],
    backend: SemanticBackendKind,
) -> Vec<RankedChunk> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for candidate in candidates {
        let fp = normalize_text(&candidate.text);
        if !seen.insert(fp) {
            continue;
        }

        let semantic_similarity = match backend {
            SemanticBackendKind::LocalHash | SemanticBackendKind::Onnx => {
                local_hash_similarity(query, &candidate.text)
            }
        };

        let features = Features {
            semantic_similarity,
            keyword_overlap: jaccard_similarity(query, &candidate.keyword_hint),
            recency: candidate.recency.clamp(0.0, 1.0),
            graph_distance_bonus: graph_distance_bonus(candidate.graph_distance),
            failure_bonus: candidate.failure_relevance.clamp(0.0, 1.0),
        };

        out.push(RankedChunk {
            id: candidate.id.clone(),
            score: score(features),
            features,
            reason: "fallback rank".to_string(),
            text: candidate.text.clone(),
        });
    }

    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

fn dedup_ranked(items: Vec<RankedChunk>) -> Vec<RankedChunk> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for item in items {
        let fp = normalize_text(&item.text);
        if seen.insert(fp) {
            out.push(item);
        }
    }

    out
}

fn local_hash_similarity(a: &str, b: &str) -> f64 {
    let va = hash_embedding(a);
    let vb = hash_embedding(b);
    cosine_similarity(&va, &vb)
}

fn hash_embedding(text: &str) -> HashMap<u64, f64> {
    let mut map = HashMap::new();
    for token in tokenize(text) {
        let hash = fxhash64(token.as_bytes());
        *map.entry(hash).or_insert(0.0) += 1.0;
    }
    map
}

fn cosine_similarity(a: &HashMap<u64, f64>, b: &HashMap<u64, f64>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0;
    for (key, va) in a {
        if let Some(vb) = b.get(key) {
            dot += va * vb;
        }
    }

    let norm_a = a.values().map(|v| v * v).sum::<f64>().sqrt();
    let norm_b = b.values().map(|v| v * v).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        (dot / (norm_a * norm_b)).clamp(0.0, 1.0)
    }
}

fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let sa = tokenize(a).into_iter().collect::<HashSet<_>>();
    let sb = tokenize(b).into_iter().collect::<HashSet<_>>();
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }

    let inter = sa.intersection(&sb).count() as f64;
    let union = sa.union(&sb).count() as f64;
    (inter / union).clamp(0.0, 1.0)
}

fn graph_distance_bonus(distance: f64) -> f64 {
    let d = distance.max(0.0);
    (1.0 / (1.0 + d)).clamp(0.0, 1.0)
}

fn normalize_text(text: &str) -> String {
    text.split_whitespace()
        .map(|s| s.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|part| part.len() > 1)
        .map(|part| part.to_lowercase())
        .collect()
}

fn fxhash64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_is_monotonic_for_semantic_similarity() {
        let low = score(Features {
            semantic_similarity: 0.1,
            keyword_overlap: 0.0,
            recency: 0.0,
            graph_distance_bonus: 0.0,
            failure_bonus: 0.0,
        });

        let high = score(Features {
            semantic_similarity: 0.9,
            keyword_overlap: 0.0,
            recency: 0.0,
            graph_distance_bonus: 0.0,
            failure_bonus: 0.0,
        });

        assert!(high > low);
    }
}
