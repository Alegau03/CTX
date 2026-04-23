use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ctx_ast::{SymbolKind, extract_symbols, slice_symbols};
use ctx_config::{CtxConfig, write_default_config};
use ctx_graph::{GraphStore, SnippetHit, SymbolHit};
use ctx_intake::{Intent, QueryIntake};
use ctx_pack::{PackInput, PackResult, build_pack};
use ctx_prune::{PruneReport, prune_diff, prune_logs};
use ctx_semantic::{ChunkCandidate, RankingConfig, SemanticBackendKind, rank_chunks_hybrid};
use ctx_telemetry::{StatsSnapshot, write_latest_stats};
use serde::Serialize;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize)]
pub struct ExplainResult {
    pub query: String,
    pub intent: Intent,
    pub likely_symbols: Vec<String>,
    pub related_command_history: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetrievalHit {
    pub id: String,
    pub source: String,
    pub content: String,
    pub score: f64,
    pub reason: String,
}

pub fn init_repo(repo_root: &Path) -> Result<PathBuf> {
    let config_path = write_default_config(repo_root)?;

    let cfg = CtxConfig::load(repo_root)?;
    if cfg.graph.enabled {
        let graph_path = repo_root.join(&cfg.graph.store);
        let store = GraphStore::open(&graph_path)?;
        store.init_schema()?;
    }

    Ok(config_path)
}

pub fn load_or_default_config(repo_root: &Path) -> Result<CtxConfig> {
    let config_path = repo_root.join(".ctx/config.toml");
    if config_path.exists() {
        CtxConfig::load(repo_root)
    } else {
        Ok(CtxConfig::default())
    }
}

pub fn run_prune_logs(input: &str, max_lines: usize) -> PruneReport {
    prune_logs(input, max_lines)
}

pub fn run_prune_diff(input: &str, query: &str, max_lines: usize) -> PruneReport {
    prune_diff(input, query, max_lines)
}

pub fn run_pack(
    repo_root: &Path,
    query: &str,
    budget: Option<usize>,
    attach: Option<&Path>,
) -> Result<PackResult> {
    let cfg = load_or_default_config(repo_root)?;
    let max_lines = cfg.pruning.max_log_lines;

    let root_cause = if let Some(path) = attach {
        if cfg.security.exclude_sensitive_files
            && is_sensitive_path(path, &cfg.security.sensitive_patterns)
        {
            bail!(
                "attachment {} matches sensitive file patterns and was blocked",
                path.display()
            );
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read attachment {}", path.display()))?;
        let pruned = run_prune_logs(&raw, max_lines);
        Some(pruned.output)
    } else {
        None
    };

    let retrieved = run_retrieve(repo_root, query, 8).unwrap_or_default();
    let mut symbols = Vec::new();
    let mut docs = Vec::new();
    for hit in &retrieved {
        if hit.source == "symbol" {
            symbols.push(hit.content.clone());
        } else {
            docs.push(hit.content.clone());
        }
    }

    let pack_input = PackInput {
        query: query.to_string(),
        error_root_cause: root_cause,
        symbols,
        tests: Vec::new(),
        recent_diff: None,
        dependencies: Vec::new(),
        memory: Vec::new(),
        docs,
        budget: budget.unwrap_or(cfg.general.default_budget),
    };

    let result = build_pack(&pack_input);

    let stats = StatsSnapshot {
        original_tokens: result.original_estimated_tokens,
        packed_tokens: result.packed_tokens,
        reduction_pct: result.reduction_pct,
        latency_ms: 0,
    };
    let _ = write_latest_stats(&repo_root.join(".ctx/stats"), &stats);
    let _ = append_audit_entry(
        repo_root,
        &format!(
            "run_pack query=\"{}\" packed_tokens={} reduction_pct={:.2}",
            query, result.packed_tokens, result.reduction_pct
        ),
    );

    Ok(result)
}

pub fn run_explain(repo_root: &Path, query: &str) -> Result<ExplainResult> {
    let intake = QueryIntake::new(query, &repo_root.to_string_lossy());
    let hits = run_retrieve(repo_root, query, 5).unwrap_or_default();

    let mut likely_symbols = hits
        .iter()
        .filter(|h| h.source == "symbol")
        .map(|h| h.content.clone())
        .collect::<Vec<_>>();
    likely_symbols.sort();
    likely_symbols.dedup();

    Ok(ExplainResult {
        query: query.to_string(),
        intent: intake.intent,
        likely_symbols,
        related_command_history: vec!["local history unavailable yet".to_string()],
    })
}

pub fn run_index(repo_root: &Path, include_paths: &[String]) -> Result<usize> {
    let cfg = load_or_default_config(repo_root)?;
    if !cfg.graph.enabled {
        bail!("graph is disabled in config")
    }

    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;

    let roots: Vec<PathBuf> = if include_paths.is_empty() {
        vec![repo_root.to_path_buf()]
    } else {
        include_paths.iter().map(|p| repo_root.join(p)).collect()
    };

    let mut indexed = 0usize;
    for root in roots {
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !is_ignored_dir(e.path()))
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if !is_code_file(path) {
                continue;
            }
            if cfg.security.exclude_sensitive_files
                && is_sensitive_path(path, &cfg.security.sensitive_patterns)
            {
                continue;
            }

            let rel = path
                .strip_prefix(repo_root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            store.index_file(&rel)?;

            if let Ok(content) = fs::read_to_string(path) {
                index_symbols_and_edges(&store, &rel, &content)?;
            }
            indexed += 1;
        }
    }

    Ok(indexed)
}

pub fn run_graph_query(repo_root: &Path, query: &str) -> Result<Vec<String>> {
    let cfg = load_or_default_config(repo_root)?;
    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;
    store.query_files(query)
}

pub fn run_retrieve(repo_root: &Path, query: &str, top_k: usize) -> Result<Vec<RetrievalHit>> {
    let cfg = load_or_default_config(repo_root)?;
    if !cfg.graph.enabled {
        return Ok(Vec::new());
    }

    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;

    let terms = query_terms(query);
    let mut symbol_hits = Vec::new();
    let mut snippet_hits = Vec::new();

    for term in &terms {
        symbol_hits.extend(store.search_symbols(term)?);
        snippet_hits.extend(store.search_snippets(term, 20)?);
    }

    // add local neighborhood from graph traversal
    let mut neighborhood = Vec::new();
    for sym in symbol_hits.iter().take(10) {
        neighborhood.extend(store.related_symbols(&sym.name, 10)?);
    }
    symbol_hits.extend(neighborhood);

    dedup_symbol_hits(&mut symbol_hits);
    dedup_snippet_hits(&mut snippet_hits);

    let recent_failure_text = store
        .recent_failures(20)
        .unwrap_or_default()
        .into_iter()
        .map(|f| format!("{} {}", f.message, f.root_cause.unwrap_or_default()))
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();

    let mut candidates = Vec::new();
    for hit in &symbol_hits {
        let failure_rel = failure_overlap_score(query, &recent_failure_text);
        candidates.push(ChunkCandidate {
            id: format!("symbol:{}", hit.id),
            text: format!("{} {} {}", hit.file_path, hit.name, hit.signature),
            keyword_hint: format!("{} {}", hit.name, hit.file_path),
            recency: 0.7,
            graph_distance: 1.0,
            failure_relevance: failure_rel,
        });
    }

    for hit in &snippet_hits {
        let hint = hit
            .symbol_name
            .clone()
            .unwrap_or_else(|| hit.file_path.clone());
        candidates.push(ChunkCandidate {
            id: format!("snippet:{}", hit.snippet_id),
            text: hit.content.clone(),
            keyword_hint: hint,
            recency: 0.8,
            graph_distance: 1.4,
            failure_relevance: failure_overlap_score(query, &recent_failure_text),
        });
    }

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let ranked = rank_chunks_hybrid(
        query,
        &candidates,
        RankingConfig {
            backend: SemanticBackendKind::LocalHash,
            max_chunks: top_k.max(1),
            adaptive_threshold: true,
        },
    );

    let symbol_map = symbol_hits
        .iter()
        .map(|h| (format!("symbol:{}", h.id), h))
        .collect::<HashMap<_, _>>();
    let snippet_map = snippet_hits
        .iter()
        .map(|h| (format!("snippet:{}", h.snippet_id), h))
        .collect::<HashMap<_, _>>();

    let mut out = Vec::new();
    for item in ranked.into_iter().take(top_k.max(1)) {
        let (source, content) = if let Some(sym) = symbol_map.get(&item.id) {
            (
                "symbol".to_string(),
                format!("{}::{}", sym.file_path, sym.name),
            )
        } else if let Some(snippet) = snippet_map.get(&item.id) {
            ("snippet".to_string(), snippet.content.clone())
        } else {
            ("unknown".to_string(), item.text.clone())
        };

        out.push(RetrievalHit {
            id: item.id,
            source,
            content,
            score: item.score,
            reason: item.reason,
        });
    }

    Ok(out)
}

fn index_symbols_and_edges(store: &GraphStore, file_path: &str, content: &str) -> Result<()> {
    let symbols = extract_symbols(content, file_path);
    if symbols.is_empty() {
        return Ok(());
    }

    let mut ids_by_name = HashMap::new();
    for symbol in &symbols {
        let kind = kind_to_str(&symbol.kind);
        let id = store.upsert_symbol(file_path, &symbol.name, kind, &symbol.signature)?;
        ids_by_name.insert(symbol.name.clone(), id);

        let slices = slice_symbols(content, file_path, &[symbol.name.as_str()]);
        if let Some(slice) = slices.first() {
            let snippet = slice.content.trim();
            if !snippet.is_empty() {
                let _ = store.add_snippet(file_path, Some(&symbol.name), snippet);
            }
        }
    }

    // Naive call/test edges from sliced function/test content.
    for symbol in &symbols {
        if !matches!(symbol.kind, SymbolKind::Function | SymbolKind::Test) {
            continue;
        }

        let caller_id = if let Some(id) = ids_by_name.get(&symbol.name) {
            *id
        } else {
            continue;
        };

        let slices = slice_symbols(content, file_path, &[symbol.name.as_str()]);
        let body = slices
            .first()
            .map(|s| s.content.as_str())
            .unwrap_or_default();

        for (candidate_name, candidate_id) in &ids_by_name {
            if candidate_name == &symbol.name {
                continue;
            }

            let token = format!("{}(", candidate_name);
            if body.contains(&token) {
                let edge_type = if matches!(symbol.kind, SymbolKind::Test) {
                    "tests"
                } else {
                    "calls"
                };
                let _ = store.link_symbols(caller_id, *candidate_id, edge_type, None);
            }
        }
    }

    Ok(())
}

fn kind_to_str(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Module => "module",
        SymbolKind::Class => "class",
        SymbolKind::Function => "function",
        SymbolKind::Test => "test",
        SymbolKind::Import => "import",
    }
}

