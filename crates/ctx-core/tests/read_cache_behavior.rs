use std::fs;
use std::path::Path;

use ctx_core::{ReadMode, init_repo, run_read};
use tempfile::tempdir;

fn write_test_config(repo_root: &Path, ignored_files: &[&str]) {
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
ignored_dirs = [".git", ".ctx"]
ignored_files = [{ignored_files}]
"#
    );
    fs::write(repo_root.join(".ctx/config.toml"), config).expect("write config");
}

#[test]
fn full_read_returns_file_content_on_first_access() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let path = tmp.path().join("src/auth.ts");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(
        &path,
        "export function validateRefreshToken(token: string) {\n  return token.length > 10;\n}\n",
    )
    .expect("write file");

    let report = run_read(tmp.path(), "src/auth.ts", ReadMode::Full).expect("run read");

    assert_eq!(report.mode, ReadMode::Full);
    assert!(!report.cache_hit);
    assert!(report.output.contains("validateRefreshToken"));
    assert!(report.output.contains("return token.length > 10"));
}

#[test]
fn outline_read_surfaces_symbols_without_full_body_dump() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let path = tmp.path().join("src/session.ts");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(
        &path,
        "export function hydrateSession(token: string) {\n  const refresh = token.trim();\n  return refresh;\n}\n",
    )
    .expect("write file");

    let report = run_read(tmp.path(), "src/session.ts", ReadMode::Outline).expect("run read");

    assert_eq!(report.mode, ReadMode::Outline);
    assert!(!report.cache_hit);
    assert!(report.output.contains("hydrateSession"));
    assert!(!report.output.contains("const refresh = token.trim()"));
}

#[test]
fn digest_reread_hits_cache_for_unchanged_file() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let path = tmp.path().join("docs/runbook.md");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(
        &path,
        "# Docker Compose\n\n## Services\n\nUse docker compose up.\n",
    )
    .expect("write file");

    let first = run_read(tmp.path(), "docs/runbook.md", ReadMode::Full).expect("seed read");
    let reread = run_read(tmp.path(), "docs/runbook.md", ReadMode::Digest).expect("digest reread");

    assert!(!first.cache_hit);
    assert!(reread.cache_hit);
    assert_eq!(first.fingerprint, reread.fingerprint);
    assert!(reread.output.contains("cache: hit"));
    assert!(reread.output.contains("Docker Compose"));
    assert!(!reread.output.contains("Use docker compose up."));
}

#[test]
fn digest_reread_invalidates_after_file_change() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let path = tmp.path().join("src/login.ts");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, "export const login = () => true;\n").expect("write file");

    let first = run_read(tmp.path(), "src/login.ts", ReadMode::Full).expect("seed read");
    fs::write(&path, "export const login = () => false;\n").expect("rewrite file");

    let reread = run_read(tmp.path(), "src/login.ts", ReadMode::Digest).expect("digest reread");

    assert!(!reread.cache_hit);
    assert_ne!(first.fingerprint, reread.fingerprint);
    assert!(reread.output.contains("cache: miss"));
}

#[test]
fn read_blocks_files_matching_ignored_file_patterns() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    write_test_config(tmp.path(), &["docs/*.md"]);
    let path = tmp.path().join("docs/runbook.md");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, "# Runbook\nnever read this through ctx\n").expect("write");

    let result = run_read(tmp.path(), "docs/runbook.md", ReadMode::Full);
    assert!(result.is_err());
    assert!(
        result
            .expect_err("expected ignored file block")
            .to_string()
            .contains("ignored file patterns")
    );
}
