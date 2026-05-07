use std::fs;

use ctx_config::{CtxConfig, write_default_config};
use tempfile::tempdir;

#[test]
fn parses_minimal_toml_into_defaults() {
    let parsed = CtxConfig::from_toml_str(
        r#"
[general]
default_budget = 7000
agent = "opencode"
"#,
    )
    .expect("parse should succeed");

    assert_eq!(parsed.general.default_budget, 7000);
    assert_eq!(parsed.general.agent, "opencode");
    assert!(parsed.pruning.collapse_success_logs);
    assert_eq!(parsed.mcp.port, 8765);
}

#[test]
fn write_default_config_creates_ctx_structure() {
    let dir = tempdir().expect("tempdir");
    let config_path = write_default_config(dir.path()).expect("should write");

    assert!(config_path.ends_with(".ctx/config.toml"));
    assert!(config_path.exists());

    let ctx_dir = dir.path().join(".ctx");
    for entry in ["packs", "cache", "stats"] {
        assert!(ctx_dir.join(entry).exists(), "missing {}", entry);
    }
    assert!(ctx_dir.join("audit.log").exists());

    let content = fs::read_to_string(config_path).expect("config readable");
    assert!(content.contains("[general]"));
    assert!(content.contains("default_budget = 6000"));
}

#[test]
fn invalid_budget_fails_validation() {
    let result = CtxConfig::from_toml_str(
        r#"
[general]
default_budget = 0
"#,
    );

    assert!(result.is_err());
}

#[test]
fn semantic_config_supports_onnx_paths_and_fallback_policy() {
    let parsed = CtxConfig::from_toml_str(
        r#"
[semantic]
enabled = true
backend = "onnx"
model = "models/embed.onnx"
vocab = "models/vocab.txt"
max_chunks = 12
allow_fallback = false
"#,
    )
    .expect("semantic config should parse");

    assert_eq!(parsed.semantic.backend, "onnx");
    assert_eq!(parsed.semantic.model, "models/embed.onnx");
    assert_eq!(parsed.semantic.vocab.as_deref(), Some("models/vocab.txt"));
    assert_eq!(parsed.semantic.max_chunks, 12);
    assert!(!parsed.semantic.allow_fallback);
}

#[test]
fn template_config_is_valid() {
    let template = std::fs::read_to_string("../../templates/config.default.toml")
        .expect("template config should exist");
    let parsed = CtxConfig::from_toml_str(&template).expect("template should parse");
    assert_eq!(parsed.general.default_budget, 6000);
    assert!(parsed.security.exclude_sensitive_files);
    assert!(!parsed.security.sensitive_patterns.is_empty());
}

#[test]
fn security_defaults_are_local_first_and_telemetry_opt_in() {
    let cfg = CtxConfig::default();

    assert!(cfg.security.local_only);
    assert!(!cfg.security.remote_upload_enabled);
    assert!(!cfg.security.anonymous_telemetry_enabled);
    assert!(cfg.security.local_stats_enabled);
    assert!(cfg.security.audit_include_exclude);
    assert!(cfg.security.exclude_sensitive_files);
    for expected in ["node_modules", ".claude", ".venv", "__pycache__", ".vscode"] {
        assert!(
            cfg.security.ignored_dirs.iter().any(|dir| dir == expected),
            "missing default ignored dir: {expected}"
        );
    }
    for expected in ["package-lock.json", "*.log", ".coverage.*"] {
        assert!(
            cfg.security
                .ignored_files
                .iter()
                .any(|pattern| pattern == expected),
            "missing default ignored file pattern: {expected}"
        );
    }
}

#[test]
fn security_rejects_remote_upload_when_local_only_is_enabled() {
    let result = CtxConfig::from_toml_str(
        r#"
[security]
local_only = true
remote_upload_enabled = true
"#,
    );

    assert!(result.is_err());
}
