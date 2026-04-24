use ctx_prune::prune_diff;

#[test]
fn diff_parser_keeps_only_query_matching_hunks_with_headers() {
    let diff = r#"
diff --git a/src/auth.rs b/src/auth.rs
index 111..222 100644
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,4 +1,4 @@
-fn old_auth() {}
+fn validate_refresh_token() {}
@@ -20,4 +20,4 @@
-fn old_noise() {}
+fn render_button() {}
diff --git a/src/payments.rs b/src/payments.rs
index 333..444 100644
--- a/src/payments.rs
+++ b/src/payments.rs
@@ -1,4 +1,4 @@
-fn old_payment() {}
+fn capture_payment() {}
"#;

    let report = prune_diff(diff, "refresh token", 80);

    assert!(
        report
            .output
            .contains("diff --git a/src/auth.rs b/src/auth.rs")
    );
    assert!(report.output.contains("--- a/src/auth.rs"));
    assert!(report.output.contains("+++ b/src/auth.rs"));
    assert!(report.output.contains("validate_refresh_token"));
    assert!(!report.output.contains("render_button"));
    assert!(!report.output.contains("capture_payment"));
    assert!(
        report
            .included
            .iter()
            .any(|line| line.contains("query match"))
    );
}

#[test]
fn diff_parser_empty_query_keeps_hunks_until_budget() {
    let diff = r#"
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,1 +1,1 @@
-old
+new
diff --git a/src/b.rs b/src/b.rs
--- a/src/b.rs
+++ b/src/b.rs
@@ -1,1 +1,1 @@
-old_b
+new_b
"#;

    let report = prune_diff(diff, "", 20);

    assert!(report.output.contains("src/a.rs"));
    assert!(report.output.contains("src/b.rs"));
    assert!(report.kept_lines <= 20);
}
