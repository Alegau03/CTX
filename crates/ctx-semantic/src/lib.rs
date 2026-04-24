use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SemanticError {
    #[error(
        "ONNX model not found at {path}; set [semantic].model to a local .onnx file or enable allow_fallback"
    )]
    OnnxModelNotFound { path: String },
    #[error(
        "ONNX vocab not found at {path}; set [semantic].vocab to a local vocab.txt/tokenizer file or enable allow_fallback"
    )]
    OnnxVocabNotFound { path: String },
    #[error(
        "ONNX backend requested but ctx-semantic was built without the `onnx` feature; rebuild with `cargo build --features ctx-semantic/onnx` or enable semantic.allow_fallback"
    )]
    OnnxFeatureDisabled,
    #[error("ONNX inference failed: {0}")]
    OnnxInference(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Features {
    pub semantic_similarity: f64,
    pub keyword_overlap: f64,
    pub recency: f64,
    pub graph_distance_bonus: f64,
    pub failure_bonus: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticBackendKind {
    LocalHash,
    Onnx,
}

impl SemanticBackendKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "local" | "local_hash" | "hash" => Some(Self::LocalHash),
            "onnx" | "onnx_runtime" => Some(Self::Onnx),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingConfig {
    pub backend: SemanticBackendKind,
    pub max_chunks: usize,
    pub adaptive_threshold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEngineConfig {
    pub backend: SemanticBackendKind,
    pub model_path: Option<PathBuf>,
    pub vocab_path: Option<PathBuf>,
    pub max_chunks: usize,
    pub adaptive_threshold: bool,
    pub allow_fallback: bool,
}

impl SemanticEngineConfig {
    pub fn local_hash(max_chunks: usize, adaptive_threshold: bool) -> Self {
        Self {
            backend: SemanticBackendKind::LocalHash,
            model_path: None,
            vocab_path: None,
            max_chunks,
            adaptive_threshold,
            allow_fallback: true,
        }
    }

    fn from_ranking_config(config: RankingConfig) -> Self {
        Self {
            backend: config.backend,
            model_path: None,
            vocab_path: None,
            max_chunks: config.max_chunks,
            adaptive_threshold: config.adaptive_threshold,
            allow_fallback: true,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddingMetadata {
    pub model_id: String,
    pub text_hash: u64,
    pub dimensions: usize,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    metadata: EmbeddingMetadata,
    vector: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct EmbeddingCache {
    capacity: usize,
    order: VecDeque<String>,
    entries: HashMap<String, CacheEntry>,
}

impl EmbeddingCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            order: VecDeque::new(),
            entries: HashMap::new(),
        }
    }

    pub fn put(&mut self, model_id: &str, text: &str, vector: Vec<f32>) -> EmbeddingMetadata {
        let metadata = EmbeddingMetadata {
            model_id: model_id.to_string(),
            text_hash: stable_text_hash(text),
            dimensions: vector.len(),
        };
        let key = cache_key(&metadata.model_id, metadata.text_hash);

        if !self.entries.contains_key(&key) {
            self.order.push_back(key.clone());
        }
        self.entries.insert(
            key.clone(),
            CacheEntry {
                metadata: metadata.clone(),
                vector,
            },
        );
        self.evict_if_needed();
        metadata
    }

    pub fn get(&self, model_id: &str, text: &str) -> Option<&[f32]> {
        let key = cache_key(model_id, stable_text_hash(text));
        self.entries.get(&key).map(|entry| entry.vector.as_slice())
    }

    pub fn metadata(&self, model_id: &str, text: &str) -> Option<&EmbeddingMetadata> {
        let key = cache_key(model_id, stable_text_hash(text));
        self.entries.get(&key).map(|entry| &entry.metadata)
    }

    fn evict_if_needed(&mut self) {
        while self.entries.len() > self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            } else {
                break;
            }
        }
    }
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
    rank_chunks(
        query,
        candidates,
        SemanticEngineConfig::from_ranking_config(config),
    )
    .unwrap_or_default()
}

pub fn rank_chunks(
    query: &str,
    candidates: &[ChunkCandidate],
    config: SemanticEngineConfig,
) -> Result<Vec<RankedChunk>, SemanticError> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let backend = resolve_backend(&config)?;
    let mut cache = EmbeddingCache::new(candidates.len() + 1);
    let query_embedding = embed_text(query, &backend, &mut cache)?;
    let mut seen_fingerprint = HashSet::new();
    let mut ranked = Vec::new();

    for candidate in candidates {
        let fingerprint = normalize_text(&candidate.text);
        if !seen_fingerprint.insert(fingerprint) {
            continue;
        }

        let candidate_embedding = embed_text(&candidate.text, &backend, &mut cache)?;
        let semantic_similarity = cosine_dense(&query_embedding, &candidate_embedding);
        let keyword_overlap = keyword_similarity(query, &candidate.keyword_hint, &candidate.text);
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
            reason: format_reason(&backend, features),
            text: candidate.text.clone(),
        });
    }

    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let thresholded = if config.adaptive_threshold && !ranked.is_empty() {
        apply_adaptive_threshold(ranked, config.max_chunks.max(1))
    } else {
        ranked
    };

    let mut final_ranked = thresholded;
    final_ranked.truncate(config.max_chunks.max(1));
    Ok(final_ranked)
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum ResolvedBackend {
    LocalHash {
        model_id: String,
        fallback_from: Option<&'static str>,
    },
    Onnx {
        model_id: String,
        model_path: PathBuf,
        vocab_path: Option<PathBuf>,
    },
}

