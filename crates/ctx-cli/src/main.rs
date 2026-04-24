use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use clap::{Args, Parser, Subcommand};
use ctx_adapters::Agent;
use ctx_core::{
    init_repo, load_or_default_config, run_agent_invocation, run_explain, run_graph_query,
    run_index, run_memory_ab_benchmark, run_memory_delete, run_memory_export_markdown,
    run_memory_get, run_memory_import_markdown, run_memory_list, run_memory_set, run_pack,
    run_prune_diff, run_prune_logs, run_retrieve,
};
use ctx_hooks::apply_pre_prompt_hook;
use ctx_mcp::{McpServerConfig, serve_http, serve_stdio};

#[derive(Debug, Parser)]
#[command(
    name = "ctx",
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
    Wrap(WrapArgs),
    Explain {
        query: String,
    },
    Retrieve {
        query: String,
        #[arg(long, default_value_t = 8)]
        limit: usize,
    },
    Codex {
        query: String,
    },
    Claude {
        query: String,
    },
    Opencode {
        #[command(subcommand)]
        command: OpenCodeCommands,
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
    Stats,
    Doctor,
    Help,
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
enum OpenCodeCommands {
    Run { query: String },
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
    Export(MemoryExportArgs),
    Get {
        key: String,
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
    #[arg(default_value = "claude")]
    client: String,

    #[arg(long)]
    port: Option<u16>,
}

#[derive(Debug, Args)]
struct WrapArgs {
    agent: String,

    #[arg(long)]
    prompt: String,
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
        Commands::Index { paths } | Commands::Reindex { paths } => {
            let indexed = run_index(&repo_root, &paths)?;
            println!("indexed_files: {indexed}");
        }
        Commands::Graph { command } => match command {
            GraphCommands::Build | GraphCommands::Rebuild => {
                let indexed = run_index(&repo_root, &[])?;
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
        Commands::Wrap(args) => {
            let agent = parse_agent(&args.agent)?;
            run_adapter_wrapper(
                &repo_root,
                agent,
                &args.prompt,
                cli.budget,
                cli.attach.as_deref(),
                cli.json,
            )?;
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
        Commands::Codex { query } => {
            run_adapter_wrapper(
                &repo_root,
                Agent::Codex,
                &query,
                cli.budget,
                cli.attach.as_deref(),
                cli.json,
            )?;
        }
        Commands::Claude { query } => {
            run_adapter_wrapper(
                &repo_root,
                Agent::Claude,
                &query,
                cli.budget,
                cli.attach.as_deref(),
                cli.json,
            )?;
        }
        Commands::Opencode { command } => match command {
            OpenCodeCommands::Run { query } => {
                run_adapter_wrapper(
                    &repo_root,
                    Agent::OpenCode,
                    &query,
                    cli.budget,
                    cli.attach.as_deref(),
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
        },
        Commands::Stats => {
            let stats_path = repo_root.join(".ctx/stats/latest.json");
            if !stats_path.exists() {
                println!("no stats recorded yet");
            } else {
                let body = std::fs::read_to_string(&stats_path)
                    .with_context(|| format!("failed to read {}", stats_path.display()))?;
                println!("{body}");
            }
        }
        Commands::Doctor => {
            println!("{}", render_doctor_report(&repo_root));
        }
        Commands::Help => {
            println!("{}", command_guide());
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

fn parse_agent(raw: &str) -> Result<Agent> {
    match raw.to_ascii_lowercase().as_str() {
        "codex" => Ok(Agent::Codex),
        "claude" | "claude-code" => Ok(Agent::Claude),
        "opencode" | "open-code" => Ok(Agent::OpenCode),
        "generic" => Ok(Agent::Generic),
        other => Err(anyhow!(
            "unknown adapter '{other}'. Expected one of: codex, claude, opencode, generic"
        )),
    }
}

fn render_mcp_config(repo_root: &std::path::Path, client: &str, port: u16) -> Result<String> {
    match client.to_ascii_lowercase().as_str() {
        "claude" | "claude-code" => Ok(serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {
                "ctx": {
                    "command": "ctx",
                    "args": [
                        "--repo-root",
                        repo_root.to_string_lossy(),
                        "mcp",
                        "stdio"
                    ]
                }
            }
        }))?),
        "http" | "generic-http" => Ok(serde_json::to_string_pretty(&serde_json::json!({
            "name": "ctx",
            "transport": "http-json-rpc",
            "url": format!("http://127.0.0.1:{port}/rpc"),
            "health": format!("http://127.0.0.1:{port}/health"),
            "repo_root": repo_root.to_string_lossy()
        }))?),
        other => Err(anyhow!(
            "unknown MCP config client '{other}'. Expected: claude or http"
        )),
    }
}

fn run_adapter_wrapper(
    repo_root: &std::path::Path,
    agent: Agent,
    query: &str,
    budget: Option<usize>,
    attach: Option<&std::path::Path>,
    json: bool,
) -> Result<()> {
    let report = run_agent_invocation(repo_root, agent, query, budget, attach)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if report.fallback_used {
        eprintln!(
            "ctx warning: '{}' not found in PATH. Falling back to prepared context output.",
            agent.label()
        );
        println!(
            "adapter={}\ncommand={}\n{}",
            report.agent,
            report.command,
            report.prompt_preview.unwrap_or_default()
        );
    }

    if report.status == "failed" {
        Err(anyhow!(
            "adapter '{}' exited with non-zero status: {:?}",
            report.agent,
            report.exit_code
        ))
    } else {
        Ok(())
    }
}

fn render_doctor_report(repo_root: &std::path::Path) -> String {
    let config_path = repo_root.join(".ctx/config.toml");
    let graph_path = repo_root.join(".ctx/graph.db");
    let stats_dir = repo_root.join(".ctx/stats");
    let audit_path = repo_root.join(".ctx/audit.log");
    let packs_dir = repo_root.join(".ctx/packs");

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
    } else if !graph_path.is_file() {
        "ctx init"
    } else {
        "ctx index"
    };
    lines.push(format!("next: {next}"));
    lines.join("\n")
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

14) ctx codex <query>
What it does: Builds compact context, runs `codex exec`, and records local invocation telemetry.
Example: ctx codex "review the last diff and find risky changes"

15) ctx claude <query>
What it does: Builds compact context, runs `claude -p`, and records local invocation telemetry.
Example: ctx claude "explain why this test is flaky"

16) ctx opencode run <query>
What it does: Builds compact context, runs `opencode run`, and records local invocation telemetry.
Example: ctx opencode run "implement caching for embeddings"

17) ctx wrap <agent> --prompt <query>
What it does: Generic wrapper entrypoint for codex, claude, opencode, or generic adapters.
Example: ctx wrap claude --prompt "explain why this test is flaky"

18) ctx mcp serve [--port p] [--once]
What it does: Starts local MCP-compatible RPC server on localhost.
Example: ctx mcp serve --port 8765
Example: ctx mcp serve --port 8765 --once

19) ctx mcp stdio
What it does: Runs MCP JSON-RPC over stdin/stdout for clients that launch local MCP commands.
Example: ctx --repo-root /path/to/project mcp stdio