fn dedup_symbol_hits(items: &mut Vec<SymbolHit>) {
    items.sort_by_key(|h| h.id);
    items.dedup_by_key(|h| h.id);
}

fn dedup_snippet_hits(items: &mut Vec<SnippetHit>) {
    items.sort_by_key(|h| h.snippet_id);
    items.dedup_by_key(|h| h.snippet_id);
}

fn failure_overlap_score(query: &str, failure_text: &str) -> f64 {
    if failure_text.is_empty() {
        return 0.0;
    }

    let tokens = query_terms(query);
    if tokens.is_empty() {
        return 0.0;
    }

    let overlap = tokens
        .iter()
        .filter(|token| failure_text.contains(token.as_str()))
        .count() as f64;
    (overlap / tokens.len() as f64).clamp(0.0, 1.0)
}

fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|part| part.len() > 1)
        .map(|part| part.to_lowercase())
        .collect()
}

fn is_ignored_dir(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|name| {
                matches!(
                    name,
                    ".git" | ".ctx" | "target" | "node_modules" | "build" | "dist" | "artifacts"
                )
            })
            .unwrap_or(false)
    })
}

fn is_code_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return false;
    };

    matches!(
        ext,
        "py" | "rs"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "go"
            | "java"
            | "kt"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
    )
}

fn is_sensitive_path(path: &Path, patterns: &[String]) -> bool {
    let lower = path.to_string_lossy().to_lowercase();
    patterns
        .iter()
        .any(|pattern| lower.contains(&pattern.to_lowercase()))
}

fn append_audit_entry(repo_root: &Path, line: &str) -> Result<()> {
    let audit_path = repo_root.join(".ctx/audit.log");
    if let Some(parent) = audit_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&audit_path)
        .with_context(|| format!("failed to open {}", audit_path.display()))?;
    writeln!(file, "{line}").context("failed to append audit entry")?;
    Ok(())
}