fn resolve_backend(config: &SemanticEngineConfig) -> Result<ResolvedBackend, SemanticError> {
    match config.backend {
        SemanticBackendKind::LocalHash => Ok(ResolvedBackend::LocalHash {
            model_id: "local_hash:v1".to_string(),
            fallback_from: None,
        }),
        SemanticBackendKind::Onnx => resolve_onnx_backend(config),
    }
}

fn resolve_onnx_backend(config: &SemanticEngineConfig) -> Result<ResolvedBackend, SemanticError> {
    let Some(model_path) = config.model_path.clone() else {
        return fallback_or_error(
            config.allow_fallback,
            SemanticError::OnnxModelNotFound {
                path: "semantic.model".to_string(),
            },
        );
    };

    if !model_path.exists() {
        return fallback_or_error(
            config.allow_fallback,
            SemanticError::OnnxModelNotFound {
                path: format!("{} (semantic.model)", model_path.display()),
            },
        );
    }

    if let Some(vocab_path) = &config.vocab_path {
        if !vocab_path.exists() {
            return fallback_or_error(
                config.allow_fallback,
                SemanticError::OnnxVocabNotFound {
                    path: format!("{} (semantic.vocab)", vocab_path.display()),
                },
            );
        }
    }

    #[cfg(not(feature = "onnx"))]
    {
        fallback_or_error(config.allow_fallback, SemanticError::OnnxFeatureDisabled)
    }

    #[cfg(feature = "onnx")]
    {
        Ok(ResolvedBackend::Onnx {
            model_id: format!("onnx:{}", model_path.display()),
            model_path,
            vocab_path: config.vocab_path.clone(),
        })
    }
}

fn fallback_or_error(
    allow_fallback: bool,
    error: SemanticError,
) -> Result<ResolvedBackend, SemanticError> {
    if allow_fallback {
        Ok(ResolvedBackend::LocalHash {
            model_id: "local_hash:v1".to_string(),
            fallback_from: Some("onnx"),
        })
    } else {
        Err(error)
    }
}

