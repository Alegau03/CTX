use std::collections::{BTreeMap, hash_map::DefaultHasher};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexCacheState {
    pub files: BTreeMap<String, IndexedFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFileEntry {
    pub fingerprint: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexCacheReport {
    pub scanned_files: usize,
    pub indexed_files: usize,
    pub reused_files: usize,
    pub changed_files: usize,
    pub new_files: usize,
}

#[derive(Debug, Clone)]
pub struct IndexCachePaths {
    pub state_path: String,
    pub report_path: String,
}

pub fn compute_fingerprint(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn load_index_cache_state(repo_root: &Path) -> Result<IndexCacheState> {
    let path = index_state_path(repo_root);
    if !path.exists() {
        return Ok(IndexCacheState::default());
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn save_index_cache_state(
    repo_root: &Path,
    state: &IndexCacheState,
) -> Result<IndexCachePaths> {
    let cache_dir = repo_root.join(".ctx/cache");
    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("failed to create {}", cache_dir.display()))?;
    let path = index_state_path(repo_root);
    fs::write(&path, format!("{}\n", serde_json::to_string_pretty(state)?))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(IndexCachePaths {
        state_path: path.to_string_lossy().to_string(),
        report_path: index_report_path(repo_root).to_string_lossy().to_string(),
    })
}

pub fn write_index_cache_report(
    repo_root: &Path,
    report: &IndexCacheReport,
) -> Result<IndexCachePaths> {
    let cache_dir = repo_root.join(".ctx/cache");
    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("failed to create {}", cache_dir.display()))?;
    let path = index_report_path(repo_root);
    fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(report)?),
    )
    .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(IndexCachePaths {
        state_path: index_state_path(repo_root).to_string_lossy().to_string(),
        report_path: path.to_string_lossy().to_string(),
    })
}

pub fn load_latest_index_cache_report(repo_root: &Path) -> Result<Option<IndexCacheReport>> {
    let path = index_report_path(repo_root);
    if !path.exists() {
        return Ok(None);
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let report = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(report))
}

fn index_state_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(".ctx/cache/index-state.json")
}

fn index_report_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(".ctx/cache/index-report.json")
}
