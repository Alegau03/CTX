use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSnapshot {
    pub original_tokens: usize,
    pub packed_tokens: usize,
    pub reduction_pct: f64,
    pub latency_ms: u64,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub fallback_used: bool,
    #[serde(default)]
    pub pack_path: Option<String>,
}

pub fn write_latest_stats(stats_dir: &Path, snapshot: &StatsSnapshot) -> Result<()> {
    fs::create_dir_all(stats_dir)
        .with_context(|| format!("failed to create stats dir {}", stats_dir.display()))?;

    let body = serde_json::to_string_pretty(snapshot).context("failed to serialize stats")?;
    fs::write(stats_dir.join("latest.json"), body).context("failed to write latest stats")?;
    Ok(())
}

pub fn read_latest_stats(stats_dir: &Path) -> Result<StatsSnapshot> {
    let body = fs::read_to_string(stats_dir.join("latest.json")).context("failed to read stats")?;
    serde_json::from_str(&body).context("failed to parse stats json")
}
