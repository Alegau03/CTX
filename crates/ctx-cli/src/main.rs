use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use ctx_core::{
    ReadMode, init_repo, load_or_default_config, run_command, run_explain, run_gain,
    run_graph_query, run_index, run_memory_ab_benchmark, run_memory_ab_benchmark_suite,
    run_memory_bootstrap_markdown, run_memory_delete, run_memory_export_markdown, run_memory_get,
    run_memory_import_markdown, run_memory_list, run_memory_search, run_memory_set, run_pack,
    run_prune_diff, run_prune_logs, run_read, run_reindex, run_retrieve,
};
use ctx_graph::GraphStore;
use ctx_hooks::apply_pre_prompt_hook;
use ctx_mcp::{McpServerConfig, serve_http, serve_stdio};
mod dashboard;
mod host_integration;
mod update;

use dashboard::{build_dashboard_value, render_dashboard};
use host_integration::{OpencodeInstallProfile, install_opencode_integration, render_mcp_config};
use update::{UpdateArgs, run_update};

#[derive(Debug, Parser)]
#[command(
    name = "ctx",
    version = concat!(env!("CARGO_PKG_VERSION"), env!("CTX_BUILD_INFO")),
    about = "Context Runtime Engine for Coding Agents",
    disable_help_subcommand = true
)]
struct Cli {
    #[arg(long, global = true)]
    repo_root: Option<PathBuf>,

    #[arg(long, global = true)]
    budget: Option<usize>,

    #[arg(long, global = true)]
    json: bool,

    #[arg(long, global = true)]
    attach: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init,
    Index {
        paths: Vec<String>,
    },
    Reindex {
        paths: Vec<String>,
    },
    Graph {
        #[command(subcommand)]
        command: GraphCommands,
    },
    Prune {
        #[command(subcommand)]
        command: PruneCommands,
    },
    Pack {
        query: String,
    },
    Ask {
        query: String,
    },
    Hook {
        query: String,
    },
    Explain {
        query: String,
    },
    Retrieve {
        query: String,
        #[arg(long, default_value_t = 8)]
        limit: usize,
    },
    Opencode {
        #[command(subcommand)]
        command: HostInstallCommands,
    },
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },
    Memory {
        #[command(subcommand)]
        command: MemoryCommands,
    },
    Benchmark {
        #[command(subcommand)]
        command: BenchmarkCommands,
    },
    Stats(StatsArgs),
    Update(UpdateArgs),
    Doctor,
    Menu,
    Help,
    #[command(hide = true)]
    HostRun(HostRunArgs),
    #[command(hide = true)]
    HostRead(HostReadArgs),
    #[command(hide = true)]
    HostDashboard,
}

#[derive(Debug, Subcommand)]
enum GraphCommands {
    Build,
    Rebuild,
    Query { query: String },
}

#[derive(Debug, Subcommand)]
enum PruneCommands {
    Logs(PruneArgs),
    Diff(PruneDiffArgs),
}

#[derive(Debug, Subcommand)]
enum HostInstallCommands {
    Install(OpencodeInstallArgs),
}

#[derive(Debug, Subcommand)]
enum McpCommands {
    Serve(McpServeArgs),
    Stdio,
    Config(McpConfigArgs),
}

#[derive(Debug, Subcommand)]
enum MemoryCommands {
    Set(MemorySetArgs),
    Import(MemoryImportArgs),
    Bootstrap(MemoryBootstrapArgs),
    Export(MemoryExportArgs),
    Get {
        key: String,
    },
    Search {
        query: String,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    List {
        #[arg(long)]
        scope: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    Delete {
        key: String,
    },
}

#[derive(Debug, Subcommand)]
enum BenchmarkCommands {
    MemoryAb {
        query: String,
        #[arg(long)]
        markdown: PathBuf,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        checklist: Option<PathBuf>,
        #[arg(long)]
        markdown_answer: Option<PathBuf>,
        #[arg(long)]
        graph_answer: Option<PathBuf>,
    },
    MemorySuite {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long = "report-out")]
        report_out: PathBuf,
        #[arg(long = "json-out")]
        json_out: Option<PathBuf>,
    },
}

#[derive(Debug, Args)]
struct PruneArgs {
    #[arg(long, default_value_t = 200)]
    max_lines: usize,
}

