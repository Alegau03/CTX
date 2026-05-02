use std::fs;

use ctx_core::{init_repo, run_index, run_retrieve};
use tempfile::tempdir;

#[test]
fn retrieval_returns_relevant_auth_context() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(
        tmp.path().join("src/auth.rs"),
        r#"
use crate::tokens::decode_token;

fn validate_refresh_token(token: &str) -> bool {
    decode_token(token)
}

fn decode_token(token: &str) -> bool {
    !token.is_empty()
}
"#,
    )
    .expect("write");

    run_index(tmp.path(), &[]).expect("index");

    let hits = run_retrieve(tmp.path(), "fix refresh token decode failure", 5).expect("retrieve");
    assert!(!hits.is_empty());
    assert!(
        hits.iter()
            .any(|h| h.content.contains("validate_refresh_token"))
    );
}

#[test]
fn retrieval_returns_relevant_typescript_context() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(
        tmp.path().join("src/auth.ts"),
        r#"
import { decodeToken } from "./tokens";

export const validateRefreshToken = (token: string): boolean => {
    return decodeToken(token);
};
"#,
    )
    .expect("write");

    run_index(tmp.path(), &[]).expect("index");

    let hits = run_retrieve(tmp.path(), "refresh token decode failure", 5).expect("retrieve");
    assert!(!hits.is_empty());
    assert!(
        hits.iter()
            .any(|h| h.content.contains("validateRefreshToken"))
    );
}

#[test]
fn retrieval_respects_limit() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(
        tmp.path().join("src/a.rs"),
        "fn a() {}\nfn b() {}\nfn c() {}\n",
    )
    .expect("write");

    run_index(tmp.path(), &[]).expect("index");

    let hits = run_retrieve(tmp.path(), "function", 2).expect("retrieve");
    assert!(hits.len() <= 2);
}

#[test]
fn retrieval_uses_semantic_config_and_reports_onnx_fallback() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    fs::write(
        tmp.path().join(".ctx/config.toml"),
        r#"
[general]
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
model = "models/missing.onnx"
vocab = "models/vocab.txt"
max_chunks = 8
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
exclude_sensitive_files = true
sensitive_patterns = [".env", "id_rsa", ".pem", ".key", "credentials", "secret"]
"#,
    )
    .expect("write config");

    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(
        tmp.path().join("src/auth.rs"),
        "fn validate_refresh_token() -> bool { true }\n",
    )
    .expect("write");

    run_index(tmp.path(), &[]).expect("index");
    let hits = run_retrieve(tmp.path(), "fix refresh token", 5).expect("retrieve");

    assert!(!hits.is_empty());
    assert!(
        hits.iter()
            .any(|hit| hit.reason.contains("fallback_from=onnx"))
    );
}

#[test]
fn retrieval_returns_markdown_runbook_context() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    fs::create_dir_all(tmp.path().join("docs")).expect("mkdir");
    fs::write(
        tmp.path().join("docs/docker-runbook.md"),
        r#"
# Docker Compose

Use docker compose up -d to boot the local stack.

## Services

The api service depends on redis.
"#,
    )
    .expect("write markdown");

    run_index(tmp.path(), &[]).expect("index");

    let hits = run_retrieve(tmp.path(), "docker compose", 5).expect("retrieve");
    assert!(!hits.is_empty());
    assert!(
        hits.iter()
            .any(|h| h.content.contains("Docker Compose") || h.content.contains("docker compose"))
    );
}
