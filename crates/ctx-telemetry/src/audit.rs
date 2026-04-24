use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub kind: String,
    pub message: String,
    pub agent: Option<String>,
    pub command: Option<String>,
    pub status: Option<String>,
    pub fallback_used: bool,
    pub pack_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyAuditEvent {
    pub kind: String,
    pub decision: String,
    pub path: Option<String>,
    pub reason: String,
    pub local_only: bool,
    pub remote_upload_enabled: bool,
    pub message: String,
}

pub fn append_audit_line(audit_path: &Path, line: &str) -> Result<()> {
    if let Some(parent) = audit_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create audit parent {}", parent.display()))?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(audit_path)
        .with_context(|| format!("failed to open {}", audit_path.display()))?;
    writeln!(file, "{line}").context("failed to append audit line")?;
    Ok(())
}

pub fn append_audit_event(audit_path: &Path, event: &AuditEvent) -> Result<()> {
    let line = serde_json::to_string(event).context("failed to serialize audit event")?;
    append_audit_line(audit_path, &line)
}

pub fn append_privacy_audit_event(audit_path: &Path, event: &PrivacyAuditEvent) -> Result<()> {
    let line = serde_json::to_string(event).context("failed to serialize privacy audit event")?;
    append_audit_line(audit_path, &line)
}