#[derive(Debug, Args)]
struct PruneDiffArgs {
    query: Option<String>,

    #[arg(long = "query")]
    query_flag: Option<String>,

    #[arg(long, default_value_t = 200)]
    max_lines: usize,
}

#[derive(Debug, Args)]
struct McpServeArgs {
    #[arg(long)]
    port: Option<u16>,

    #[arg(long, default_value_t = false)]
    once: bool,
}

#[derive(Debug, Args)]
struct McpConfigArgs {
    #[arg(default_value = "opencode")]
    client: String,

    #[arg(long)]
    port: Option<u16>,
}

#[derive(Debug, Args)]
struct MemorySetArgs {
    key: String,
    body: String,
    #[arg(long, default_value = "project")]
    scope: String,
    #[arg(long, default_value = "manual")]
    source: String,
}

#[derive(Debug, Args)]
struct MemoryImportArgs {
    #[arg(long)]
    from: PathBuf,
    #[arg(long, default_value = "project")]
    scope: String,
    #[arg(long, default_value = "markdown")]
    source: String,
    #[arg(long)]
    prefix: Option<String>,
}

#[derive(Debug, Args)]
struct MemoryBootstrapArgs {
    paths: Vec<PathBuf>,
    #[arg(long, default_value = "project")]
    scope: String,
    #[arg(long, default_value = "markdown")]
    source: String,
}

#[derive(Debug, Args)]
struct MemoryExportArgs {
    #[arg(long)]
    to: PathBuf,
    #[arg(long)]
    scope: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
    #[arg(long)]
    title: Option<String>,
}

#[derive(Debug, Args)]
struct StatsArgs {
    #[arg(long, default_value_t = 1)]
    history: usize,
}

#[derive(Debug, Args)]
struct HostRunArgs {
    command: String,
}

#[derive(Debug, Args)]
struct HostReadArgs {
    path: String,
    #[arg(long, default_value = "digest")]
    mode: String,
}