fn embed_text(
    text: &str,
    backend: &ResolvedBackend,
    cache: &mut EmbeddingCache,
) -> Result<Vec<f32>, SemanticError> {
    let model_id = backend.model_id();
    if let Some(cached) = cache.get(model_id, text) {
        return Ok(cached.to_vec());
    }

    let vector = match backend {
        ResolvedBackend::LocalHash { .. } => local_hash_embedding(text),
        ResolvedBackend::Onnx {
            model_path,
            vocab_path,
            ..
        } => onnx_embedding(model_path, vocab_path.as_ref(), text)?,
    };
    cache.put(model_id, text, vector.clone());
    Ok(vector)
}

impl ResolvedBackend {
    fn model_id(&self) -> &str {
        match self {
            Self::LocalHash { model_id, .. } | Self::Onnx { model_id, .. } => model_id,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::LocalHash { .. } => "local_hash",
            Self::Onnx { .. } => "onnx",
        }
    }

    fn fallback_from(&self) -> Option<&'static str> {
        match self {
            Self::LocalHash { fallback_from, .. } => *fallback_from,
            Self::Onnx { .. } => None,
        }
    }
}

fn format_reason(backend: &ResolvedBackend, features: Features) -> String {
    let mut reason = format!(
        "backend={} semantic={:.3} keyword={:.3} recency={:.3} graph={:.3} failure={:.3}",
        backend.label(),
        features.semantic_similarity,
        features.keyword_overlap,
        features.recency,
        features.graph_distance_bonus,
        features.failure_bonus
    );
    if let Some(source) = backend.fallback_from() {
        reason.push_str(&format!(" fallback_from={source}"));
    }
    reason
}

fn apply_adaptive_threshold(ranked: Vec<RankedChunk>, max_chunks: usize) -> Vec<RankedChunk> {
    let top = ranked[0].score;
    let threshold = (top * 0.35).max(0.15);
    let mut kept = ranked
        .iter()
        .filter(|entry| entry.score >= threshold)
        .cloned()
        .collect::<Vec<_>>();

    if kept.len() < 2 && ranked.len() >= 2 && max_chunks >= 2 {
        kept = ranked.iter().take(2).cloned().collect();
    }

    kept
}

fn local_hash_embedding(text: &str) -> Vec<f32> {
    const DIMS: usize = 256;
    let mut vector = vec![0.0f32; DIMS];
    for token in tokenize(text) {
        let hash = stable_text_hash(&token);
        let idx = (hash as usize) % DIMS;
        vector[idx] += 1.0;

        let chars = token.chars().collect::<Vec<_>>();
        for window in chars.windows(3) {
            let gram = window.iter().collect::<String>();
            let gram_idx = (stable_text_hash(&gram) as usize) % DIMS;
            vector[gram_idx] += 0.25;
        }
    }
    normalize_dense(vector)
}

#[cfg(feature = "onnx")]
fn onnx_embedding(
    model_path: &std::path::Path,
    vocab_path: Option<&PathBuf>,
    text: &str,
) -> Result<Vec<f32>, SemanticError> {
    use tract_onnx::prelude::*;

    let tokens = if let Some(vocab_path) = vocab_path {
        wordpiece_token_ids(vocab_path, text)?
    } else {
        hashed_token_ids(text)
    };
    let seq_len = tokens.len().max(1);
    let attention = vec![1_i64; seq_len];
    let token_types = vec![0_i64; seq_len];

    let model = tract_onnx::onnx()
        .model_for_path(model_path)
        .map_err(|err| SemanticError::OnnxInference(err.to_string()))?
        .into_optimized()
        .map_err(|err| SemanticError::OnnxInference(err.to_string()))?;
    let input_count = model
        .input_outlets()
        .map_err(|err| SemanticError::OnnxInference(err.to_string()))?
        .len();
    if input_count == 0 || input_count > 3 {
        return Err(SemanticError::OnnxInference(format!(
            "expected ONNX text embedding model with 1-3 inputs, got {input_count}"
        )));
    }

    let model = model
        .into_runnable()
        .map_err(|err| SemanticError::OnnxInference(err.to_string()))?;

    let mut inputs = TVec::new();
    for values in [&tokens, &attention, &token_types]
        .into_iter()
        .take(input_count)
    {
        inputs.push(
            Tensor::from_shape(&[1, seq_len], values)
                .map_err(|err| SemanticError::OnnxInference(err.to_string()))?
                .into(),
        );
    }

    let outputs = model
        .run(inputs)
        .map_err(|err| SemanticError::OnnxInference(err.to_string()))?;
    let first = outputs
        .first()
        .ok_or_else(|| SemanticError::OnnxInference("model returned no outputs".to_string()))?;
    let view = first
        .to_array_view::<f32>()
        .map_err(|err| SemanticError::OnnxInference(err.to_string()))?;
    let shape = view.shape();

    let vector = match shape.len() {
        2 => view.iter().copied().collect::<Vec<_>>(),
        3 => {
            let dim = shape[2];
            let mut pooled = vec![0.0f32; dim];
            for token_idx in 0..shape[1] {
                for dim_idx in 0..dim {
                    pooled[dim_idx] += view[[0, token_idx, dim_idx]];
                }
            }
            for value in &mut pooled {
                *value /= shape[1].max(1) as f32;
            }
            pooled
        }
        _ => view.iter().copied().collect::<Vec<_>>(),
    };

    Ok(normalize_dense(vector))
}

