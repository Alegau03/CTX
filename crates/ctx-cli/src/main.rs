use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use clap::{Args, Parser, Subcommand};
use ctx_adapters::{Agent, execute_invocation, prepare_invocation};
use ctx_core::{
    init_repo, load_or_default_config, run_explain, run_graph_query, run_index,
    run_memory_ab_benchmark, run_memory_delete, run_memory_export_markdown, run_memory_get,
    run_memory_import_markdown, run_memory_list, run_memory_set, run_pack, run_prune_diff,
    run_prune_logs, run_retrieve,
};
use ctx_mcp::{McpServerConfig, serve_http};

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
                let query = args.query.unwrap_or_default();
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
            )?;
        }
        Commands::Claude { query } => {
            let packed = run_pack(&repo_root, &query, cli.budget, cli.attach.as_deref())?;
            println!("adapter=claude\n{}", packed.compact_context);
        }
        Commands::Opencode { command } => match command {
            OpenCodeCommands::Run { query } => {
                run_adapter_wrapper(
                    &repo_root,
                    Agent::OpenCode,
                    &query,
                    cli.budget,
                    cli.attach.as_deref(),
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

fn run_adapter_wrapper(
    repo_root: &std::path::Path,
    agent: Agent,
    query: &str,
    budget: Option<usize>,
    attach: Option<&std::path::Path>,
) -> Result<()> {
    let packed = run_pack(repo_root, query, budget, attach)?;
    let invocation = prepare_invocation(agent, query, &packed.compact_context);

    match execute_invocation(&invocation) {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(anyhow!(
            "adapter '{}' exited with non-zero status: {status}",
            agent.label()
        )),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            eprintln!(
                "ctx warning: '{}' not found in PATH. Falling back to prepared context output.",
                invocation.program
            );
            println!(
                "adapter={}\ncommand={}\n{}",
                agent.label(),
                invocation.command_preview(),
                invocation.prompt
            );
            Ok(())
        }
        Err(err) => Err(err).with_context(|| {
            format!(
                "failed to execute adapter '{}' via '{}'",
                agent.label(),
                invocation.program
            )
        }),
    }
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

8) ctx prune diff [query]
What it does: Compacts diffs and keeps query-relevant hunks.
Example: git diff | ctx prune diff "refresh token"

9) ctx pack <query> [--json] [--attach file] [--budget n]
What it does: Creates a compact context package under a token budget.
Example: ctx pack "fix failing pytest in auth" --json --attach /tmp/fail.txt

10) ctx explain <query>
What it does: Explains likely relevant context and detected intent.
Example: ctx explain "fix failing pytest in auth"

11) ctx retrieve <query> [--limit n]
What it does: Runs hybrid retrieval (graph + snippets + semantic ranking).
Example: ctx retrieve "refresh token auth failure" --limit 5

12) ctx codex <query>
What it does: Builds compact context and invokes Codex CLI with that context.
Example: ctx codex "review the last diff and find risky changes"

13) ctx claude <query>
What it does: Prepares compact context for Claude adapter flow.
Example: ctx claude "explain why this test is flaky"

14) ctx opencode run <query>
What it does: Builds compact context and invokes OpenCode CLI with that context.
Example: ctx opencode run "implement caching for embeddings"

15) ctx mcp serve [--port p] [--once]
What it does: Starts local MCP-compatible RPC server on localhost.
Example: ctx mcp serve --port 8765
Example: ctx mcp serve --port 8765 --once

16) ctx memory set <key> <body> [--scope s] [--source src]
What it does: Upserts a graph-backed memory directive replacing markdown habit files.
Example: ctx memory set testing.always_run "Run targeted tests before completion" --scope project --source manual

17) ctx memory get <key>
What it does: Reads one memory directive from graph memory.
Example: ctx memory get testing.always_run

18) ctx memory import --from <file> [--scope s] [--source src] [--prefix p]
What it does: Imports markdown habit files (AGENTS/CLAUDE/CODEX) into graph memory directives.
Example: ctx memory import --from AGENTS.md --scope project --source markdown --prefix agents

19) ctx memory export --to <file> [--scope s] [--limit n]
What it does: Exports graph memory directives back to markdown for compatibility or auditing.
Example: ctx memory export --to AGENTS.generated.md --scope project --limit 200

20) ctx memory list [--scope s] [--limit n]
What it does: Lists recent memory directives (optionally filtered by scope).
Example: ctx memory list --scope project --limit 10

21) ctx memory delete <key>
What it does: Deletes one memory directive from graph memory.
Example: ctx memory delete testing.always_run

22) ctx benchmark memory-ab <query> --markdown <file> [--limit n]
What it does: Compares graph memory directives vs markdown rules on token cost, query coverage and optional quality/success via checklist + answer files.
Example: ctx benchmark memory-ab "run tests and fix root cause" --markdown AGENTS.md --limit 20

23) ctx stats
What it does: Prints latest local telemetry snapshot from .ctx/stats/latest.json.
Example: ctx stats

Global options:
--repo-root <path>  Use a specific repository root
--budget <n>        Override context token budget
--json              Print JSON output when supported
--attach <file>     Attach diagnostic input file (used by pack/adapters)
"#
}