#[derive(Debug, Args)]
struct OpencodeInstallArgs {
    #[arg(long, default_value = "full")]
    profile: String,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("ctx error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let repo_root = cli
        .repo_root
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));

    match cli.command {
        Commands::Init => {
            let config_path = init_repo(&repo_root)?;
            println!("initialized: {}", config_path.display());
        }
        Commands::Index { paths } => {
            let indexed = run_index(&repo_root, &paths)?;
            println!("indexed_files: {indexed}");
        }
        Commands::Reindex { paths } => {
            let indexed = run_reindex(&repo_root, &paths)?;
            println!("indexed_files: {indexed}");
        }
        Commands::Graph { command } => match command {
            GraphCommands::Build | GraphCommands::Rebuild => {
                let indexed = run_reindex(&repo_root, &[])?;
                println!("graph_build_indexed_files: {indexed}");
            }
            GraphCommands::Query { query } => {
                let results = run_graph_query(&repo_root, &query)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&results)?);
                } else if results.is_empty() {
                    println!("no graph matches");
                } else {
                    for result in results {
                        println!("{result}");
                    }
                }
            }
        },
        Commands::Prune { command } => match command {
            PruneCommands::Logs(args) => {
                let input = read_stdin_all()?;
                let report = run_prune_logs(&input, args.max_lines);
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    println!("{}", report.output);
                }
            }
            PruneCommands::Diff(args) => {
                let input = read_stdin_all()?;
                let query = args.query_flag.or(args.query).unwrap_or_default();
                let report = run_prune_diff(&input, &query, args.max_lines);
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    println!("{}", report.output);
                }
            }
        },
        Commands::Pack { query } => {
            let result = run_pack(&repo_root, &query, cli.budget, cli.attach.as_deref())?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{}", result.compact_context);
            }
        }
        Commands::Ask { query } => {
            let result = run_pack(&repo_root, &query, cli.budget, cli.attach.as_deref())?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{}", result.compact_context);
            }
        }
        Commands::Hook { query } => {
            let result = run_pack(&repo_root, &query, cli.budget, cli.attach.as_deref())?;
            let hook_prompt = apply_pre_prompt_hook(&query, &result.compact_context);
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "query": query,
                        "hook_prompt": hook_prompt,
                        "packed_tokens": result.packed_tokens,
                        "reduction_pct": result.reduction_pct,
                        "pack_path": result.pack_path,
                    }))?
                );
            } else {
                println!("{hook_prompt}");
            }
        }
        Commands::Explain { query } => {
            let explain = run_explain(&repo_root, &query)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&explain)?);
            } else {
                println!("query: {}", explain.query);
                println!("intent: {}", intent_label(explain.intent));
                if !explain.likely_symbols.is_empty() {
                    println!("likely_symbols:");
                    for symbol in explain.likely_symbols {
                        println!("- {symbol}");
                    }
                }
            }
        }
        Commands::Retrieve { query, limit } => {
            let hits = run_retrieve(&repo_root, &query, limit)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&hits)?);
            } else if hits.is_empty() {
                println!("no retrieval hits");
            } else {
                for hit in hits {
                    println!(
                        "[{}] {:.3} {} => {}",
                        hit.source, hit.score, hit.id, hit.content
                    );
                }
            }
        }
        Commands::Opencode { command } => match command {
            HostInstallCommands::Install(args) => {
                let profile = OpencodeInstallProfile::from_str(&args.profile)?;
                print_host_install_report(
                    install_opencode_integration(&repo_root, profile)?,
                    cli.json,
                )?;
            }
        },
        Commands::Mcp { command } => match command {
            McpCommands::Serve(args) => {
                let cfg = load_or_default_config(&repo_root)?;
                let port = args.port.unwrap_or(cfg.mcp.port);
                serve_http(McpServerConfig {
                    repo_root: repo_root.clone(),
                    port,
                    once: args.once,
                })?;
            }
            McpCommands::Stdio => {
                let cfg = load_or_default_config(&repo_root)?;
                serve_stdio(McpServerConfig {
                    repo_root: repo_root.clone(),
                    port: cfg.mcp.port,
                    once: false,
                })?;
            }
            McpCommands::Config(args) => {
                let cfg = load_or_default_config(&repo_root)?;
                let port = args.port.unwrap_or(cfg.mcp.port);
                println!("{}", render_mcp_config(&repo_root, &args.client, port)?);
            }
        },
        Commands::Memory { command } => match command {
            MemoryCommands::Set(args) => {
                let directive =
                    run_memory_set(&repo_root, &args.key, &args.body, &args.scope, &args.source)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&directive)?);
                } else {
                    println!(
                        "memory directive upserted: key={} scope={} source={}",
                        directive.key, directive.scope, directive.source
                    );
                }
            }
            MemoryCommands::Import(args) => {
                let report = run_memory_import_markdown(
                    &repo_root,
                    &args.from,
                    &args.scope,
                    &args.source,
                    args.prefix.as_deref(),
                )?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    println!(
                        "imported {} directives from {}",
                        report.imported, report.markdown_path
                    );
                }
            }
            MemoryCommands::Export(args) => {
                let report = run_memory_export_markdown(
                    &repo_root,
                    &args.to,
                    args.scope.as_deref(),
                    args.limit,
                    args.title.as_deref(),
                )?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    println!(
                        "exported {} directives to {}",
                        report.directives, report.output_path
                    );
                }
            }
            MemoryCommands::Bootstrap(args) => {
                let report = run_memory_bootstrap_markdown(
                    &repo_root,
                    &args.paths,
                    &args.scope,
                    &args.source,
                )?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    println!(
                        "imported_files={} imported_directives={}",
                        report.imported_files, report.imported_directives
                    );
                    for item in report.reports {
                        println!("- {} => {} directives", item.markdown_path, item.imported);
                    }
                }
            }
            MemoryCommands::Get { key } => {
                let result = run_memory_get(&repo_root, &key)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else if let Some(directive) = result {
                    println!(
                        "key={}\nscope={}\nsource={}\nbody={}",
                        directive.key, directive.scope, directive.source, directive.body
                    );
                } else {
                    println!("memory directive not found");
                }
            }
            MemoryCommands::Search {
                query,
                scope,
                limit,
            } => {
                let items = run_memory_search(&repo_root, &query, scope.as_deref(), limit)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&items)?);
                } else if items.is_empty() {
                    println!("no memory directives");
                } else {
                    for item in items {
                        println!(
                            "{} [{}:{}] {}",
                            item.key, item.scope, item.source, item.body
                        );
                    }
                }
            }
            MemoryCommands::List { scope, limit } => {
                let items = run_memory_list(&repo_root, scope.as_deref(), limit)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&items)?);
                } else if items.is_empty() {
                    println!("no memory directives");
                } else {
                    for item in items {
                        println!(
                            "{} [{}:{}] {}",
                            item.key, item.scope, item.source, item.body
                        );
                    }
                }
            }
            MemoryCommands::Delete { key } => {
                let deleted = run_memory_delete(&repo_root, &key)?;
                if deleted {
                    println!("memory directive deleted: {key}");
                } else {
                    println!("memory directive not found");
                }
            }
        },
        Commands::Benchmark { command } => match command {
            BenchmarkCommands::MemoryAb {
                query,
                markdown,
                limit,
                checklist,
                markdown_answer,
                graph_answer,
            } => {
                let result = run_memory_ab_benchmark(
                    &repo_root,
                    &query,
                    &markdown,
                    limit,
                    checklist.as_deref(),
                    markdown_answer.as_deref(),
                    graph_answer.as_deref(),
                )?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("query: {}", result.query);
                    println!("markdown_path: {}", result.markdown_path);
                    println!("markdown_tokens: {}", result.markdown_tokens);
                    println!("graph_memory_tokens: {}", result.graph_memory_tokens);
                    println!("token_reduction_pct: {:.2}", result.token_reduction_pct);
                    println!(
                        "query_term_coverage markdown={:.2} graph={:.2}",
                        result.markdown_query_term_coverage, result.graph_query_term_coverage
                    );
                    println!(
                        "directive_units markdown_lines={} graph_directives={}",
                        result.markdown_directive_lines, result.graph_directives_count
                    );
                    if let (Some(md), Some(gr)) =
                        (result.markdown_success_rate, result.graph_success_rate)
                    {
                        println!("success_rate markdown={:.2} graph={:.2}", md, gr);
                    }
                    if let Some(winner) = result.quality_winner.as_deref() {
                        let delta = result.quality_delta_pct.unwrap_or(0.0);
                        println!("quality_winner: {} (delta_pct={:.2})", winner, delta);
                    }
                }
            }
            BenchmarkCommands::MemorySuite {
                spec,
                report_out,
                json_out,
            } => {
                let report = run_memory_ab_benchmark_suite(
                    &repo_root,
                    &spec,
                    &report_out,
                    json_out.as_deref(),
                )?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    println!("title: {}", report.title);
                    println!("case_count: {}", report.summary.case_count);
                    println!(
                        "avg_token_reduction_pct: {:.2}",
                        report.summary.avg_token_reduction_pct
                    );
                    println!(
                        "avg_query_coverage markdown={:.2} graph={:.2}",
                        report.summary.avg_markdown_query_term_coverage,
                        report.summary.avg_graph_query_term_coverage
                    );
                    println!(
                        "quality_wins markdown={} graph={} ties={}",
                        report.summary.markdown_quality_wins,
                        report.summary.graph_quality_wins,
                        report.summary.ties
                    );
                    println!("report_markdown_path: {}", report.report_markdown_path);
                    if let Some(path) = report.json_output_path.as_deref() {
                        println!("json_output_path: {path}");
                    }
                }
            }
        },
        Commands::Stats(args) => {
            if args.history > 1 {
                let report = run_gain(&repo_root, args.history)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else if report.sampled_runs == 0 {
                    println!("no stats recorded yet");
                } else {
                    println!("sampled_runs: {}", report.sampled_runs);
                    println!("estimated_tokens_saved: {}", report.estimated_tokens_saved);
                    if let Some(latest) = report.latest_reduction_pct {
                        println!("latest_reduction_pct: {:.2}", latest);
                    }
                    println!("average_reduction_pct: {:.2}", report.average_reduction_pct);
                    println!("max_reduction_pct: {:.2}", report.max_reduction_pct);
                    if let Some(pack_path) = report.latest_pack_path.as_deref() {
                        println!("latest_pack_path: {pack_path}");
                    }
                    for item in report.top_queries {
                        println!(
                            "top_query: {} runs={} avg_reduction_pct={:.2} estimated_tokens_saved={}",
                            item.query,
                            item.runs,
                            item.average_reduction_pct,
                            item.estimated_tokens_saved
                        );
                    }
                }
            } else {
                let stats_path = repo_root.join(".ctx/stats/latest.json");
                if !stats_path.exists() {
                    println!("no stats recorded yet");
                } else {
                    let body = std::fs::read_to_string(&stats_path)
                        .with_context(|| format!("failed to read {}", stats_path.display()))?;
                    println!("{body}");
                }
            }
        }
        Commands::Update(args) => {
            run_update(&args)?;
        }
        Commands::Doctor => {
            println!("{}", render_doctor_report(&repo_root));
        }
        Commands::Menu => {
            println!("{}", render_command_center(&repo_root));
        }
        Commands::Help => {
            println!("{}", command_guide());
        }
        Commands::HostRun(args) => {
            let report = run_command(&repo_root, &args.command)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", report.pruned_output);
                println!(
                    "exit_code={} latency_ms={} raw_log_path={}",
                    report.exit_code, report.latency_ms, report.raw_log_path
                );
            }
        }
        Commands::HostRead(args) => {
            let mode = args.mode.parse::<ReadMode>()?;
            let report = run_read(&repo_root, &args.path, mode)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", report.output);
                println!(
                    "mode={} cache_hit={} fingerprint={} path={}",
                    report.mode, report.cache_hit, report.fingerprint, report.path
                );
            }
        }
        Commands::HostDashboard => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&build_dashboard_value(&repo_root)?)?
                );
            } else {
                println!("{}", render_dashboard(&repo_root)?);
            }
        }
    }

    Ok(())
}

