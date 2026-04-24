use ctx_semantic::{
    ChunkCandidate, EmbeddingCache, SemanticBackendKind, SemanticEngineConfig, rank_chunks,
};
use tempfile::tempdir;

fn candidates() -> Vec<ChunkCandidate> {
    vec![
        ChunkCandidate {
            id: "auth".to_string(),
            text: "validate refresh token and rotate expired credentials".to_string(),
            keyword_hint: "auth refresh token".to_string(),
            recency: 0.8,
            graph_distance: 1.0,
            failure_relevance: 1.0,
        },
        ChunkCandidate {
            id: "ui".to_string(),
            text: "render navbar layout and footer styles".to_string(),
            keyword_hint: "ui css".to_string(),
            recency: 0.2,
            graph_distance: 5.0,
            failure_relevance: 0.0,
        },
    ]
}

#[test]
fn strict_onnx_missing_model_returns_actionable_error() {
    let tmp = tempdir().expect("tempdir");
    let model = tmp.path().join("missing.onnx");
    let vocab = tmp.path().join("vocab.txt");

    let err = rank_chunks(
        "fix refresh token",
        &candidates(),
        SemanticEngineConfig {
            backend: SemanticBackendKind::Onnx,
            model_path: Some(model),
            vocab_path: Some(vocab),
            max_chunks: 4,
            adaptive_threshold: true,
            allow_fallback: false,
        },
    )
    .expect_err("strict ONNX must fail if local model files are missing");

    let message = err.to_string();
    assert!(message.contains("ONNX model not found"));
    assert!(message.contains("semantic.model"));
}

#[test]
fn onnx_missing_model_can_fallback_with_explicit_reason() {
    let tmp = tempdir().expect("tempdir");
    let ranked = rank_chunks(
        "fix refresh token",
        &candidates(),
        SemanticEngineConfig {
            backend: SemanticBackendKind::Onnx,
            model_path: Some(tmp.path().join("missing.onnx")),
            vocab_path: Some(tmp.path().join("vocab.txt")),
            max_chunks: 4,
            adaptive_threshold: true,
            allow_fallback: true,
        },
    )
    .expect("fallback should keep retrieval usable");

    assert_eq!(ranked[0].id, "auth");
    assert!(ranked[0].reason.contains("backend=local_hash"));
    assert!(ranked[0].reason.contains("fallback_from=onnx"));
}

#[test]
fn embedding_cache_invalidates_by_model_identity_and_text_hash() {
    let mut cache = EmbeddingCache::new(2);
    cache.put("model-a", "hello auth", vec![1.0, 0.0, 0.5]);

    assert_eq!(
        cache.get("model-a", "hello auth").expect("cached vector"),
        &[1.0, 0.0, 0.5]
    );
    assert!(cache.get("model-b", "hello auth").is_none());
    assert!(cache.get("model-a", "hello payments").is_none());
}
