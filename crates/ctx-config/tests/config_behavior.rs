use std::fs;

use ctx_config::{CtxConfig, write_default_config};
use tempfile::tempdir;

#[test]
fn parses_minimal_toml_into_defaults() {
    let parsed = CtxConfig::from_toml_str(
        r#"
[general]
default_budget = 7000
agent = "claude"
"#,
    )
    .expect("parse should succeed");

    assert_eq!(parsed.general.default_budget, 7000);
    assert_eq!(parsed.general.agent, "claude");
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
fn template_config_is_valid() {
    let template = std::fs::read_to_string("../../templates/config.default.toml")
        .expect("template config should exist");
    let parsed = CtxConfig::from_toml_str(&template).expect("template should parse");
    assert_eq!(parsed.general.default_budget, 6000);
    assert!(parsed.security.exclude_sensitive_files);
    assert!(!parsed.security.sensitive_patterns.is_empty());
}
