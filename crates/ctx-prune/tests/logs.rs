use ctx_prune::prune_logs;

#[test]
fn parser_packs_keep_tool_specific_root_causes() {
    let input = r#"
PASS tests/test_auth.py::test_login
============================= FAILURES =============================
____ test_refresh_token ____
E   AssertionError: expected rotated token
FAILED tests/test_auth.py::test_refresh_token - AssertionError
src/app.ts(12,5): error TS2322: Type 'string' is not assignable to type 'number'.
/ctx/web/src/App.tsx
  7:9  error  'token' is assigned a value but never used  @typescript-eslint/no-unused-vars
1 problem (1 error, 0 warnings)
src/auth.py:8:1: F401 `os` imported but unused
src/auth.py:9: error: Incompatible return value type [return-value]
error[E0425]: cannot find value `token` in this scope
  --> src/lib.rs:10:5
--- FAIL: TestRefreshToken (0.00s)
    auth_test.go:42: expected rotated token
FAIL github.com/example/auth 0.12s
npm ERR! code ERESOLVE
npm ERR! ERESOLVE unable to resolve dependency tree
On branch main
modified:   src/auth.rs
added 812 packages in 12s
PASS tests/test_other.py::test_ok
"#;

    let report = prune_logs(input, 80);

    assert!(
        report
            .output
            .contains("FAILED tests/test_auth.py::test_refresh_token")
    );
    assert!(report.output.contains("src/app.ts(12,5): error TS2322"));
    assert!(report.output.contains("@typescript-eslint/no-unused-vars"));
    assert!(report.output.contains("src/auth.py:8:1: F401"));
    assert!(report.output.contains("src/auth.py:9: error:"));
    assert!(report.output.contains("error[E0425]"));
    assert!(report.output.contains("src/lib.rs:10:5"));
    assert!(report.output.contains("--- FAIL: TestRefreshToken"));
    assert!(report.output.contains("auth_test.go:42"));
    assert!(report.output.contains("npm ERR! ERESOLVE"));
    assert!(report.output.contains("modified:   src/auth.rs"));
    assert!(!report.output.contains("added 812 packages"));
    assert!(!report.output.contains("PASS tests/test_other"));
}

#[test]
fn parser_budget_prioritizes_root_causes_over_warnings() {
    let input = r#"
warning: unused import
warning: deprecated package
npm ERR! code ELIFECYCLE
error[E0308]: mismatched types
--- FAIL: TestPayment (0.00s)
FAIL github.com/example/payments 0.10s
"#;

    let report = prune_logs(input, 3);

    assert_eq!(report.kept_lines, 3);
    assert!(report.output.contains("npm ERR! code ELIFECYCLE"));
    assert!(report.output.contains("error[E0308]"));
    assert!(report.output.contains("--- FAIL: TestPayment"));
    assert!(!report.output.contains("warning: unused import"));
}