fn read_stdin_all() -> Result<String> {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .context("failed to read stdin")?;
    Ok(buf)
}

fn intent_label(intent: ctx_intake::Intent) -> &'static str {
    match intent {
        ctx_intake::Intent::Debug => "debug",
        ctx_intake::Intent::Refactor => "refactor",
        ctx_intake::Intent::Review => "review",
        ctx_intake::Intent::Explain => "explain",
        ctx_intake::Intent::Ask => "ask",
    }
}

fn print_host_install_report(report: serde_json::Value, as_json: bool) -> Result<()> {
    if as_json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!(
        "installed {} integration",
        report["display_name"].as_str().unwrap_or("CTX host")
    );
    if let Some(profile) = report["profile"].as_str() {
        println!("profile: {profile}");
    }
    println!(
        "config_path: {}",
        report["config_path"].as_str().unwrap_or("")
    );
    if let Some(commands_dir) = report["commands_dir"].as_str() {
        println!("commands_dir: {commands_dir}");
    }
    if let Some(commands_written) = report["commands_written"].as_u64() {
        println!("commands_written: {commands_written}");
    }
    if let Some(sidebar_enabled) = report["sidebar"]["enabled"].as_bool() {
        println!(
            "sidebar_dashboard: {}",
            if sidebar_enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
    if let Some(plugin_path) = report["sidebar"]["plugin_path"].as_str() {
        println!("sidebar_plugin_path: {plugin_path}");
    }
    if let Some(skills_dir) = report["skills_dir"].as_str() {
        println!("skills_dir: {skills_dir}");
    }
    if let Some(skills_written) = report["skills_written"].as_u64() {
        println!("skills_written: {skills_written}");
    }
    if let Some(next) = report["next_step"].as_str() {
        println!("next: {next}");
    }

    Ok(())
}

fn render_doctor_report(repo_root: &Path) -> String {
    let config_path = repo_root.join(".ctx/config.toml");
    let graph_path = repo_root.join(".ctx/graph.db");
    let stats_dir = repo_root.join(".ctx/stats");
    let audit_path = repo_root.join(".ctx/audit.log");
    let packs_dir = repo_root.join(".ctx/packs");
    let indexed_files = indexed_file_count(&graph_path);
    let ready = config_path.is_file()
        && graph_path.is_file()
        && indexed_files.unwrap_or(0) > 0
        && packs_dir.is_dir()
        && stats_dir.is_dir()
        && audit_path.is_file();

    let mut lines = vec![
        "CTX Doctor".to_string(),
        format!("repo_root: {}", repo_root.display()),
        format!("binary: {}", current_binary_label()),
        format!("config: {}", status_label(config_path.is_file())),
        format!("graph: {}", status_label(graph_path.is_file())),
        format!("packs_dir: {}", status_label(packs_dir.is_dir())),
        format!("stats_dir: {}", status_label(stats_dir.is_dir())),
        format!("audit_log: {}", status_label(audit_path.is_file())),
    ];

    if let Some(indexed_files) = indexed_files {
        lines.push(format!("indexed_files: {indexed_files}"));
    }

    match load_or_default_config(repo_root) {
        Ok(cfg) => {
            lines.push(format!("local_only: {}", cfg.security.local_only));
            lines.push(format!(
                "remote_upload_enabled: {}",
                cfg.security.remote_upload_enabled
            ));
            lines.push(format!(
                "anonymous_telemetry_enabled: {}",
                cfg.security.anonymous_telemetry_enabled
            ));
            lines.push(format!(
                "exclude_sensitive_files: {}",
                cfg.security.exclude_sensitive_files
            ));
        }
        Err(err) => {
            lines.push(format!("config_load_error: {err:#}"));
        }
    }

    let next = if !config_path.is_file() {
        "ctx init"
    } else if !graph_path.is_file() || indexed_files.unwrap_or(0) == 0 {
        "ctx index"
    } else if count_memory_directives(&graph_path).unwrap_or(0) == 0 {
        "ctx memory bootstrap"
    } else {
        "ctx pack <task>"
    };
    lines.push(format!("ready: {ready}"));
    lines.push(format!("next: {next}"));
    lines.join("\n")
}

fn render_command_center(repo_root: &Path) -> String {
    let config_path = repo_root.join(".ctx/config.toml");
    let graph_path = repo_root.join(".ctx/graph.db");
    let ready = config_path.is_file() && indexed_file_count(&graph_path).unwrap_or(0) > 0;
    let profile = installed_opencode_profile(repo_root);
    let next = if !config_path.is_file() {
        "ctx init"
    } else if indexed_file_count(&graph_path).unwrap_or(0) == 0 {
        "ctx index"
    } else if count_memory_directives(&graph_path).unwrap_or(0) == 0 {
        "ctx memory bootstrap"
    } else {
        "ctx pack <task>"
    };
    let why = match next {
        "ctx init" => "runtime folders and config are missing",
        "ctx index" => "the graph exists but does not contain indexed source files yet",
        "ctx memory bootstrap" => {
            "the graph is ready, but project rules have not been imported into graph memory yet"
        }
        _ => "the repository is indexed and ready for task-focused context packing",
    };

    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");

    match profile {
        OpencodeInstallProfile::Core => format!(
            "CTX Command Center\n\nrepo: {} | status: {} | profile: core\n\nRecommended Start\n- `/ctx-doctor` - check repo health and next step\n- `/ctx-retrieve <query>` - fetch the smallest useful context slice\n- `/ctx-plan <task>` - turn retrieval and packs into an implementation plan\n- `/ctx-pack <task>` - build the smallest useful context pack\n\nCore Surface\n- `/ctx-doctor`\n- `/ctx-plan <task>`\n- `/ctx-retrieve <query>`\n- `/ctx-pack <task>`\n- `/ctx-run <shell command>`\n- `/ctx-prune-logs <shell command>`\n- `/ctx-stats`\n- `/ctx-gain`\n\nUpgrade\n- rerun `ctx opencode install --profile full` to unlock read cache, memory, toolbooks, dashboard, and benchmarks\n\nBest next command:\n1. `{next}`\n2. copy-paste example: `{next}`\n3. why next: {why}",
            repo_name,
            if ready { "ready" } else { "needs setup" },
        ),
        OpencodeInstallProfile::Full => format!(
            "CTX Command Center\n\nrepo: {} | status: {} | profile: full\n\nRecommended Start\n- `/ctx-doctor` - check repo health and next step\n- `/ctx-index` - build or refresh the graph\n- `/ctx-memory-bootstrap` - import AGENTS-style project rules\n- `/ctx-pack <task>` - build the smallest useful context pack\n\nSetup\n- `/ctx-init`\n- `/ctx-index`\n- `/ctx-reindex`\n- `/ctx-opencode-install`\n\nPlanning\n- `/ctx-plan <task>`\n\nContext\n- `/ctx-pack <task>`\n- `/ctx-compare <task>`\n- `/ctx-ask <task>`\n- `/ctx-retrieve <query>`\n- `/ctx-read <file> [mode]`\n- `/ctx-graph-query <query>`\n- `/ctx-explain <task>`\n\nMemory\n- `/ctx-memory-bootstrap`\n- `/ctx-memory-search <topic>`\n- `/ctx-memory-list`\n- `/ctx-memory-get <key>`\n- `/ctx-memory-set <key> <body>`\n- `/ctx-memory-export <file>`\n\nToolbooks\n- `/ctx-toolbook-import <name> <file>`\n- `/ctx-toolbook-search <name> \"<query>\"`\n- `/ctx-toolbook-list <name>`\n- `/ctx-toolbook-pack <name> \"<task>\"`\n\nLearning\n- `/ctx-learn <key> \"<body>\"`\n\nDebug\n- `/ctx-run <shell command>`\n- `/ctx-prune-logs <shell command>`\n- `/ctx-prune-diff <topic>`\n- `/ctx-hook <task>`\n\nBenchmark\n- `/ctx-dashboard`\n- `/ctx-gain`\n- `/ctx-benchmark-memory-ab ...`\n- `/ctx-benchmark-memory-suite ...`\n- `/ctx-stats`\n\nMCP\n- `/ctx-mcp-stdio`\n- `/ctx-mcp-serve`\n- `/ctx-mcp-config-opencode`\n\nBest next command:\n1. `{next}`\n2. copy-paste example: `{next}`\n3. why next: {why}",
            repo_name,
            if ready { "ready" } else { "needs setup" },
        ),
    }
}

fn installed_opencode_profile(repo_root: &Path) -> OpencodeInstallProfile {
    let marker = repo_root.join(".opencode/ctx-profile.txt");
    let body = std::fs::read_to_string(marker).ok();
    body.as_deref()
        .map(str::trim)
        .and_then(|value| OpencodeInstallProfile::from_str(value).ok())
        .unwrap_or(OpencodeInstallProfile::Full)
}

fn indexed_file_count(graph_path: &Path) -> Option<usize> {
    if !graph_path.is_file() {
        return None;
    }

    let store = GraphStore::open(graph_path).ok()?;
    store.query_files("").ok().map(|files| files.len())
}

fn count_memory_directives(graph_path: &Path) -> Option<usize> {
    if !graph_path.is_file() {
        return None;
    }

    let store = GraphStore::open(graph_path).ok()?;
    store
        .list_memory_directives(None, 10_000)
        .ok()
        .map(|items| items.len())
}

fn current_binary_label() -> String {
    std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn status_label(ok: bool) -> &'static str {
    if ok { "ok" } else { "missing" }
}

fn command_guide() -> &'static str {
    r#"CTX Command Guide

Each command shows what it does and one usage example.

Primary OpenCode path:
- run `ctx opencode install` once in the repo
- use `ctx opencode install --profile core` when you want the lean daily surface first
- open `opencode`
- use `/ctx-*` commands inside OpenCode
- legacy wrapper commands have been removed from the public CLI

1) ctx init
What it does: Initializes local runtime folders, config, and graph database.
Example: ctx init

2) ctx index [paths...]
What it does: Indexes code files, symbols, snippets, and graph links.
Example: ctx index
Example: ctx index src tests

