use ctx_semantic::{ChunkCandidate, RankingConfig, SemanticBackendKind, rank_chunks_hybrid};

#[test]
fn hybrid_ranking_prioritizes_query_relevant_chunk() {
    let candidates = vec![
        ChunkCandidate {
            id: "a".to_string(),
            text: "validate_refresh_token handles refresh validation".to_string(),
            keyword_hint: "auth refresh token".to_string(),
            recency: 0.9,
            graph_distance: 1.0,
            failure_relevance: 1.0,
        },
        ChunkCandidate {
            id: "b".to_string(),
            text: "render navbar and footer styles".to_string(),
            keyword_hint: "ui css".to_string(),
            recency: 0.2,
            graph_distance: 4.0,
            failure_relevance: 0.0,
        },
    ];

    let ranked = rank_chunks_hybrid(
        "fix refresh token auth failure",
        &candidates,
        RankingConfig {
            backend: SemanticBackendKind::LocalHash,
            max_chunks: 8,
            adaptive_threshold: true,
        },
    );

    assert!(!ranked.is_empty());
    assert_eq!(ranked[0].id, "a");
    assert!(ranked[0].score > ranked[1].score);
}

#[test]
fn deduplication_removes_near_identical_chunks() {
    let candidates = vec![
        ChunkCandidate {
            id: "x1".to_string(),
            text: "decode token and validate audience".to_string(),
            keyword_hint: "token decode".to_string(),
            recency: 0.8,
            graph_distance: 1.0,
            failure_relevance: 1.0,
        },
        ChunkCandidate {
            id: "x2".to_string(),
            text: "decode token and validate audience".to_string(),
            keyword_hint: "token decode".to_string(),
            recency: 0.7,
            graph_distance: 1.2,
            failure_relevance: 1.0,
        },
    ];

    let ranked = rank_chunks_hybrid(
        "decode token",
        &candidates,
        RankingConfig {
            backend: SemanticBackendKind::LocalHash,
            max_chunks: 8,
            adaptive_threshold: false,
        },
    );

    assert_eq!(ranked.len(), 1);
}
