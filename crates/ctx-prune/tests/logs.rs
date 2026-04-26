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

#[test]
fn parser_packs_cover_vitest_jest_and_alt_package_managers() {
    let input = r#"
 RUN  v1.6.0 /repo
 ❯ src/auth.test.ts (3)
   × refresh token rotates 12ms
     → expected rotated token

 FAIL  src/session.test.ts
  ● session refresh

    expect(received).toBeTruthy()

    Received: false

yarn error v1.22.19
yarn error An unexpected error occurred: "https://registry.yarnpkg.com/foo: Not found".
pnpm ERR!  ERR_PNPM_FETCH_404  GET https://registry.npmjs.org/foo: Not Found - 404
bun install v1.1.0
error: GET https://registry.npmjs.org/foo - 404
"#;

    let report = prune_logs(input, 50);

    assert!(report.output.contains("× refresh token rotates 12ms"));
    assert!(report.output.contains("expected rotated token"));
    assert!(report.output.contains("FAIL  src/session.test.ts"));
    assert!(report.output.contains("expect(received).toBeTruthy()"));
    assert!(report.output.contains("Received: false"));
    assert!(report.output.contains("yarn error"));
    assert!(report.output.contains("ERR_PNPM_FETCH_404"));
    assert!(
        report
            .output
            .contains("error: GET https://registry.npmjs.org/foo - 404")
    );
    assert!(!report.output.contains("RUN  v1.6.0 /repo"));
    assert!(!report.output.contains("bun install v1.1.0"));
}
