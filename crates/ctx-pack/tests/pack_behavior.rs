use ctx_pack::{PackInput, build_pack};

#[test]
fn pack_prioritizes_query_and_root_cause() {
    let input = PackInput {
        query: "fix auth failure".into(),
        error_root_cause: Some("Traceback: decode token failed".into()),
        symbols: vec!["src/auth.rs::decode_token".into()],
        tests: vec!["tests/test_auth.rs::test_refresh_expired_token".into()],
        recent_diff: Some("+fn decode_token()".into()),
        dependencies: vec!["src/api/routes/auth.rs".into()],
        task_memory: vec!["decision: keep signature".into()],
        failure_memory: vec!["failure: token decode regression".into()],
        memory: vec!["decision: keep signature".into()],
        docs: vec!["docs/auth.md".into()],
        budget: 10,
    };

    let packed = build_pack(&input);
    assert!(packed.compact_context.contains("query:"));
    assert!(packed.compact_context.contains("root_cause:"));
}

#[test]
fn pack_emits_included_and_excluded_sections() {
    let input = PackInput {
        query: "review diff".into(),
        error_root_cause: None,
        symbols: vec!["a".into(), "b".into(), "c".into()],
        tests: vec![],
        recent_diff: None,
        dependencies: vec![],
        task_memory: vec![],
        failure_memory: vec![],
        memory: vec![],
        docs: vec!["long docs".into()],
        budget: 5,
    };

    let packed = build_pack(&input);
    assert!(!packed.included.is_empty());
    assert!(!packed.excluded.is_empty());
    assert!(packed.packed_tokens <= 5);
}