3) ctx reindex [paths...]
What it does: Re-runs indexing for selected paths.
Example: ctx reindex src tests

4) ctx graph build
What it does: Builds graph data by indexing the repository.
Example: ctx graph build

5) ctx graph rebuild
What it does: Alias of graph build for explicit rebuild workflows.
Example: ctx graph rebuild

6) ctx graph query <query>
What it does: Searches indexed graph file paths by keyword.
Example: ctx graph query auth

7) ctx prune logs
What it does: Removes repetitive/noisy log lines and keeps diagnostic signal.
Example: pytest -q 2>&1 | ctx prune logs

8) ctx prune diff [query] [--query q]
What it does: Compacts diffs and keeps query-relevant hunks.
Example: git diff | ctx prune diff --query "refresh token"

9) ctx pack <query> [--json] [--attach file] [--budget n]
What it does: Creates an advanced compact context package with strict priorities, included/excluded reasons and a saved pack artifact.
Example: ctx pack "fix failing pytest in auth" --json --attach /tmp/fail.txt

10) ctx ask <query>
What it does: Builds compact context for a human or agent without invoking a specific CLI.
Example: ctx ask "where is retry logic implemented?"

11) ctx hook <query>
What it does: Produces a pre-prompt payload for agent hook/preprocessing scripts.
Example: ctx hook "fix flaky auth test"

