use std::fs;
use std::path::Path;
use std::process::Command;

use ctx_core::{
    init_repo, run_gain, run_graph_query, run_index, run_pack, run_reindex, run_retrieve,
};
use ctx_graph::GraphStore;
use tempfile::tempdir;

fn write_test_config(repo_root: &Path, ignored_dirs: &[&str], ignored_files: &[&str]) {
    let ignored_dirs = ignored_dirs
        .iter()
        .map(|pattern| format!("\"{pattern}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let ignored_files = ignored_files
        .iter()
        .map(|pattern| format!("\"{pattern}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let config = format!(
        r#"[general]
repo_root = "."
default_budget = 6000
agent = "opencode"

[pruning]
collapse_success_logs = true
keep_imports = true
keep_public_signatures = true
max_log_lines = 200

[semantic]
enabled = true
backend = "onnx"
model = "local-mini-embed"
max_chunks = 64
allow_fallback = true

[graph]
enabled = true
store = ".ctx/graph.db"
index_tests = true
index_docs = true

[mcp]
enabled = true
port = 8765

[security]
local_only = true
remote_upload_enabled = false
anonymous_telemetry_enabled = false
local_stats_enabled = true
audit_include_exclude = true
exclude_sensitive_files = true
sensitive_patterns = [".env", "id_rsa", ".pem", ".key", "credentials", "secret"]
ignored_dirs = [{ignored_dirs}]
ignored_files = [{ignored_files}]
"#
    );
    fs::write(repo_root.join(".ctx/config.toml"), config).expect("write config");
}

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
fn index_and_graph_query_find_typescript_symbols() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(
        tmp.path().join("src/auth.ts"),
        r#"
export class AuthService {
  validateRefreshToken(token: string): boolean {
    return token.length > 0;
  }
}
"#,
    )
    .expect("write");

    let count = run_index(tmp.path(), &[]).expect("index");
    assert!(count >= 1);

    let matches = run_graph_query(tmp.path(), "auth").expect("query");
    assert!(matches.iter().any(|m| m.ends_with("src/auth.ts")));
}

#[test]
fn index_includes_runbook_and_config_file_types() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    fs::create_dir_all(tmp.path().join("docs")).expect("mkdir docs");
    fs::create_dir_all(tmp.path().join("ops")).expect("mkdir ops");
    fs::create_dir_all(tmp.path().join("scripts")).expect("mkdir scripts");
    fs::create_dir_all(tmp.path().join("config")).expect("mkdir config");
    fs::create_dir_all(tmp.path().join("fixtures")).expect("mkdir fixtures");

    fs::write(
        tmp.path().join("docs/docker-runbook.md"),
        "# Docker Compose\nBring up services with docker compose up -d.\n",
    )
    .expect("write markdown");
    fs::write(
        tmp.path().join("ops/docker-compose.yml"),
        "services:\n  api:\n    image: busybox\n",
    )
    .expect("write yaml");
    fs::write(
        tmp.path().join("scripts/deploy.sh"),
        "#!/usr/bin/env bash\necho deploy\n",
    )
    .expect("write shell");
    fs::write(
        tmp.path().join("config/settings.toml"),
        "[server]\nport = 3000\n",
    )
    .expect("write toml");
    fs::write(
        tmp.path().join("fixtures/manifest.json"),
        "{\n  \"name\": \"ctx\"\n}\n",
    )
    .expect("write json");

    let count = run_index(tmp.path(), &[]).expect("index");
    assert_eq!(count, 5);

    let docker_matches = run_graph_query(tmp.path(), "docker").expect("docker query");
    assert!(
        docker_matches
            .iter()
            .any(|m| m.ends_with("docs/docker-runbook.md"))
    );
    assert!(
        docker_matches
            .iter()
            .any(|m| m.ends_with("ops/docker-compose.yml"))
    );
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
fn run_index_skips_files_matching_ignored_file_patterns() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    write_test_config(
        tmp.path(),
        &[".git", ".ctx"],
        &["package-lock.json", "docs/*.md"],
    );
    fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    fs::create_dir_all(tmp.path().join("docs")).expect("mkdir docs");
    fs::write(
        tmp.path().join("src/auth.ts"),
        "export function refreshSession() { return true; }\n",
    )
    .expect("write auth");
    fs::write(
        tmp.path().join("docs/runbook.md"),
        "# Runbook\nrotate the session token\n",
    )
    .expect("write doc");
    fs::write(
        tmp.path().join("package-lock.json"),
        "{\n  \"name\": \"pulseboard-web-demo\"\n}\n",
    )
    .expect("write lockfile");

    let count = run_index(tmp.path(), &[]).expect("index");
    assert_eq!(count, 1);

    let auth_matches = run_graph_query(tmp.path(), "auth").expect("auth query");
    assert!(auth_matches.iter().any(|m| m.ends_with("src/auth.ts")));

    let doc_matches = run_graph_query(tmp.path(), "runbook").expect("doc query");
    assert!(doc_matches.is_empty());

    let lockfile_matches = run_graph_query(tmp.path(), "package-lock").expect("lock query");
    assert!(lockfile_matches.is_empty());

    let audit = fs::read_to_string(tmp.path().join(".ctx/audit.log")).expect("audit readable");
    assert!(audit.contains("\"reason\":\"ignored_file_pattern\""));
    assert!(audit.contains("docs/runbook.md"));
    assert!(audit.contains("package-lock.json"));
}

#[test]
fn run_index_skips_directories_matching_globbed_component_patterns() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    write_test_config(tmp.path(), &[".git", ".ctx", "*.egg-info"], &[]);
    fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    fs::create_dir_all(tmp.path().join("fixtures/demo.egg-info")).expect("mkdir egg-info");
    fs::write(
        tmp.path().join("src/auth.py"),
        "def validate_refresh_token():\n    return True\n",
    )
    .expect("write auth");
    fs::write(
        tmp.path().join("fixtures/demo.egg-info/metadata.py"),
        "def generated_metadata():\n    return 'ignore me'\n",
    )
    .expect("write metadata");

    let count = run_index(tmp.path(), &[]).expect("index");
    assert_eq!(count, 1);

    let matches = run_graph_query(tmp.path(), "generated_metadata").expect("query");
    assert!(matches.is_empty());
}

