use std::collections::{BTreeMap, hash_map::DefaultHasher};
use std::fmt;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, anyhow, bail};
use ctx_ast::{SymbolKind, extract_symbols};
use serde::{Deserialize, Serialize};

use crate::path_filters::PathMatcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadMode {
    Full,
    Outline,
    Digest,
}

impl ReadMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Outline => "outline",
            Self::Digest => "digest",
        }
    }
}

impl fmt::Display for ReadMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ReadMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "full" => Ok(Self::Full),
            "outline" => Ok(Self::Outline),
            "digest" => Ok(Self::Digest),
            other => Err(anyhow!(
                "unknown read mode '{other}'. Expected: full, outline, digest"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadCacheReport {
    pub path: String,
    pub mode: ReadMode,
    pub cache_hit: bool,
    pub fingerprint: String,
    pub line_count: usize,
    pub symbol_count: usize,
    pub output: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ReadCacheState {
    #[serde(default)]
    cache_hits: usize,
    #[serde(default)]
    cache_misses: usize,
    #[serde(default)]
    files: BTreeMap<String, CachedFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedFileEntry {
    fingerprint: String,
    line_count: usize,
    symbol_count: usize,
    outline_lines: Vec<String>,
}

pub fn run_cached_read(
    repo_root: &Path,
    raw_path: &str,
    mode: ReadMode,
    exclude_sensitive_files: bool,
    sensitive_patterns: &[String],
    ignored_files: &[String],
) -> Result<ReadCacheReport> {
    let target = resolve_repo_file(repo_root, raw_path)?;
    let ignored_files = PathMatcher::exact_or_glob(ignored_files);
    let sensitive_files = PathMatcher::contains_or_glob(sensitive_patterns);
    if exclude_sensitive_files && sensitive_files.matches_path(Some(repo_root), &target) {
        bail!(
            "read path {} matches sensitive file patterns and was blocked",
            target.display()
        );
    }
    if ignored_files.matches_path(Some(repo_root), &target) {
        bail!(
            "read path {} matches ignored file patterns and was blocked",
            target.display()
        );
    }

    let display_path = relativize_path(repo_root, &target);
    let content = fs::read_to_string(&target)
        .with_context(|| format!("failed to read {}", target.display()))?;
    let line_count = content.lines().count();
    let fingerprint = compute_fingerprint(&content);
    let outline_lines = build_outline_lines(&content, &display_path);
    let symbol_count = outline_lines.len();

    let cache_dir = repo_root.join(".ctx/cache");
    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("failed to create {}", cache_dir.display()))?;
    let cache_path = cache_dir.join("read-session.json");
    let mut state = load_cache_state(&cache_path)?;
    let cache_hit = state
        .files
        .get(&display_path)
        .map(|entry| entry.fingerprint == fingerprint)
        .unwrap_or(false);
    if cache_hit {
        state.cache_hits += 1;
    } else {
        state.cache_misses += 1;
    }

    let entry = CachedFileEntry {
        fingerprint: fingerprint.clone(),
        line_count,
        symbol_count,
        outline_lines: outline_lines.clone(),
    };
    state.files.insert(display_path.clone(), entry.clone());
    save_cache_state(&cache_path, &state)?;

    let output = match mode {
        ReadMode::Full => content.clone(),
        ReadMode::Outline => render_outline(&display_path, &entry),
        ReadMode::Digest => render_digest(&display_path, &entry, cache_hit),
    };
    let summary = match (mode, cache_hit) {
        (ReadMode::Digest, true) => format!("digest cache hit for unchanged file {display_path}"),
        (ReadMode::Digest, false) => format!("digest cache miss for {display_path}"),
        (ReadMode::Outline, _) => format!("outline read for {display_path}"),
        (ReadMode::Full, _) => format!("full read for {display_path}"),
    };

    Ok(ReadCacheReport {
        path: display_path,
        mode,
        cache_hit,
        fingerprint,
        line_count,
        symbol_count,
        output,
        summary,
    })
}

fn load_cache_state(path: &Path) -> Result<ReadCacheState> {
    if !path.exists() {
        return Ok(ReadCacheState::default());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

fn save_cache_state(path: &Path, state: &ReadCacheState) -> Result<()> {
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(state)?))
        .with_context(|| format!("failed to write {}", path.display()))
}

fn resolve_repo_file(repo_root: &Path, raw_path: &str) -> Result<PathBuf> {
    let candidate = PathBuf::from(raw_path);
    let target = if candidate.is_absolute() {
        candidate
    } else {
        repo_root.join(candidate)
    };
    if !target.is_file() {
        bail!("read path not found: {}", target.display());
    }

    let repo_canon = repo_root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", repo_root.display()))?;
    let target_canon = target
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", target.display()))?;
    if !target_canon.starts_with(&repo_canon) {
        bail!(
            "read path {} is outside the repository root {}",
            target.display(),
            repo_root.display()
        );
    }

    Ok(target_canon)
}

fn relativize_path(repo_root: &Path, target: &Path) -> String {
    let repo_canon = repo_root.canonicalize().ok();
    if let Some(repo_canon) = repo_canon
        && let Ok(stripped) = target.strip_prefix(repo_canon)
    {
        return stripped.to_string_lossy().to_string();
    }
    target.to_string_lossy().to_string()
}

fn compute_fingerprint(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn build_outline_lines(content: &str, display_path: &str) -> Vec<String> {
    let symbols = extract_symbols(content, display_path);
    if !symbols.is_empty() {
        return symbols
            .into_iter()
            .take(12)
            .map(|symbol| {
                format!(
                    "- {} {} :: {}",
                    symbol_kind_label(&symbol.kind),
                    symbol.name,
                    symbol.signature
                )
            })
            .collect();
    }

    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(12)
        .map(|line| format!("- {line}"))
        .collect()
}

fn render_outline(path: &str, entry: &CachedFileEntry) -> String {
    let mut out = vec![
        format!("path: {path}"),
        "mode: outline".to_string(),
        format!("fingerprint: {}", entry.fingerprint),
        format!("lines: {}", entry.line_count),
        format!("symbols: {}", entry.symbol_count),
    ];
    if entry.outline_lines.is_empty() {
        out.push("outline: no symbols detected".to_string());
    } else {
        out.push("outline:".to_string());
        out.extend(entry.outline_lines.iter().cloned());
    }
    out.join("\n")
}

fn render_digest(path: &str, entry: &CachedFileEntry, cache_hit: bool) -> String {
    let mut out = vec![
        format!("path: {path}"),
        "mode: digest".to_string(),
        format!("cache: {}", if cache_hit { "hit" } else { "miss" }),
        format!("fingerprint: {}", entry.fingerprint),
        format!("lines: {}", entry.line_count),
        format!("symbols: {}", entry.symbol_count),
    ];
    if entry.outline_lines.is_empty() {
        out.push("outline: no symbols detected".to_string());
    } else {
        out.push("outline:".to_string());
        out.extend(entry.outline_lines.iter().take(6).cloned());
    }
    out.join("\n")
}

fn symbol_kind_label(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Module => "module",
        SymbolKind::Class => "class",
        SymbolKind::Function => "function",
        SymbolKind::Test => "test",
        SymbolKind::Import => "import",
    }
}
