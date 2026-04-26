use ctx_pack::{PackInput, build_pack};

fn rich_input(budget: usize) -> PackInput {
    PackInput {
        query: "fix refresh token failure".into(),
        error_root_cause: Some("ValueError: expired token at auth/service.py:44".into()),
        symbols: vec![
            "src/auth.rs::fn validate_refresh_token(token: &str) -> bool { very long body omitted }".into(),
        ],
        tests: vec!["tests/auth_test.rs::test_refresh_token_rotates".into()],
        recent_diff: Some(
            "diff --git a/src/auth.rs b/src/auth.rs\n@@ -1,1 +1,2 @@\n-fn old() {}\n+fn validate_refresh_token() {}\n+fn helper() {}".into(),
        ),
        dependencies: vec!["src/tokens.rs -> decode_token -> src/auth.rs".into()],
        task_memory: vec!["decision: preserve public auth API".into()],
        failure_memory: vec!["failure: expired token regression fixed by rotation".into()],
        memory: vec!["directive: always run auth tests".into()],
        docs: vec!["docs/auth.md: refresh token lifecycle, long secondary explanation".into()],
        budget,
    }
}

#[test]
fn packer_uses_strict_priority_order_and_explainable_reasons() {
    let packed = build_pack(&rich_input(58));
    let context = packed.compact_context;

    assert!(context.contains("query:"));
    assert!(context.contains("root_cause:"));
    assert!(context.contains("symbols:"));
    assert!(context.contains("tests:"));
    assert!(context.contains("recent_diff:"));
    assert!(context.contains("dependencies:"));
    assert!(context.contains("task_memory:"));
    assert!(context.contains("failure_memory:"));
    assert!(
        !context.contains("docs/auth.md"),
        "secondary docs should lose first under tight budget"
    );

    let query_pos = context.find("query:").unwrap();
    let root_pos = context.find("root_cause:").unwrap();
    let symbol_pos = context.find("symbols:").unwrap();
    let tests_pos = context.find("tests:").unwrap();
    assert!(query_pos < root_pos && root_pos < symbol_pos && symbol_pos < tests_pos);

    assert!(
        packed
            .included
            .iter()
            .any(|entry| entry.contains("query included"))
    );
    assert!(
        packed
            .excluded
            .iter()
            .any(|entry| entry.contains("docs excluded"))
    );
    assert!(packed.packed_tokens <= 58);
}

#[test]
fn rewriter_preserves_traceability_while_compressing_blocks() {
    let packed = build_pack(&rich_input(120));
    let context = packed.compact_context;

    assert!(context.contains("src/auth.rs"));
    assert!(context.contains("validate_refresh_token"));
    assert!(context.contains("lines:"));
    assert!(context.contains("relationships:"));
    assert!(context.contains("diff_files:"));
    assert!(context.contains("changed_symbols:"));
    assert!(context.contains("changes:"));
    assert!(context.len() < rich_input(120).recent_diff.unwrap().len() + 500);
}