#[cfg(not(feature = "onnx"))]
fn onnx_embedding(
    _model_path: &std::path::Path,
    _vocab_path: Option<&PathBuf>,
    _text: &str,
) -> Result<Vec<f32>, SemanticError> {
    Err(SemanticError::OnnxFeatureDisabled)
}

#[cfg(feature = "onnx")]
fn wordpiece_token_ids(vocab_path: &PathBuf, text: &str) -> Result<Vec<i64>, SemanticError> {
    let vocab = std::fs::read_to_string(vocab_path)
        .map_err(|err| SemanticError::OnnxInference(err.to_string()))?;
    let mut ids = HashMap::new();
    for (idx, token) in vocab.lines().enumerate() {
        ids.insert(token.trim().to_string(), idx as i64);
    }

    let cls = *ids.get("[CLS]").unwrap_or(&101);
    let sep = *ids.get("[SEP]").unwrap_or(&102);
    let unk = *ids.get("[UNK]").unwrap_or(&100);
    let mut out = vec![cls];
    for token in tokenize(text).into_iter().take(254) {
        out.push(*ids.get(&token).unwrap_or(&unk));
    }
    out.push(sep);
    Ok(out)
}

#[cfg(feature = "onnx")]
fn hashed_token_ids(text: &str) -> Vec<i64> {
    let mut out = vec![101_i64];
    out.extend(
        tokenize(text)
            .into_iter()
            .take(254)
            .map(|token| ((stable_text_hash(&token) % 30_000) + 1_000) as i64),
    );
    out.push(102_i64);
    out
}

fn keyword_similarity(query: &str, hint: &str, text: &str) -> f64 {
    let hinted = if hint.trim().is_empty() {
        text.to_string()
    } else {
        format!("{hint} {text}")
    };
    jaccard_similarity(query, &hinted)
}

fn cosine_dense(a: &[f32], b: &[f32]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let len = a.len().min(b.len());
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for idx in 0..len {
        let va = a[idx] as f64;
        let vb = b[idx] as f64;
        dot += va * vb;
        norm_a += va * va;
        norm_b += vb * vb;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        (dot / (norm_a.sqrt() * norm_b.sqrt())).clamp(0.0, 1.0)
    }
}

fn normalize_dense(mut vector: Vec<f32>) -> Vec<f32> {
    let norm = vector
        .iter()
        .map(|value| (*value as f64) * (*value as f64))
        .sum::<f64>()
        .sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value = (*value as f64 / norm) as f32;
        }
    }
    vector
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

fn cache_key(model_id: &str, text_hash: u64) -> String {
    format!("{model_id}:{text_hash:016x}")
}

fn stable_text_hash(text: &str) -> u64 {
    fxhash64(normalize_text(text).as_bytes())
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
