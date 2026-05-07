mod command_run;
mod index_cache;
mod path_filters;
mod read_cache;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use ctx_ast::{SymbolKind, extract_symbols, slice_symbols};
use ctx_config::{CtxConfig, write_default_config};
use ctx_graph::{GraphStore, MemoryDirective, SnippetHit, SymbolHit};
use ctx_intake::{Intent, QueryIntake};
use ctx_pack::{PackInput, PackResult, build_pack};
use ctx_prune::{PruneReport, prune_diff, prune_logs};
use ctx_semantic::{ChunkCandidate, SemanticBackendKind, SemanticEngineConfig, rank_chunks};
use ctx_telemetry::{
    GainReport, PrivacyAuditEvent, StatsSnapshot, append_audit_line, append_privacy_audit_event,
    build_gain_report, write_latest_stats,
};
use ctx_token::estimate_tokens;
use regex::Regex;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

pub use command_run::CommandRunReport;
use command_run::run_command_capture;
use index_cache::{
    IndexCachePaths, IndexCacheReport, IndexedFileEntry, compute_fingerprint,
    load_index_cache_state, load_latest_index_cache_report, save_index_cache_state,
    write_index_cache_report,
};
use path_filters::{PathMatcher, SegmentMatcher};
use read_cache::run_cached_read;
pub use read_cache::{ReadCacheReport, ReadMode};

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

