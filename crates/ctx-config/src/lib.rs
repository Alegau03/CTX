use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const CONFIG_RELATIVE_PATH: &str = ".ctx/config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CtxConfig {
    pub general: GeneralConfig,
    pub pruning: PruningConfig,
    pub semantic: SemanticConfig,
    pub graph: GraphConfig,
    pub mcp: McpConfig,
    pub security: SecurityConfig,
}

impl Default for CtxConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            pruning: PruningConfig::default(),
            semantic: SemanticConfig::default(),
            graph: GraphConfig::default(),
            mcp: McpConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl CtxConfig {
    pub fn from_toml_str(raw: &str) -> Result<Self> {
        let parsed: CtxConfig = toml::from_str(raw).context("failed to parse config TOML")?;
        parsed.validate()?;
        Ok(parsed)
    }

    pub fn load(repo_root: &Path) -> Result<Self> {
        let config_path = repo_root.join(CONFIG_RELATIVE_PATH);
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read config at {}", config_path.display()))?;
        Self::from_toml_str(&content)
    }

    pub fn validate(&self) -> Result<()> {
        if self.general.default_budget == 0 {
            bail!("general.default_budget must be greater than 0")
        }

        if self.pruning.max_log_lines == 0 {
            bail!("pruning.max_log_lines must be greater than 0")
        }

        if self.semantic.enabled && self.semantic.max_chunks == 0 {
            bail!("semantic.max_chunks must be greater than 0 when semantic is enabled")
        }

        let semantic_backend = self.semantic.backend.trim().to_lowercase();
        if self.semantic.enabled
            && !matches!(
                semantic_backend.as_str(),
                "local" | "local_hash" | "hash" | "onnx" | "onnx_runtime"
            )
        {
            bail!("semantic.backend must be one of: local_hash, onnx")
        }

        if self.mcp.port == 0 {
            bail!("mcp.port must be greater than 0")
        }

        if self.security.local_only && self.security.remote_upload_enabled {
            bail!("security.remote_upload_enabled cannot be true when security.local_only is true")
        }

        if self.security.anonymous_telemetry_enabled && !self.security.remote_upload_enabled {
            bail!(
                "security.anonymous_telemetry_enabled requires security.remote_upload_enabled = true"
            )
        }

        if self.security.exclude_sensitive_files && self.security.sensitive_patterns.is_empty() {
            bail!(
                "security.sensitive_patterns must not be empty when security.exclude_sensitive_files is true"
            )
        }

        Ok(())
    }

    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(self).context("failed to serialize config")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub repo_root: String,
    pub default_budget: usize,
    pub agent: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            repo_root: ".".to_string(),
            default_budget: 6000,
            agent: "opencode".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PruningConfig {
    pub collapse_success_logs: bool,
    pub keep_imports: bool,
    pub keep_public_signatures: bool,
    pub max_log_lines: usize,
}

impl Default for PruningConfig {
    fn default() -> Self {
        Self {
            collapse_success_logs: true,
            keep_imports: true,
            keep_public_signatures: true,
            max_log_lines: 200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SemanticConfig {
    pub enabled: bool,
    pub backend: String,
    pub model: String,
    pub vocab: Option<String>,
    pub max_chunks: usize,
    pub allow_fallback: bool,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: "onnx".to_string(),
            model: "local-mini-embed".to_string(),
            vocab: None,
            max_chunks: 64,
            allow_fallback: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphConfig {
    pub enabled: bool,
    pub store: String,
    pub index_tests: bool,
    pub index_docs: bool,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            store: ".ctx/graph.db".to_string(),
            index_tests: true,
            index_docs: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpConfig {
    pub enabled: bool,
    pub port: u16,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 8765,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    pub local_only: bool,
    pub remote_upload_enabled: bool,
    pub anonymous_telemetry_enabled: bool,
    pub local_stats_enabled: bool,
    pub audit_include_exclude: bool,
    pub exclude_sensitive_files: bool,
    pub sensitive_patterns: Vec<String>,
    pub ignored_dirs: Vec<String>,
    pub ignored_files: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            local_only: true,
            remote_upload_enabled: false,
            anonymous_telemetry_enabled: false,
            local_stats_enabled: true,
            audit_include_exclude: true,
            exclude_sensitive_files: true,
            sensitive_patterns: vec![
                ".env".to_string(),
                "id_rsa".to_string(),
                ".pem".to_string(),
                ".key".to_string(),
                "credentials".to_string(),
                "secret".to_string(),
            ],
            ignored_dirs: vec![
                ".git".to_string(),
                ".ctx".to_string(),
                ".venv".to_string(),
                "venv".to_string(),
                "env".to_string(),
                "__pycache__".to_string(),
                ".pytest_cache".to_string(),
                ".ruff_cache".to_string(),
                ".mypy_cache".to_string(),
                ".tox".to_string(),
                ".nox".to_string(),
                ".idea".to_string(),
                ".vscode".to_string(),
                ".claude".to_string(),
                ".superpowers".to_string(),
                ".eggs".to_string(),
                "htmlcov".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                "build".to_string(),
                "dist".to_string(),
                "artifacts".to_string(),
                ".next".to_string(),
                ".cache".to_string(),
                "coverage".to_string(),
            ],
            ignored_files: vec![
                "*.db".to_string(),
                "*.sqlite".to_string(),
                "*.sqlite3".to_string(),
                "*.pyc".to_string(),
                "*.pyo".to_string(),
                "*.pem".to_string(),
                "*.log".to_string(),
                ".env".to_string(),
                "*.env".to_string(),
                ".coverage".to_string(),
                ".coverage.*".to_string(),
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
                "package-lock.json".to_string(),
            ],
        }
    }
}

pub fn write_default_config(repo_root: &Path) -> Result<PathBuf> {
    let config = CtxConfig::default();
    let ctx_dir = repo_root.join(".ctx");

    fs::create_dir_all(ctx_dir.join("packs")).context("failed to create .ctx/packs")?;
    fs::create_dir_all(ctx_dir.join("cache")).context("failed to create .ctx/cache")?;
    fs::create_dir_all(ctx_dir.join("stats")).context("failed to create .ctx/stats")?;

    let audit_log = ctx_dir.join("audit.log");
    if !audit_log.exists() {
        fs::write(&audit_log, "").context("failed to create .ctx/audit.log")?;
    }

    let config_path = repo_root.join(CONFIG_RELATIVE_PATH);
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).context("failed to create .ctx directory")?;
    }

    let rendered = config.to_toml_string()?;
    fs::write(&config_path, rendered)
        .with_context(|| format!("failed to write {}", config_path.display()))?;

    Ok(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let cfg = CtxConfig::default();
        assert!(cfg.validate().is_ok());
    }
}