20) ctx mcp config claude
What it does: Prints a Claude Code MCP configuration snippet for this repository.
Example: ctx mcp config claude

21) ctx memory set <key> <body> [--scope s] [--source src]
What it does: Upserts a graph-backed memory directive replacing markdown habit files.
Example: ctx memory set testing.always_run "Run targeted tests before completion" --scope project --source manual

22) ctx memory get <key>
What it does: Reads one memory directive from graph memory.
Example: ctx memory get testing.always_run

23) ctx memory import --from <file> [--scope s] [--source src] [--prefix p]
What it does: Imports markdown habit files (AGENTS/CLAUDE/CODEX) into graph memory directives.
Example: ctx memory import --from AGENTS.md --scope project --source markdown --prefix agents

24) ctx memory export --to <file> [--scope s] [--limit n]
What it does: Exports graph memory directives back to markdown for compatibility or auditing.
Example: ctx memory export --to AGENTS.generated.md --scope project --limit 200

25) ctx memory list [--scope s] [--limit n]
What it does: Lists recent memory directives (optionally filtered by scope).
Example: ctx memory list --scope project --limit 10

26) ctx memory delete <key>
What it does: Deletes one memory directive from graph memory.
Example: ctx memory delete testing.always_run

27) ctx benchmark memory-ab <query> --markdown <file> [--limit n]
What it does: Compares graph memory directives vs markdown rules on token cost, query coverage and optional quality/success via checklist + answer files.
Example: ctx benchmark memory-ab "run tests and fix root cause" --markdown AGENTS.md --limit 20

28) ctx stats
What it does: Prints latest local telemetry snapshot, including token reduction, latency, adapter status, and fallback metadata.
Example: ctx stats

29) ctx doctor
What it does: Checks first-run/install readiness: config, graph, local stats, audit log, and privacy defaults.
Example: ctx doctor

Global options:
--repo-root <path>  Use a specific repository root
--budget <n>        Override context token budget
--json              Print JSON output when supported
--attach <file>     Attach diagnostic input file (used by pack/adapters)
"#
}