#[derive(Debug, Clone, Serialize)]
pub struct MemoryDirectiveResult {
    pub key: String,
    pub body: String,
    pub scope: String,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryAbBenchmarkResult {
    pub query: String,
    pub markdown_path: String,
    pub markdown_tokens: usize,
    pub graph_memory_tokens: usize,
    pub token_reduction_pct: f64,
    pub markdown_query_term_coverage: f64,
    pub graph_query_term_coverage: f64,
    pub markdown_directive_lines: usize,
    pub graph_directives_count: usize,
    pub markdown_success_rate: Option<f64>,
    pub graph_success_rate: Option<f64>,
    pub quality_winner: Option<String>,
    pub quality_delta_pct: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemoryBenchmarkSuiteSpec {
    #[serde(default)]
    pub title: Option<String>,
    pub cases: Vec<MemoryBenchmarkCaseSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemoryBenchmarkCaseSpec {
    pub name: String,
    pub query: String,
    pub markdown: PathBuf,
    #[serde(default = "default_memory_benchmark_limit")]
    pub limit: usize,
    #[serde(default)]
    pub checklist: Option<PathBuf>,
    #[serde(default)]
    pub markdown_answer: Option<PathBuf>,
    #[serde(default)]
    pub graph_answer: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryBenchmarkSuiteCaseReport {
    pub case_name: String,
    pub latency_ms: u64,
    pub result: MemoryAbBenchmarkResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryBenchmarkSuiteSummary {
    pub case_count: usize,
    pub avg_token_reduction_pct: f64,
    pub avg_markdown_query_term_coverage: f64,
    pub avg_graph_query_term_coverage: f64,
    pub avg_latency_ms: f64,
    pub avg_markdown_success_rate: Option<f64>,
    pub avg_graph_success_rate: Option<f64>,
    pub markdown_quality_wins: usize,
    pub graph_quality_wins: usize,
    pub ties: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryBenchmarkSuiteReport {
    pub title: String,
    pub spec_path: String,
    pub report_markdown_path: String,
    pub json_output_path: Option<String>,
    pub summary: MemoryBenchmarkSuiteSummary,
    pub cases: Vec<MemoryBenchmarkSuiteCaseReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryImportReport {
    pub markdown_path: String,
    pub scope: String,
    pub source: String,
    pub imported: usize,
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryBootstrapReport {
    pub scope: String,
    pub source: String,
    pub scanned_paths: Vec<String>,
    pub imported_files: usize,
    pub imported_directives: usize,
    pub reports: Vec<MemoryImportReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryExportReport {
    pub output_path: String,
    pub scope: Option<String>,
    pub directives: usize,
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

pub fn run_command(repo_root: &Path, command: &str) -> Result<CommandRunReport> {
    let cfg = load_or_default_config(repo_root)?;
    run_command_capture(repo_root, command, cfg.pruning.max_log_lines)
}

pub fn run_read(repo_root: &Path, path: &str, mode: ReadMode) -> Result<ReadCacheReport> {
    let cfg = load_or_default_config(repo_root)?;
    run_cached_read(
        repo_root,
        path,
        mode,
        cfg.security.exclude_sensitive_files,
        &cfg.security.sensitive_patterns,
        &cfg.security.ignored_files,
    )
}

pub fn run_pack(
    repo_root: &Path,
    query: &str,
    budget: Option<usize>,
    attach: Option<&Path>,
) -> Result<PackResult> {
    let cfg = load_or_default_config(repo_root)?;
    let max_lines = cfg.pruning.max_log_lines;
    let sensitive_files = PathMatcher::contains_or_glob(&cfg.security.sensitive_patterns);

    let root_cause = if let Some(path) = attach {
        if cfg.security.exclude_sensitive_files
            && sensitive_files.matches_path(Some(repo_root), path)
        {
            audit_privacy_decision(
                repo_root,
                &cfg,
                "excluded",
                Some(path),
                "sensitive_pattern",
                "blocked sensitive attachment before packing",
            );
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
    let mut tests = Vec::new();
    let mut docs = Vec::new();
    for hit in &retrieved {
        if hit.source == "symbol" && is_test_context(&hit.content) {
            tests.push(hit.content.clone());
        } else if hit.source == "symbol" {
            symbols.push(hit.content.clone());
        } else {
            docs.push(hit.content.clone());
        }
    }

    let (dependencies, task_memory, failure_memory, memory) = if cfg.graph.enabled {
        (
            load_immediate_dependencies(repo_root, &cfg, query, 10).unwrap_or_default(),
            load_task_memory(repo_root, &cfg, 8).unwrap_or_default(),
            load_failure_memory(repo_root, &cfg, 8).unwrap_or_default(),
            load_memory_context(repo_root, &cfg, query, 12).unwrap_or_default(),
        )
    } else {
        (Vec::new(), Vec::new(), Vec::new(), Vec::new())
    };

    let recent_diff = load_recent_diff(repo_root, query, max_lines).unwrap_or_default();

    let pack_input = PackInput {
        query: query.to_string(),
        error_root_cause: root_cause,
        symbols,
        tests,
        recent_diff,
        dependencies,
        task_memory,
        failure_memory,
        memory,
        docs,
        budget: budget.unwrap_or(cfg.general.default_budget),
    };

    let mut pack_result = build_pack(&pack_input);
    if let Some(index_report) = load_latest_index_cache_report(repo_root)? {
        pack_result.included.push(format!(
            "index_cache included: scanned_files={} indexed_files={} reused_files={} changed_files={} new_files={}",
            index_report.scanned_files,
            index_report.indexed_files,
            index_report.reused_files,
            index_report.changed_files,
            index_report.new_files
        ));
    }

    let result = write_pack_artifact(repo_root, pack_result)?;

    let stats = StatsSnapshot {
        original_tokens: result.original_estimated_tokens,
        packed_tokens: result.packed_tokens,
        reduction_pct: result.reduction_pct,
        latency_ms: 0,
        agent: None,
        command: None,
        status: None,
        exit_code: None,
        fallback_used: false,
        pack_path: result.pack_path.clone(),
        query: Some(query.to_string()),
    };
    if cfg.security.local_stats_enabled {
        let _ = write_latest_stats(&repo_root.join(".ctx/stats"), &stats);
    }
    let _ = append_audit_entry(
        repo_root,
        &format!(
            "run_pack query=\"{}\" packed_tokens={} reduction_pct={:.2} included={} excluded={}",
            query,
            result.packed_tokens,
            result.reduction_pct,
            result.included.len(),
            result.excluded.len()
        ),
    );

    Ok(result)
}

pub fn run_gain(repo_root: &Path, limit: usize) -> Result<GainReport> {
    build_gain_report(&repo_root.join(".ctx/stats"), limit)
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
    run_index_internal(repo_root, include_paths, false)
}

pub fn run_reindex(repo_root: &Path, include_paths: &[String]) -> Result<usize> {
    run_index_internal(repo_root, include_paths, true)
}

fn run_index_internal(
    repo_root: &Path,
    include_paths: &[String],
    prune_stale: bool,
) -> Result<usize> {
    let cfg = load_or_default_config(repo_root)?;
    if !cfg.graph.enabled {
        bail!("graph is disabled in config")
    }

    let mut store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;

    let roots: Vec<PathBuf> = if include_paths.is_empty() {
        vec![repo_root.to_path_buf()]
    } else {
        include_paths.iter().map(|p| repo_root.join(p)).collect()
    };
    let ignored_dirs = SegmentMatcher::new(&cfg.security.ignored_dirs);
    let ignored_files = PathMatcher::exact_or_glob(&cfg.security.ignored_files);
    let sensitive_files = PathMatcher::contains_or_glob(&cfg.security.sensitive_patterns);

    let mut indexed = 0usize;
    let mut indexed_files = Vec::new();
    let previous_state = load_index_cache_state(repo_root)?;
    let mut next_state = previous_state.clone();
    next_state.files.clear();
    let mut report = IndexCacheReport::default();
    let mut scanned_relative_paths = HashSet::new();
    for root in roots {
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !ignored_dirs.matches_path(e.path()))
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if !is_code_file(path) {
                continue;
            }
            if ignored_files.matches_path(Some(repo_root), path) {
                audit_privacy_decision(
                    repo_root,
                    &cfg,
                    "excluded",
                    Some(path),
                    "ignored_file_pattern",
                    "skipped ignored file during indexing",
                );
                continue;
            }
            if cfg.security.exclude_sensitive_files
                && sensitive_files.matches_path(Some(repo_root), path)
            {
                audit_privacy_decision(
                    repo_root,
                    &cfg,
                    "excluded",
                    Some(path),
                    "sensitive_pattern",
                    "skipped sensitive code file during indexing",
                );
                continue;
            }

            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };
            let rel = path
                .strip_prefix(repo_root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            scanned_relative_paths.insert(rel.clone());
            let fingerprint = compute_fingerprint(&content);
            let unchanged = previous_state
                .files
                .get(&rel)
                .map(|entry| entry.fingerprint == fingerprint)
                .unwrap_or(false);

            report.scanned_files += 1;
            next_state.files.insert(
                rel.clone(),
                IndexedFileEntry {
                    fingerprint,
                    bytes: content.len(),
                },
            );

            if unchanged {
                report.reused_files += 1;
                continue;
            }

            let _ = store.remove_file(&rel);

            store.index_file(&rel)?;
            upsert_symbols_and_snippets(&store, &rel, &content)?;
            indexed_files.push((rel.clone(), content));
            indexed += 1;
            report.indexed_files += 1;
            if previous_state.files.contains_key(&rel) {
                report.changed_files += 1;
            } else {
                report.new_files += 1;
            }
        }
    }

    if prune_stale {
        for rel in previous_state.files.keys() {
            if scanned_relative_paths.contains(rel) {
                continue;
            }
            if !should_prune_stale_entry(rel, include_paths) {
                if let Some(entry) = previous_state.files.get(rel) {
                    next_state.files.insert(rel.clone(), entry.clone());
                }
                continue;
            }
            let _ = store.remove_file(rel);
        }
    } else {
        for (rel, entry) in &previous_state.files {
            if !next_state.files.contains_key(rel) {
                next_state.files.insert(rel.clone(), entry.clone());
            }
        }
    }

    for (file_path, content) in &indexed_files {
        index_symbol_edges(&store, file_path, content)?;
    }

    let paths = persist_index_cache(repo_root, &next_state, &report)?;
    let _ = append_audit_entry(
        repo_root,
        &format!(
            "run_index scanned_files={} indexed_files={} reused_files={} changed_files={} new_files={} state_path={} report_path={}",
            report.scanned_files,
            report.indexed_files,
            report.reused_files,
            report.changed_files,
            report.new_files,
            paths.state_path,
            paths.report_path
        ),
    );

    Ok(indexed)
}

fn should_prune_stale_entry(rel_path: &str, include_paths: &[String]) -> bool {
    if include_paths.is_empty() {
        return true;
    }

    let normalized_rel = rel_path.replace('\\', "/");
    include_paths.iter().any(|path| {
        let normalized = path.trim_matches('/').replace('\\', "/");
        normalized_rel == normalized || normalized_rel.starts_with(&format!("{normalized}/"))
    })
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

    let ranked = rank_chunks(
        query,
        &candidates,
        semantic_engine_config(repo_root, &cfg, top_k),
    )?;

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

pub fn run_memory_set(
    repo_root: &Path,
    key: &str,
    body: &str,
    scope: &str,
    source: &str,
) -> Result<MemoryDirectiveResult> {
    let cfg = load_or_default_config(repo_root)?;
    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;
    store.upsert_memory_directive(key, body, scope, source)?;
    let directive = store
        .get_memory_directive(key)?
        .context("memory directive should exist after upsert")?;
    Ok(map_memory_directive(directive))
}

pub fn run_memory_get(repo_root: &Path, key: &str) -> Result<Option<MemoryDirectiveResult>> {
    let cfg = load_or_default_config(repo_root)?;
    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;
    Ok(store.get_memory_directive(key)?.map(map_memory_directive))
}

pub fn run_memory_list(
    repo_root: &Path,
    scope: Option<&str>,
    limit: usize,
) -> Result<Vec<MemoryDirectiveResult>> {
    let cfg = load_or_default_config(repo_root)?;
    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;
    Ok(store
        .list_memory_directives(scope, limit.max(1))?
        .into_iter()
        .map(map_memory_directive)
        .collect())
}

pub fn run_memory_search(
    repo_root: &Path,
    query: &str,
    scope: Option<&str>,
    limit: usize,
) -> Result<Vec<MemoryDirectiveResult>> {
    let cfg = load_or_default_config(repo_root)?;
    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;

    let mut directives = store.search_memory_directives(query, 500)?;
    if let Some(scope_filter) = scope {
        directives.retain(|directive| directive.scope == scope_filter);
    }

    Ok(directives
        .into_iter()
        .take(limit.max(1))
        .map(map_memory_directive)
        .collect())
}

pub fn run_memory_delete(repo_root: &Path, key: &str) -> Result<bool> {
    let cfg = load_or_default_config(repo_root)?;
    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;
    store.delete_memory_directive(key)
}

pub fn run_memory_ab_benchmark(
    repo_root: &Path,
    query: &str,
    markdown_path: &Path,
    limit: usize,
    checklist_path: Option<&Path>,
    markdown_answer_path: Option<&Path>,
    graph_answer_path: Option<&Path>,
) -> Result<MemoryAbBenchmarkResult> {
    let markdown = fs::read_to_string(markdown_path)
        .with_context(|| format!("failed to read markdown file {}", markdown_path.display()))?;
    let markdown_tokens = estimate_tokens(&markdown);
    let markdown_directive_lines = markdown_directive_lines(&markdown);
    let markdown_query_term_coverage = query_term_coverage(query, &markdown);

    let memory_items = run_memory_list(repo_root, None, limit.max(1))?;
    let memory_blob = memory_items
        .iter()
        .map(|m| format!("[{}:{}:{}] {}", m.scope, m.source, m.key, m.body))
        .collect::<Vec<_>>()
        .join("\n");

    let graph_memory_tokens = estimate_tokens(&memory_blob);
    let graph_query_term_coverage = query_term_coverage(query, &memory_blob);
    let graph_directives_count = memory_items.len();

    let token_reduction_pct = if markdown_tokens == 0 {
        0.0
    } else {
        (1.0 - graph_memory_tokens as f64 / markdown_tokens as f64) * 100.0
    };

    let checklist = if let Some(path) = checklist_path {
        load_checklist(path)?
    } else {
        Vec::new()
    };

    let markdown_success_rate = if checklist.is_empty() {
        None
    } else {
        Some(answer_success_rate(markdown_answer_path, &checklist)?)
    };

    let graph_success_rate = if checklist.is_empty() {
        None
    } else {
        Some(answer_success_rate(graph_answer_path, &checklist)?)
    };

    let (quality_winner, quality_delta_pct) = match (markdown_success_rate, graph_success_rate) {
        (Some(md), Some(gr)) => {
            let winner = if gr > md {
                Some("graph".to_string())
            } else if md > gr {
                Some("markdown".to_string())
            } else {
                Some("tie".to_string())
            };
            let delta = if md == 0.0 && gr == 0.0 {
                Some(0.0)
            } else {
                Some((gr - md) * 100.0)
            };
            (winner, delta)
        }
        _ => (None, None),
    };

    Ok(MemoryAbBenchmarkResult {
        query: query.to_string(),
        markdown_path: markdown_path.display().to_string(),
        markdown_tokens,
        graph_memory_tokens,
        token_reduction_pct,
        markdown_query_term_coverage,
        graph_query_term_coverage,
        markdown_directive_lines,
        graph_directives_count,
        markdown_success_rate,
        graph_success_rate,
        quality_winner,
        quality_delta_pct,
    })
}

pub fn run_memory_ab_benchmark_suite(
    repo_root: &Path,
    spec_path: &Path,
    report_markdown_path: &Path,
    json_output_path: Option<&Path>,
) -> Result<MemoryBenchmarkSuiteReport> {
    let raw = fs::read_to_string(spec_path)
        .with_context(|| format!("failed to read benchmark spec {}", spec_path.display()))?;
    let spec: MemoryBenchmarkSuiteSpec = toml::from_str(&raw)
        .with_context(|| format!("failed to parse benchmark spec {}", spec_path.display()))?;

    if spec.cases.is_empty() {
        bail!(
            "benchmark spec {} does not contain any cases",
            spec_path.display()
        );
    }

    let spec_dir = spec_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| repo_root.to_path_buf());
    let mut case_reports = Vec::new();
    let mut markdown_success_values = Vec::new();
    let mut graph_success_values = Vec::new();
    let mut markdown_quality_wins = 0usize;
    let mut graph_quality_wins = 0usize;
    let mut ties = 0usize;

    for case in spec.cases {
        let markdown = resolve_spec_path(&spec_dir, &case.markdown);
        let checklist = case
            .checklist
            .as_ref()
            .map(|path| resolve_spec_path(&spec_dir, path));
        let markdown_answer = case
            .markdown_answer
            .as_ref()
            .map(|path| resolve_spec_path(&spec_dir, path));
        let graph_answer = case
            .graph_answer
            .as_ref()
            .map(|path| resolve_spec_path(&spec_dir, path));

        let started = Instant::now();
        let result = run_memory_ab_benchmark(
            repo_root,
            &case.query,
            &markdown,
            case.limit,
            checklist.as_deref(),
            markdown_answer.as_deref(),
            graph_answer.as_deref(),
        )?;
        let latency_ms = started.elapsed().as_millis() as u64;

        if let Some(rate) = result.markdown_success_rate {
            markdown_success_values.push(rate);
        }
        if let Some(rate) = result.graph_success_rate {
            graph_success_values.push(rate);
        }

        match result.quality_winner.as_deref() {
            Some("markdown") => markdown_quality_wins += 1,
            Some("graph") => graph_quality_wins += 1,
            Some("tie") => ties += 1,
            _ => {}
        }

        case_reports.push(MemoryBenchmarkSuiteCaseReport {
            case_name: case.name,
            latency_ms,
            result,
        });
    }

    let case_count = case_reports.len();
    let case_count_f64 = case_count as f64;
    let avg_token_reduction_pct = case_reports
        .iter()
        .map(|case| case.result.token_reduction_pct)
        .sum::<f64>()
        / case_count_f64;
    let avg_markdown_query_term_coverage = case_reports
        .iter()
        .map(|case| case.result.markdown_query_term_coverage)
        .sum::<f64>()
        / case_count_f64;
    let avg_graph_query_term_coverage = case_reports
        .iter()
        .map(|case| case.result.graph_query_term_coverage)
        .sum::<f64>()
        / case_count_f64;
    let avg_latency_ms = case_reports
        .iter()
        .map(|case| case.latency_ms as f64)
        .sum::<f64>()
        / case_count_f64;

    let summary = MemoryBenchmarkSuiteSummary {
        case_count,
        avg_token_reduction_pct,
        avg_markdown_query_term_coverage,
        avg_graph_query_term_coverage,
        avg_latency_ms,
        avg_markdown_success_rate: average_optional(&markdown_success_values),
        avg_graph_success_rate: average_optional(&graph_success_values),
        markdown_quality_wins,
        graph_quality_wins,
        ties,
    };

    let report = MemoryBenchmarkSuiteReport {
        title: spec
            .title
            .unwrap_or_else(|| "CTX Memory Benchmark".to_string()),
        spec_path: spec_path.display().to_string(),
        report_markdown_path: report_markdown_path.display().to_string(),
        json_output_path: json_output_path.map(|path| path.display().to_string()),
        summary,
        cases: case_reports,
    };

    write_text_file(
        report_markdown_path,
        &render_memory_benchmark_suite_markdown(&report),
    )?;

    if let Some(path) = json_output_path {
        write_text_file(
            path,
            &format!("{}\n", serde_json::to_string_pretty(&report)?),
        )?;
    }

    Ok(report)
}

pub fn run_memory_import_markdown(
    repo_root: &Path,
    markdown_path: &Path,
    scope: &str,
    source: &str,
    key_prefix: Option<&str>,
) -> Result<MemoryImportReport> {
    let cfg = load_or_default_config(repo_root)?;
    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;

    let markdown = fs::read_to_string(markdown_path)
        .with_context(|| format!("failed to read markdown file {}", markdown_path.display()))?;
    let directives = parse_markdown_directives(&markdown);

    let prefix = key_prefix
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            markdown_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("memory")
                .to_lowercase()
        });

    let prefix = slugify(&prefix);
    store.delete_memory_directives_by_prefix(&prefix)?;

    let mut keys = Vec::new();
    for (idx, body) in directives.iter().enumerate() {
        let key = format!("{}.{}", prefix, idx + 1);
        store.upsert_memory_directive(&key, body, scope, source)?;
        keys.push(key);
    }

    Ok(MemoryImportReport {
        markdown_path: markdown_path.display().to_string(),
        scope: scope.to_string(),
        source: source.to_string(),
        imported: keys.len(),
        keys,
    })
}

pub fn run_memory_bootstrap_markdown(
    repo_root: &Path,
    markdown_paths: &[PathBuf],
    scope: &str,
    source: &str,
) -> Result<MemoryBootstrapReport> {
    let candidate_paths = if markdown_paths.is_empty() {
        default_memory_markdown_paths(repo_root)
    } else {
        markdown_paths
            .iter()
            .map(|path| {
                if path.is_absolute() {
                    path.clone()
                } else {
                    repo_root.join(path)
                }
            })
            .collect::<Vec<_>>()
    };

    let scanned_paths = candidate_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    let existing_paths = candidate_paths
        .into_iter()
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();

    let mut reports = Vec::new();
    let mut imported_directives = 0usize;
    for path in existing_paths {
        let report = run_memory_import_markdown(repo_root, &path, scope, source, None)?;
        imported_directives += report.imported;
        reports.push(report);
    }

    Ok(MemoryBootstrapReport {
        scope: scope.to_string(),
        source: source.to_string(),
        scanned_paths,
        imported_files: reports.len(),
        imported_directives,
        reports,
    })
}

pub fn run_memory_export_markdown(
    repo_root: &Path,
    output_path: &Path,
    scope: Option<&str>,
    limit: usize,
    title: Option<&str>,
) -> Result<MemoryExportReport> {
    let directives = run_memory_list(repo_root, scope, limit.max(1))?;
    let header = title.unwrap_or("Graph Memory Directives");

    let mut lines = vec![format!("# {header}")];
    for item in &directives {
        lines.push(format!(
            "- [{}:{}:{}] {}",
            item.scope, item.source, item.key, item.body
        ));
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(output_path, lines.join("\n") + "\n")
        .with_context(|| format!("failed to write {}", output_path.display()))?;

    Ok(MemoryExportReport {
        output_path: output_path.display().to_string(),
        scope: scope.map(ToOwned::to_owned),
        directives: directives.len(),
    })
}

fn upsert_symbols_and_snippets(store: &GraphStore, file_path: &str, content: &str) -> Result<()> {
    let symbols = extract_symbols(content, file_path);
    if symbols.is_empty() {
        return Ok(());
    }

    for symbol in &symbols {
        let kind = kind_to_str(&symbol.kind);
        let _ = store.upsert_symbol(file_path, &symbol.name, kind, &symbol.signature)?;

        let slices = slice_symbols(content, file_path, &[symbol.name.as_str()]);
        if let Some(slice) = slices.first() {
            let snippet = slice.content.trim();
            if !snippet.is_empty() {
                let _ = store.add_snippet(file_path, Some(&symbol.name), snippet);
            }
        }
    }

    Ok(())
}

fn index_symbol_edges(store: &GraphStore, file_path: &str, content: &str) -> Result<()> {
    let symbols = extract_symbols(content, file_path);
    if symbols.is_empty() {
        return Ok(());
    }

    let local_ids = symbols
        .iter()
        .map(|symbol| {
            Ok((
                symbol.name.clone(),
                store
                    .find_symbols_by_exact_name(&symbol.name, 20)?
                    .into_iter()
                    .find(|hit| hit.file_path == file_path && hit.kind == kind_to_str(&symbol.kind))
                    .map(|hit| hit.id),
            ))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    for symbol in &symbols {
        if !matches!(symbol.kind, SymbolKind::Function | SymbolKind::Test) {
            continue;
        }

        let caller_id = if let Some(Some(id)) = local_ids.get(&symbol.name) {
            *id
        } else {
            continue;
        };

        let slices = slice_symbols(content, file_path, &[symbol.name.as_str()]);
        let body = slices
            .first()
            .map(|s| s.content.as_str())
            .unwrap_or_default();

        for candidate_name in referenced_symbol_names(body) {
            if candidate_name == symbol.name {
                continue;
            }

            for candidate in store.find_symbols_by_exact_name(&candidate_name, 20)? {
                if candidate.id == caller_id || candidate.kind == "import" {
                    continue;
                }
                let edge_type = if matches!(symbol.kind, SymbolKind::Test) {
                    "tests"
                } else {
                    "calls"
                };
                let metadata = if candidate.file_path != file_path {
                    Some(r#"{"scope":"cross_file"}"#)
                } else {
                    None
                };
                let _ = store.link_symbols(caller_id, candidate.id, edge_type, metadata);
            }
        }
    }

    Ok(())
}

fn referenced_symbol_names(body: &str) -> Vec<String> {
    let call_regex = Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(").expect("valid call regex");
    let mut seen = HashSet::new();
    let mut names = Vec::new();
    for capture in call_regex.captures_iter(body) {
        let Some(name) = capture.get(1).map(|m| m.as_str()) else {
            continue;
        };
        if is_language_keyword(name) {
            continue;
        }
        if seen.insert(name.to_string()) {
            names.push(name.to_string());
        }
    }
    names
}

fn is_language_keyword(name: &str) -> bool {
    matches!(
        name,
        "if" | "for"
            | "while"
            | "loop"
            | "match"
            | "return"
            | "assert"
            | "expect"
            | "panic"
            | "Some"
            | "None"
            | "Ok"
            | "Err"
            | "true"
            | "false"
    )
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

fn semantic_engine_config(repo_root: &Path, cfg: &CtxConfig, top_k: usize) -> SemanticEngineConfig {
    let backend = if cfg.semantic.enabled {
        SemanticBackendKind::parse(&cfg.semantic.backend).unwrap_or(SemanticBackendKind::LocalHash)
    } else {
        SemanticBackendKind::LocalHash
    };

    SemanticEngineConfig {
        backend,
        model_path: non_empty_config_path(repo_root, &cfg.semantic.model),
        vocab_path: cfg
            .semantic
            .vocab
            .as_deref()
            .and_then(|path| non_empty_config_path(repo_root, path)),
        max_chunks: top_k.max(1).min(cfg.semantic.max_chunks.max(1)),
        adaptive_threshold: true,
        allow_fallback: cfg.semantic.allow_fallback,
    }
}

fn non_empty_config_path(repo_root: &Path, raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "local-mini-embed" {
        return None;
    }

    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(repo_root.join(path))
    }
}

fn is_test_context(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("test") || lower.contains("spec") || lower.contains("tests/")
}

fn load_recent_diff(repo_root: &Path, query: &str, max_lines: usize) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["diff", "--no-ext-diff", "--"])
        .current_dir(repo_root)
        .output();

    let Ok(output) = output else {
        return Ok(None);
    };
    if !output.status.success() {
        return Ok(None);
    }

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    if raw.trim().is_empty() {
        return Ok(None);
    }

    let pruned = run_prune_diff(&raw, query, max_lines);
    if pruned.output.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(pruned.output))
    }
}

fn load_immediate_dependencies(
    repo_root: &Path,
    cfg: &CtxConfig,
    query: &str,
    limit: usize,
) -> Result<Vec<String>> {
    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for term in query_terms(query).into_iter().take(8) {
        for hit in store.search_symbols(&term)?.into_iter().take(4) {
            for related in store.related_symbols(&hit.name, limit.max(1))? {
                let dependency = format!(
                    "{}::{} -> {}::{} ({})",
                    hit.file_path, hit.name, related.file_path, related.name, related.kind
                );
                if !seen.insert(dependency.clone()) {
                    continue;
                }
                out.push(dependency);
                if out.len() >= limit.max(1) {
                    return Ok(out);
                }
            }
        }
    }

    Ok(out)
}

fn load_task_memory(repo_root: &Path, cfg: &CtxConfig, limit: usize) -> Result<Vec<String>> {
    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;
    Ok(store
        .recent_decisions(limit.max(1))?
        .into_iter()
        .map(|decision| format!("decision: {decision}"))
        .collect())
}

fn load_failure_memory(repo_root: &Path, cfg: &CtxConfig, limit: usize) -> Result<Vec<String>> {
    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;
    Ok(store
        .recent_failures(limit.max(1))?
        .into_iter()
        .map(|failure| {
            let root = failure.root_cause.unwrap_or_default();
            if root.is_empty() {
                format!("failure: {}", failure.message)
            } else {
                format!("failure: {} root_cause: {}", failure.message, root)
            }
        })
        .collect())
}

fn write_pack_artifact(repo_root: &Path, result: PackResult) -> Result<PackResult> {
    let packs_dir = repo_root.join(".ctx/packs");
    fs::create_dir_all(&packs_dir)
        .with_context(|| format!("failed to create packs dir {}", packs_dir.display()))?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path = packs_dir.join(format!("pack-{ts}.json"));
    let with_path = result.with_pack_path(path.to_string_lossy().to_string());
    let json = serde_json::to_string_pretty(&with_path).context("failed to serialize pack")?;
    fs::write(&path, json).with_context(|| format!("failed to write pack {}", path.display()))?;
    Ok(with_path)
}

fn persist_index_cache(
    repo_root: &Path,
    state: &index_cache::IndexCacheState,
    report: &IndexCacheReport,
) -> Result<IndexCachePaths> {
    let mut paths = save_index_cache_state(repo_root, state)?;
    let report_paths = write_index_cache_report(repo_root, report)?;
    paths.report_path = report_paths.report_path;
    Ok(paths)
}

fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|part| part.len() > 1)
        .map(|part| part.to_lowercase())
        .collect()
}

fn map_memory_directive(item: MemoryDirective) -> MemoryDirectiveResult {
    MemoryDirectiveResult {
        key: item.key,
        body: item.body,
        scope: item.scope,
        source: item.source,
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

fn default_memory_benchmark_limit() -> usize {
    20
}

fn resolve_spec_path(base_dir: &Path, candidate: &Path) -> PathBuf {
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        base_dir.join(candidate)
    }
}

fn average_optional(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn write_text_file(path: &Path, body: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }
    fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))
}

fn render_memory_benchmark_suite_markdown(report: &MemoryBenchmarkSuiteReport) -> String {
    let mut body = String::new();
    body.push_str(&format!("# {}\n\n", report.title));
    body.push_str("## Summary\n\n");
    body.push_str(&format!("- Cases: {}\n", report.summary.case_count));
    body.push_str(&format!(
        "- Token reduction (avg %): {:.2}\n",
        report.summary.avg_token_reduction_pct
    ));
    body.push_str(&format!(
        "- Query coverage markdown={:.2} graph={:.2}\n",
        report.summary.avg_markdown_query_term_coverage,
        report.summary.avg_graph_query_term_coverage
    ));
    body.push_str(&format!(
        "- Latency (avg ms): {:.2}\n",
        report.summary.avg_latency_ms
    ));
    if let (Some(markdown), Some(graph)) = (
        report.summary.avg_markdown_success_rate,
        report.summary.avg_graph_success_rate,
    ) {
        body.push_str(&format!(
            "- Success rate markdown={:.2} graph={:.2}\n",
            markdown, graph
        ));
    }
    body.push_str(&format!(
        "- Quality wins markdown={} graph={} ties={}\n",
        report.summary.markdown_quality_wins,
        report.summary.graph_quality_wins,
        report.summary.ties
    ));

    body.push_str("\n## Cases\n\n");
    body.push_str("| Case | Token reduction % | Markdown coverage | Graph coverage | Quality winner | Latency ms |\n");
    body.push_str("|---|---:|---:|---:|---|---:|\n");
    for case in &report.cases {
        body.push_str(&format!(
            "| {} | {:.2} | {:.2} | {:.2} | {} | {} |\n",
            case.case_name,
            case.result.token_reduction_pct,
            case.result.markdown_query_term_coverage,
            case.result.graph_query_term_coverage,
            case.result.quality_winner.as_deref().unwrap_or("n/a"),
            case.latency_ms
        ));
    }

    body
}

fn load_memory_context(
    repo_root: &Path,
    cfg: &CtxConfig,
    query: &str,
    limit: usize,
) -> Result<Vec<String>> {
    let store = GraphStore::open(&repo_root.join(&cfg.graph.store))?;
    store.init_schema()?;

    let mut directives = store.search_memory_directives(query, limit.max(1))?;
    if directives.is_empty() {
        directives = store.list_memory_directives(None, limit.max(1))?;
    }

    Ok(directives
        .into_iter()
        .map(|m| format!("[{}:{}:{}] {}", m.scope, m.source, m.key, m.body))
        .collect())
}

fn default_memory_markdown_paths(repo_root: &Path) -> Vec<PathBuf> {
    vec![
        repo_root.join("AGENTS.md"),
        repo_root.join("CLAUDE.md"),
        repo_root.join("CODEX.md"),
        repo_root.join(".github/copilot-instructions.md"),
    ]
}

fn query_term_coverage(query: &str, text: &str) -> f64 {
    let terms = query_terms(query);
    if terms.is_empty() {
        return 1.0;
    }
    let hay = text.to_lowercase();
    let matched = terms
        .iter()
        .filter(|term| hay.contains(term.as_str()))
        .count();
    (matched as f64 / terms.len() as f64).clamp(0.0, 1.0)
}

fn markdown_directive_lines(markdown: &str) -> usize {
    markdown
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let bullet = trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("1. ")
                || trimmed.starts_with("2. ")
                || trimmed.starts_with("3. ");
            let heading = trimmed.starts_with('#');
            if bullet || heading { Some(()) } else { None }
        })
        .count()
}

fn parse_markdown_directives(markdown: &str) -> Vec<String> {
    let mut current_heading = String::new();
    let mut out = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix('#') {
            let heading = rest.trim().trim_start_matches('#').trim();
            if !heading.is_empty() {
                current_heading = heading.to_string();
            }
            continue;
        }

        let body = if let Some(rest) = trimmed
            .strip_prefix("- [ ] ")
            .or_else(|| trimmed.strip_prefix("- [x] "))
            .or_else(|| trimmed.strip_prefix("* [ ] "))
            .or_else(|| trimmed.strip_prefix("* [x] "))
            .or_else(|| trimmed.strip_prefix("- "))
            .or_else(|| trimmed.strip_prefix("* "))
        {
            rest.trim()
        } else {
            continue;
        };

        if body.is_empty() {
            continue;
        }

        if current_heading.is_empty() {
            out.push(body.to_string());
        } else {
            out.push(format!("{current_heading}: {body}"));
        }
    }

    out
}

fn slugify(input: &str) -> String {
    let lowered = input.to_lowercase();
    let mut out = String::new();
    let mut last_dot = false;
    for c in lowered.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dot = false;
        } else if !last_dot {
            out.push('.');
            last_dot = true;
        }
    }
    let cleaned = out.trim_matches('.').to_string();
    if cleaned.is_empty() {
        "memory".to_string()
    } else {
        cleaned
    }
}

fn load_checklist(path: &Path) -> Result<Vec<String>> {
    let body = fs::read_to_string(path)
        .with_context(|| format!("failed to read checklist {}", path.display()))?;
    let items = body
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            trimmed
                .strip_prefix("- [ ] ")
                .or_else(|| trimmed.strip_prefix("- [x] "))
                .or_else(|| trimmed.strip_prefix("* [ ] "))
                .or_else(|| trimmed.strip_prefix("* [x] "))
                .or_else(|| trimmed.strip_prefix("- "))
                .or_else(|| trimmed.strip_prefix("* "))
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();
    Ok(items)
}

fn answer_success_rate(answer_path: Option<&Path>, checklist: &[String]) -> Result<f64> {
    let Some(path) = answer_path else {
        return Ok(0.0);
    };
    let answer = fs::read_to_string(path)
        .with_context(|| format!("failed to read answer {}", path.display()))?
        .to_lowercase();
    if checklist.is_empty() {
        return Ok(1.0);
    }
    let matched = checklist
        .iter()
        .filter(|item| answer.contains(&item.to_lowercase()))
        .count();
    Ok((matched as f64 / checklist.len() as f64).clamp(0.0, 1.0))
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
            | "md"
            | "yml"
            | "yaml"
            | "toml"
            | "sh"
            | "json"
            | "go"
            | "java"
            | "kt"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
    )
}

fn audit_privacy_decision(
    repo_root: &Path,
    cfg: &CtxConfig,
    decision: &str,
    path: Option<&Path>,
    reason: &str,
    message: &str,
) {
    if !cfg.security.audit_include_exclude {
        return;
    }

    let path = path.map(|value| {
        value
            .strip_prefix(repo_root)
            .unwrap_or(value)
            .to_string_lossy()
            .to_string()
    });
    let _ = append_privacy_audit_event(
        &repo_root.join(".ctx/audit.log"),
        &PrivacyAuditEvent {
            kind: "privacy_decision".to_string(),
            decision: decision.to_string(),
            path,
            reason: reason.to_string(),
            local_only: cfg.security.local_only,
            remote_upload_enabled: cfg.security.remote_upload_enabled,
            message: message.to_string(),
        },
    );
}

fn append_audit_entry(repo_root: &Path, line: &str) -> Result<()> {
    append_audit_line(&repo_root.join(".ctx/audit.log"), line)
}