12) ctx explain <query>
What it does: Explains likely relevant context and detected intent.
Example: ctx explain "fix failing pytest in auth"

13) ctx retrieve <query> [--limit n]
What it does: Runs hybrid retrieval (graph + snippets + semantic ranking).
Example: ctx retrieve "refresh token auth failure" --limit 5

14) ctx opencode install [--profile full|core]
What it does: Primary OpenCode bootstrap. Writes `opencode.json` and `.opencode/commands/*.md` for host-native CTX usage.
Example: ctx opencode install
Example: ctx opencode install --profile core

15) ctx mcp serve [--port p] [--once]
What it does: Starts local MCP-compatible RPC server on localhost.
Example: ctx mcp serve --port 8765
Example: ctx mcp serve --port 8765 --once

16) ctx mcp stdio
What it does: Runs MCP JSON-RPC over stdin/stdout for clients that launch local MCP commands.
Example: ctx --repo-root /path/to/project mcp stdio

17) ctx mcp config <client>
What it does: Prints an MCP configuration snippet for OpenCode or a generic HTTP JSON-RPC client.
Example: ctx mcp config opencode
Example: ctx mcp config http

18) ctx memory set <key> <body> [--scope s] [--source src]
What it does: Upserts a graph-backed memory directive replacing markdown habit files.
Example: ctx memory set testing.always_run "Run targeted tests before completion" --scope project --source manual

