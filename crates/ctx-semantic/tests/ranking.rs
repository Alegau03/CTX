use ctx_semantic::{Features, score};

#[test]
fn score_uses_pdf_weight_formula() {
    let features = Features {
        semantic_similarity: 1.0,
        keyword_overlap: 0.5,
        recency: 0.4,
        graph_distance_bonus: 0.2,
        failure_bonus: 0.1,
    };

    let s = score(features);
    let expected = 0.40 * 1.0 + 0.20 * 0.5 + 0.15 * 0.4 + 0.15 * 0.2 + 0.10 * 0.1;
    assert!((s - expected).abs() < 1e-6);
}
