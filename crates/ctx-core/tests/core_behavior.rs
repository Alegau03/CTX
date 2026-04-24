use std::fs;
use std::process::Command;

use ctx_core::{init_repo, run_graph_query, run_index, run_pack};
use ctx_graph::GraphStore;
use tempfile::tempdir;

#[test]
fn init_repo_creates_config_and_graph_db() {
    let tmp = tempdir().expect("tempdir");
    let config_path = init_repo(tmp.path()).expect("init");

    assert!(config_path.exists());
    assert!(tmp.path().join(".ctx/graph.db").exists());
}

#[test]
fn index_and_graph_query_find_code_files() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(
        tmp.path().join("src/auth.rs"),
        "fn validate_refresh_token() {}\n",
    )
    .expect("write");

    let count = run_index(tmp.path(), &[]).expect("index");
    assert!(count >= 1);

    let matches = run_graph_query(tmp.path(), "auth").expect("query");
    assert!(matches.iter().any(|m| m.ends_with("src/auth.rs")));
}

#[test]
fn run_pack_returns_compact_context() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let attach = tmp.path().join("failure.txt");
    fs::write(&attach, "ERROR token decode failed\nTraceback line 2").expect("write");

    let result = run_pack(tmp.path(), "fix auth", Some(100), Some(&attach)).expect("pack");
    assert!(result.compact_context.contains("query:"));
    assert!(result.compact_context.contains("root_cause:"));
}

#[test]
fn run_pack_blocks_sensitive_attachment_by_default() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let attach = tmp.path().join(".env");
    fs::write(&attach, "API_KEY=secret").expect("write");

    let result = run_pack(tmp.path(), "fix auth", Some(100), Some(&attach));
    assert!(result.is_err());
}

#[test]
fn run_pack_audits_blocked_sensitive_attachment() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let attach = tmp.path().join(".env");
    fs::write(&attach, "API_KEY=secret").expect("write");

    let result = run_pack(tmp.path(), "fix auth", Some(100), Some(&attach));
    assert!(result.is_err());

    let audit = fs::read_to_string(tmp.path().join(".ctx/audit.log")).expect("audit readable");
    assert!(audit.contains("privacy_decision"));
    assert!(audit.contains("\"decision\":\"excluded\""));
    assert!(audit.contains("\"reason\":\"sensitive_pattern\""));
    assert!(audit.contains(".env"));
}

#[test]
fn run_index_skips_sensitive_code_files_and_audits_decision() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(tmp.path().join("src/auth.rs"), "fn validate_token() {}\n").expect("write auth");
    fs::write(
        tmp.path().join("src/secret_tokens.rs"),
        "fn leaked_token_fixture() {}\n",
    )
    .expect("write secret");

    let count = run_index(tmp.path(), &[]).expect("index");
    assert_eq!(count, 1);

    let matches = run_graph_query(tmp.path(), "leaked").expect("query");
    assert!(matches.is_empty());

    let audit = fs::read_to_string(tmp.path().join(".ctx/audit.log")).expect("audit readable");
    assert!(audit.contains("privacy_decision"));
    assert!(audit.contains("src/secret_tokens.rs"));
    assert!(audit.contains("\"decision\":\"excluded\""));
    assert!(audit.contains("\"reason\":\"sensitive_pattern\""));
}

#[test]
fn run_pack_appends_audit_log_entry() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let attach = tmp.path().join("failure.txt");
    fs::write(&attach, "ERROR token decode failed").expect("write");

    run_pack(tmp.path(), "fix auth", Some(100), Some(&attach)).expect("pack");
    let audit = fs::read_to_string(tmp.path().join(".ctx/audit.log")).expect("audit readable");

    assert!(audit.contains("run_pack"));
    assert!(audit.contains("query=\"fix auth\""));
}

#[test]
fn run_agent_invocation_records_fallback_metadata_when_binary_missing() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    let report = ctx_core::run_agent_invocation(
        tmp.path(),
        ctx_adapters::Agent::Claude,
        "explain flaky test",
        Some(500),
        None,
    )
    .expect("run invocation");

    assert_eq!(report.agent, "claude");
    assert_eq!(report.status, "fallback");
    assert!(report.fallback_used);
    assert!(
        report
            .prompt_preview
            .expect("fallback prompt")
            .contains("[CTX COMPACT CONTEXT]")
    );

    let stats = std::fs::read_to_string(tmp.path().join(".ctx/stats/latest.json")).expect("stats");
    assert!(stats.contains("claude"));
    assert!(stats.contains("fallback"));

    let audit = std::fs::read_to_string(tmp.path().join(".ctx/audit.log")).expect("audit");
    assert!(audit.contains("adapter_invocation"));
}

#[test]
fn run_pack_includes_advanced_context_and_writes_pack_artifact() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .expect("git init");
    Command::new("git")
        .args(["config", "user.email", "ctx@example.test"])
        .current_dir(tmp.path())
        .output()
        .expect("git config email");
    Command::new("git")
        .args(["config", "user.name", "CTX Test"])
        .current_dir(tmp.path())
        .output()
        .expect("git config name");

    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(
        tmp.path().join("src/auth.rs"),
        r#"
fn validate_refresh_token(token: &str) -> bool {
    decode_token(token)
}

fn decode_token(token: &str) -> bool {
    !token.is_empty()
}
"#,
    )
    .expect("write");
    Command::new("git")
        .args(["add", "."])
        .current_dir(tmp.path())
        .output()
        .expect("git add");
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(tmp.path())
        .output()
        .expect("git commit");

    fs::write(
        tmp.path().join("src/auth.rs"),
        r#"
fn validate_refresh_token(token: &str) -> bool {
    decode_token(token) && token != "expired"
}

fn decode_token(token: &str) -> bool {
    !token.is_empty()
}
"#,
    )
    .expect("modify");

    run_index(tmp.path(), &[]).expect("index");
    let store = GraphStore::open(&tmp.path().join(".ctx/graph.db")).expect("graph");
    store.init_schema().expect("schema");
    let run_id = store
        .record_run("pytest tests/auth.rs", "failed")
        .expect("run");
    store
        .record_failure(run_id, "expired refresh token", Some("rotation skipped"))
        .expect("failure");
    store
        .record_decision("Auth API", "preserve validate_refresh_token signature")
        .expect("decision");

    let packed = run_pack(tmp.path(), "fix refresh token", Some(220), None).expect("pack");

    assert!(packed.compact_context.contains("recent_diff:"));
    assert!(packed.compact_context.contains("dependencies:"));
    assert!(packed.compact_context.contains("task_memory:"));
    assert!(packed.compact_context.contains("failure_memory:"));
    let pack_path = packed.pack_path.expect("pack path");
    assert!(std::path::Path::new(&pack_path).exists());
    assert!(
        packed
            .included
            .iter()
            .any(|entry| entry.contains("included"))
    );
}