19) ctx memory get <key>
What it does: Reads one memory directive from graph memory.
Example: ctx memory get testing.always_run

20) ctx memory import --from <file> [--scope s] [--source src] [--prefix p]
What it does: Imports markdown habit files into graph memory directives.
Example: ctx memory import --from AGENTS.md --scope project --source markdown --prefix agents

21) ctx memory bootstrap [paths...] [--scope s] [--source src]
What it does: Auto-imports conventional markdown rule files like `AGENTS.md`, `CLAUDE.md`, `CODEX.md`, and `.github/copilot-instructions.md`.
Example: ctx memory bootstrap
Example: ctx memory bootstrap AGENTS.md CLAUDE.md CODEX.md .github/copilot-instructions.md

22) ctx memory export --to <file> [--scope s] [--limit n]
What it does: Exports graph memory directives back to markdown for compatibility or auditing.
Example: ctx memory export --to AGENTS.generated.md --scope project --limit 200

23) ctx memory search <query> [--scope s] [--limit n]
What it does: Searches graph memory by topic so you can inspect only the relevant directives.
Example: ctx memory search "auth tests root cause" --scope project --limit 10

24) ctx memory list [--scope s] [--limit n]
What it does: Lists recent memory directives (optionally filtered by scope).
Example: ctx memory list --scope project --limit 10

