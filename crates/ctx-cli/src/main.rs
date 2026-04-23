use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use clap::{Args, Parser, Subcommand};
use ctx_adapters::{Agent, execute_invocation, prepare_invocation};
use ctx_core::{
    init_repo, load_or_default_config, run_explain, run_graph_query, run_index, run_pack,
    run_prune_diff, run_prune_logs, run_retrieve,
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

16) ctx stats
What it does: Prints latest local telemetry snapshot from .ctx/stats/latest.json.
Example: ctx stats

Global options:
--repo-root <path>  Use a specific repository root
--budget <n>        Override context token budget
--json              Print JSON output when supported
--attach <file>     Attach diagnostic input file (used by pack/adapters)
"#
}
