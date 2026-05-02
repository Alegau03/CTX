use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{append_audit_entry, run_prune_logs};

#[derive(Debug, Clone, Serialize)]
pub struct CommandRunReport {
    pub command: String,
    pub exit_code: i32,
    pub latency_ms: u64,
    pub raw_log_path: String,
    pub pruned_output: String,
    pub summary: String,
}

pub fn run_command_capture(
    repo_root: &Path,
    command: &str,
    max_lines: usize,
) -> Result<CommandRunReport> {
    let commands_dir = repo_root.join(".ctx/cache/commands");
    fs::create_dir_all(&commands_dir)
        .with_context(|| format!("failed to create {}", commands_dir.display()))?;

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time drifted before unix epoch")?
        .as_millis();
    let raw_log_path = commands_dir.join(format!("run-{ts}.log"));

    let wrapped = format!("{{ {command}; }} 2>&1");
    let start = Instant::now();
    let output = Command::new("sh")
        .arg("-lc")
        .arg(&wrapped)
        .current_dir(repo_root)
        .output()
        .with_context(|| format!("failed to run shell command: {command}"))?;
    let latency_ms = start.elapsed().as_millis() as u64;

    let combined = String::from_utf8_lossy(&output.stdout).to_string();
    fs::write(&raw_log_path, &combined)
        .with_context(|| format!("failed to write {}", raw_log_path.display()))?;

    let pruned = run_prune_logs(&combined, max_lines);
    let pruned_output = if pruned.output.trim().is_empty() {
        combined
            .lines()
            .take(max_lines)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    } else {
        pruned.output
    };
    let exit_code = output.status.code().unwrap_or(-1);
    let summary = pruned_output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("command exited with code {exit_code}"));

    let _ = append_audit_entry(
        repo_root,
        &format!(
            "run_command command=\"{}\" exit_code={} latency_ms={} raw_log_path={}",
            command.replace('"', "\\\""),
            exit_code,
            latency_ms,
            raw_log_path.display()
        ),
    );

    Ok(CommandRunReport {
        command: command.to_string(),
        exit_code,
        latency_ms,
        raw_log_path: raw_log_path.to_string_lossy().to_string(),
        pruned_output,
        summary,
    })
}