25) ctx memory delete <key>
What it does: Deletes one memory directive from graph memory.
Example: ctx memory delete testing.always_run

26) ctx benchmark memory-ab <query> --markdown <file> [--limit n]
What it does: Compares graph memory directives vs markdown rules on token cost, query coverage and optional quality/success via checklist + answer files.
Example: ctx benchmark memory-ab "run tests and fix root cause" --markdown AGENTS.md --limit 20

27) ctx benchmark memory-suite --spec <file> --report-out <file> [--json-out <file>]
What it does: Runs a reusable benchmark suite from a spec file and writes publishable markdown/JSON reports.
Example: ctx benchmark memory-suite --spec benchmarks/memory-ab.example.toml --report-out benchmarks/report.md --json-out benchmarks/report.json

28) ctx stats [--history n]
What it does: Prints latest local telemetry snapshot, or an aggregate gain report when `--history` is greater than 1.
Example: ctx stats
Example: ctx stats --history 20

29) ctx update [--check] [--yes] [--channel installer|cargo|npm|brew]
What it does: Checks the latest CTX version, detects how CTX was installed when possible, and prints the safest update path for that install channel.
Example: ctx update --check
Example: ctx update --channel cargo

30) ctx doctor
What it does: Checks first-run/install readiness: config, graph, local stats, audit log, and privacy defaults.
Example: ctx doctor

Global options:
--repo-root <path>  Use a specific repository root
--budget <n>        Override context token budget
--json              Print JSON output when supported
--attach <file>     Attach diagnostic input file (used by pack)
"#
}
