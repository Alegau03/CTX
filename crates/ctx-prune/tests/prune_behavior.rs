use ctx_prune::{prune_diff, prune_logs};

#[test]
fn prune_logs_keeps_errors_and_deduplicates_success_noise() {
    let input = r#"
PASS test_a
PASS test_a
PASS test_b
ERROR failed to connect
Traceback: line 10
PASS test_c
"#;

    let report = prune_logs(input, 50);

    assert!(report.output.contains("ERROR failed to connect"));
    assert!(report.output.contains("Traceback: line 10"));
    assert!(!report.output.contains("PASS test_c"));
    assert!(report.excluded.iter().any(|x| x.contains("duplicate")));
}

#[test]
fn prune_logs_respects_line_budget() {
    let input = (0..20)
        .map(|i| format!("ERROR line {}", i))
        .collect::<Vec<_>>()
        .join("\n");

    let report = prune_logs(&input, 5);
    assert!(report.kept_lines <= 5);
}

#[test]
fn prune_diff_keeps_relevant_hunks_for_query() {
    let diff = r#"
diff --git a/src/auth.rs b/src/auth.rs
@@ -1,3 +1,3 @@
-fn old_auth() {}
+fn validate_refresh_token() {}

diff --git a/src/other.rs b/src/other.rs
@@ -1,3 +1,3 @@
-fn old() {}
+fn noop() {}
"#;

    let report = prune_diff(diff, "refresh token", 50);

    assert!(report.output.contains("validate_refresh_token"));
    assert!(!report.output.contains("noop"));
    assert!(report.included.iter().any(|x| x.contains("query match")));
}
