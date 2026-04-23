use std::fs;

use ctx_core::{init_repo, run_graph_query, run_index, run_pack};
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
