use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

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
    #[serde(default)]
    pub query: Option<String>,
}

pub fn write_latest_stats(stats_dir: &Path, snapshot: &StatsSnapshot) -> Result<()> {
    fs::create_dir_all(stats_dir)
        .with_context(|| format!("failed to create stats dir {}", stats_dir.display()))?;

    let body = serde_json::to_string_pretty(snapshot).context("failed to serialize stats")?;
    fs::write(stats_dir.join("latest.json"), body).context("failed to write latest stats")?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time drifted before unix epoch")?
        .as_nanos();
    fs::write(
        stats_dir.join(format!("run-{nanos}.json")),
        &serde_json::to_vec_pretty(snapshot)?,
    )
    .context("failed to write stats history snapshot")?;
    Ok(())
}

pub fn read_latest_stats(stats_dir: &Path) -> Result<StatsSnapshot> {
    let body = fs::read_to_string(stats_dir.join("latest.json")).context("failed to read stats")?;
    serde_json::from_str(&body).context("failed to parse stats json")
}

pub fn read_stats_history(stats_dir: &Path, limit: usize) -> Result<Vec<StatsSnapshot>> {
    if !stats_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(stats_dir)
        .with_context(|| format!("failed to read stats dir {}", stats_dir.display()))?
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            if name.starts_with("run-") && name.ends_with(".json") {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| right.file_name().cmp(&left.file_name()));

    let mut snapshots = Vec::new();
    for path in entries.into_iter().take(limit.max(1)) {
        let body = fs::read_to_string(&path)
            .with_context(|| format!("failed to read stats snapshot {}", path.display()))?;
        let snapshot =
            serde_json::from_str(&body).context("failed to parse stats history snapshot")?;
        snapshots.push(snapshot);
    }

    if snapshots.is_empty() && stats_dir.join("latest.json").exists() {
        snapshots.push(read_latest_stats(stats_dir)?);
    }

    Ok(snapshots)
}