#[test]
fn run_reindex_prunes_graph_entries_that_become_ignored() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    fs::create_dir_all(tmp.path().join("docs")).expect("mkdir docs");
    fs::write(
        tmp.path().join("docs/runbook.md"),
        "# Runbook\nrefresh token notes\n",
    )
    .expect("write runbook");

    let first = run_index(tmp.path(), &[]).expect("seed index");
    assert_eq!(first, 1);
    let before = run_graph_query(tmp.path(), "runbook").expect("before query");
    assert!(before.iter().any(|path| path.ends_with("docs/runbook.md")));

    write_test_config(tmp.path(), &[".git", ".ctx"], &["docs/*.md"]);
    let rebuilt = run_reindex(tmp.path(), &[]).expect("reindex");
    assert_eq!(rebuilt, 0);

    let after = run_graph_query(tmp.path(), "runbook").expect("after query");
    assert!(after.is_empty());
}

#[test]
fn run_index_replaces_stale_snippets_for_changed_files() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    let auth_path = tmp.path().join("src/auth.ts");
    fs::write(
        &auth_path,
        "export function validateRefreshToken(token: string) { return token.length > 0; }\n",
    )
    .expect("write initial auth");

    run_index(tmp.path(), &[]).expect("seed index");
    fs::write(
        &auth_path,
        "export function rotateSession(userId: string) { return `rotated:${userId}`; }\n",
    )
    .expect("rewrite auth");

    let changed = run_index(tmp.path(), &[]).expect("changed index");
    assert_eq!(changed, 1);

    let stale_hits =
        run_retrieve(tmp.path(), "validateRefreshToken", 5).expect("stale retrieve query");
    assert!(
        stale_hits
            .iter()
            .all(|hit| !hit.content.contains("validateRefreshToken")),
        "stale symbol content should not survive reindex: {stale_hits:#?}"
    );

    let fresh_hits = run_retrieve(tmp.path(), "rotateSession", 5).expect("fresh retrieve query");
    assert!(
        fresh_hits
            .iter()
            .any(|hit| hit.content.contains("rotateSession"))
    );
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
fn run_pack_allows_ignored_log_attachments_for_diagnostics() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    write_test_config(tmp.path(), &[".git", ".ctx"], &["*.log"]);
    let attach = tmp.path().join("failure.log");
    fs::write(&attach, "ERROR token decode failed").expect("write");

    let result = run_pack(tmp.path(), "fix auth", Some(100), Some(&attach)).expect("pack");
    assert!(result.compact_context.contains("root_cause:"));
    assert!(result.compact_context.contains("ERROR token decode failed"));
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

#[test]
fn run_pack_uses_cross_file_dependencies_from_import_and_call_graph() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(
        tmp.path().join("src/tokens.rs"),
        r#"
pub fn decode_token(token: &str) -> bool {
    !token.is_empty()
}
"#,
    )
    .expect("write tokens");
    fs::write(
        tmp.path().join("src/auth.rs"),
        r#"
use crate::tokens::decode_token;

pub fn validate_refresh_token(token: &str) -> bool {
    decode_token(token)
}
"#,
    )
    .expect("write auth");

    run_index(tmp.path(), &[]).expect("index");

    let packed = run_pack(
        tmp.path(),
        "fix refresh token decode failure",
        Some(220),
        None,
    )
    .expect("pack");

    assert!(packed.compact_context.contains("dependencies:"));
    assert!(packed.compact_context.contains("validate_refresh_token"));
    assert!(packed.compact_context.contains("decode_token"));
    assert!(packed.compact_context.contains("src/tokens.rs"));
}

#[test]
fn run_gain_reports_recent_pack_savings() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let attach = tmp.path().join("failure.txt");
    fs::write(&attach, "ERROR token decode failed\nTraceback line 2").expect("write");

    run_pack(tmp.path(), "fix auth", Some(100), Some(&attach)).expect("first pack");
    run_pack(tmp.path(), "plan login", Some(120), Some(&attach)).expect("second pack");

    let report = run_gain(tmp.path(), 20).expect("gain report");
    assert_eq!(report.sampled_runs, 2);
    assert!(report.latest_reduction_pct.is_some());
    assert_eq!(report.top_queries.len(), 2);
}

#[test]
fn run_pack_includes_latest_index_cache_summary() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    let path = tmp.path().join("src/auth.rs");
    fs::write(&path, "fn validate_refresh_token() -> bool { true }\n").expect("write");

    run_index(tmp.path(), &[]).expect("first index");
    run_index(tmp.path(), &[]).expect("second index");

    let packed = run_pack(tmp.path(), "fix auth", Some(120), None).expect("pack");
    assert!(
        packed
            .included
            .iter()
            .any(|entry| entry.contains("index_cache included"))
    );
    assert!(
        packed
            .included
            .iter()
            .any(|entry| entry.contains("reused_files=1"))
    );
}
