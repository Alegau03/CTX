use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde_json::{Map, Value, json};
use toml::Value as TomlValue;

#[derive(Clone, Copy, Debug)]
pub enum HostInstallTarget {
    OpenCode,
    Codex,
    Claude,
}

impl HostInstallTarget {
    pub fn name(self) -> &'static str {
        match self {
            Self::OpenCode => "OpenCode",
            Self::Codex => "Codex",
            Self::Claude => "Claude Code",
        }
    }
}

pub fn render_mcp_config(repo_root: &Path, client: &str, port: u16) -> Result<String> {
    match client.to_ascii_lowercase().as_str() {
        "claude" | "claude-code" => Ok(serde_json::to_string_pretty(&claude_mcp_config_value(
            repo_root,
        ))?),
        "codex" => Ok(render_codex_mcp_config(repo_root)?),
        "opencode" | "open-code" => Ok(serde_json::to_string_pretty(
            &opencode_project_config_value(repo_root),
        )?),
        "http" | "generic-http" => Ok(serde_json::to_string_pretty(&json!({
            "name": "ctx",
            "transport": "http-json-rpc",
            "url": format!("http://127.0.0.1:{port}/rpc"),
            "health": format!("http://127.0.0.1:{port}/health"),
            "repo_root": repo_root.to_string_lossy()
        }))?),
        other => Err(anyhow!(
            "unknown MCP config client '{other}'. Expected: claude, codex, opencode, or http"
        )),
    }
}

pub fn install_host_integration(repo_root: &Path, target: HostInstallTarget) -> Result<Value> {
    match target {
        HostInstallTarget::OpenCode => install_opencode_integration(repo_root),
        HostInstallTarget::Codex => install_codex_integration(repo_root),
        HostInstallTarget::Claude => install_claude_integration(repo_root),
    }
}

fn install_opencode_integration(repo_root: &Path) -> Result<Value> {
    let config_path = upsert_opencode_project_config(repo_root)?;
    let commands_dir = repo_root.join(".opencode/commands");
    write_markdown_assets(
        &commands_dir,
        shared_action_templates(),
        render_opencode_command_file,
    )?;

    let instructions_dir = repo_root.join(".opencode/instructions");
    fs::create_dir_all(&instructions_dir)
        .with_context(|| format!("failed to create {}", instructions_dir.display()))?;

    let mut instruction_paths = Vec::new();
    for (filename, body) in opencode_instruction_files() {
        let path = instructions_dir.join(filename);
        fs::write(&path, body).with_context(|| format!("failed to write {}", path.display()))?;
        instruction_paths.push(path.display().to_string());
    }

    let command_paths = asset_file_paths(&commands_dir, shared_action_templates(), "md");

    Ok(json!({
        "host": "opencode",
        "display_name": HostInstallTarget::OpenCode.name(),
        "config_path": config_path.display().to_string(),
        "commands_dir": commands_dir.display().to_string(),
        "instructions_dir": instructions_dir.display().to_string(),
        "commands_written": command_paths.len(),
        "command_files": command_paths,
        "instruction_files": instruction_paths,
        "next_step": "open this repo in OpenCode and run /ctx-doctor or /ctx-pack <task>"
    }))
}

fn install_codex_integration(repo_root: &Path) -> Result<Value> {
    let config_path = upsert_codex_project_config(repo_root)?;
    let skills_dir = repo_root.join(".agents/skills");
    write_skill_assets(
        &skills_dir,
        shared_action_templates(),
        HostInstallTarget::Codex,
    )?;

    let skill_paths = asset_skill_paths(&skills_dir, shared_action_templates());
    Ok(json!({
        "host": "codex",
        "display_name": HostInstallTarget::Codex.name(),
        "config_path": config_path.display().to_string(),
        "skills_dir": skills_dir.display().to_string(),
        "skills_written": skill_paths.len(),
        "skill_files": skill_paths,
        "next_step": "open this repo in Codex and invoke $ctx-doctor or $ctx-pack for explicit CTX workflows"
    }))
}

fn install_claude_integration(repo_root: &Path) -> Result<Value> {
    let config_path = upsert_claude_project_config(repo_root)?;
    let skills_dir = repo_root.join(".claude/skills");
    write_skill_assets(
        &skills_dir,
        shared_action_templates(),
        HostInstallTarget::Claude,
    )?;

    let skill_paths = asset_skill_paths(&skills_dir, shared_action_templates());
    Ok(json!({
        "host": "claude",
        "display_name": HostInstallTarget::Claude.name(),
        "config_path": config_path.display().to_string(),
        "skills_dir": skills_dir.display().to_string(),
        "skills_written": skill_paths.len(),
        "skill_files": skill_paths,
        "next_step": "open this repo in Claude Code and run /ctx-doctor or /ctx-pack <task>"
    }))
}

fn upsert_opencode_project_config(repo_root: &Path) -> Result<PathBuf> {
    let config_path = repo_root.join("opencode.json");
    let mut root = if config_path.is_file() {
        let raw = fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;
        serde_json::from_str::<Value>(&raw)
            .with_context(|| format!("failed to parse {}", config_path.display()))?
    } else {
        json!({})
    };

    let object = root.as_object_mut().ok_or_else(|| {
        anyhow!(
            "{} must contain a top-level JSON object",
            config_path.display()
        )
    })?;
    object.insert(
        "$schema".to_string(),
        Value::String("https://opencode.ai/config.json".to_string()),
    );

    let mcp = object
        .entry("mcp".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let mcp_object = mcp.as_object_mut().ok_or_else(|| {
        anyhow!(
            "{} field 'mcp' must be a JSON object",
            config_path.display()
        )
    })?;
    mcp_object.insert("ctx".to_string(), opencode_ctx_mcp_server_value(repo_root));

    merge_instruction_entries(
        object,
        &[
            "docs/guidelines.md",
            "docs/security.md",
            ".opencode/instructions/ctx-host-first.md",
        ],
    )?;

    fs::write(
        &config_path,
        format!("{}\n", serde_json::to_string_pretty(&root)?),
    )
    .with_context(|| format!("failed to write {}", config_path.display()))?;
    Ok(config_path)
}

fn upsert_codex_project_config(repo_root: &Path) -> Result<PathBuf> {
    let codex_dir = repo_root.join(".codex");
    fs::create_dir_all(&codex_dir)
        .with_context(|| format!("failed to create {}", codex_dir.display()))?;
    let config_path = codex_dir.join("config.toml");
    let mut root = if config_path.is_file() {
        let raw = fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;
        raw.parse::<TomlValue>()
            .with_context(|| format!("failed to parse {}", config_path.display()))?
    } else {
        TomlValue::Table(Default::default())
    };

    let root_table = root.as_table_mut().ok_or_else(|| {
        anyhow!(
            "{} must contain a top-level TOML table",
            config_path.display()
        )
    })?;
    let mcp_servers = ensure_toml_table(root_table, "mcp_servers")?;
    mcp_servers.insert("ctx".to_string(), codex_ctx_mcp_server_value(repo_root));

    fs::write(&config_path, toml::to_string_pretty(&root)?)
        .with_context(|| format!("failed to write {}", config_path.display()))?;
    Ok(config_path)
}

fn upsert_claude_project_config(repo_root: &Path) -> Result<PathBuf> {
    let config_path = repo_root.join(".mcp.json");
    let mut root = if config_path.is_file() {
        let raw = fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;
        serde_json::from_str::<Value>(&raw)
            .with_context(|| format!("failed to parse {}", config_path.display()))?
    } else {
        json!({})
    };

    let object = root.as_object_mut().ok_or_else(|| {
        anyhow!(
            "{} must contain a top-level JSON object",
            config_path.display()
        )
    })?;
    let servers = object
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let servers_object = servers.as_object_mut().ok_or_else(|| {
        anyhow!(
            "{} field 'mcpServers' must be a JSON object",
            config_path.display()
        )
    })?;
    servers_object.insert("ctx".to_string(), claude_ctx_mcp_server_value(repo_root));

    fs::write(
        &config_path,
        format!("{}\n", serde_json::to_string_pretty(&root)?),
    )
    .with_context(|| format!("failed to write {}", config_path.display()))?;
    Ok(config_path)
}

fn ensure_toml_table<'a>(
    root: &'a mut toml::map::Map<String, TomlValue>,
    key: &str,
) -> Result<&'a mut toml::map::Map<String, TomlValue>> {
    let entry = root
        .entry(key.to_string())
        .or_insert_with(|| TomlValue::Table(Default::default()));
    entry
        .as_table_mut()
        .ok_or_else(|| anyhow!("TOML field '{key}' must be a table"))
}

fn write_markdown_assets(
    root: &Path,
    templates: &[HostActionTemplate],
    renderer: fn(&str, &str) -> String,
) -> Result<()> {
    fs::create_dir_all(root).with_context(|| format!("failed to create {}", root.display()))?;
    for template in templates {
        let path = root.join(format!("{}.md", template.slug));
        fs::write(&path, renderer(template.description, template.body))
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

fn write_skill_assets(
    root: &Path,
    templates: &[HostActionTemplate],
    target: HostInstallTarget,
) -> Result<()> {
    fs::create_dir_all(root).with_context(|| format!("failed to create {}", root.display()))?;
    for template in templates {
        let dir = root.join(template.slug);
        fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
        let path = dir.join("SKILL.md");
        fs::write(&path, render_skill_file(target, template))
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

fn asset_file_paths(root: &Path, templates: &[HostActionTemplate], extension: &str) -> Vec<String> {
    templates
        .iter()
        .map(|template| root.join(format!("{}.{}", template.slug, extension)))
        .map(|path| path.display().to_string())
        .collect()
}

fn asset_skill_paths(root: &Path, templates: &[HostActionTemplate]) -> Vec<String> {
    templates
        .iter()
        .map(|template| root.join(template.slug).join("SKILL.md"))
        .map(|path| path.display().to_string())
        .collect()
}

fn render_opencode_command_file(description: &str, template: &str) -> String {
    format!("---\ndescription: {description}\n---\n\n{template}\n")
}

fn render_skill_file(target: HostInstallTarget, template: &HostActionTemplate) -> String {
    let invocation = match target {
        HostInstallTarget::Codex => format!(
            "Invoke this skill explicitly with `${}` inside Codex when you want CTX help for this workflow.",
            template.slug
        ),
        HostInstallTarget::Claude => format!(
            "Invoke this skill explicitly with `/{}` inside Claude Code when you want CTX help for this workflow.",
            template.slug
        ),
        HostInstallTarget::OpenCode => String::new(),
    };

    format!(
        "---\nname: {name}\ndescription: {description}\n---\n\n{invocation}\n\n{body}\n",
        name = template.slug,
        description = template.description,
        invocation = invocation,
        body = template.body,
    )
}

fn merge_instruction_entries(root: &mut Map<String, Value>, entries: &[&str]) -> Result<()> {
    let instructions = root
        .entry("instructions".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let array = instructions
        .as_array_mut()
        .ok_or_else(|| anyhow!("opencode.json field 'instructions' must be an array"))?;

    for entry in entries {
        if !array.iter().any(|item| item.as_str() == Some(entry)) {
            array.push(Value::String((*entry).to_string()));
        }
    }

    Ok(())
}

fn opencode_project_config_value(repo_root: &Path) -> Value {
    let mut mcp = Map::new();
    mcp.insert("ctx".to_string(), opencode_ctx_mcp_server_value(repo_root));

    let mut root = Map::new();
    root.insert(
        "$schema".to_string(),
        Value::String("https://opencode.ai/config.json".to_string()),
    );
    root.insert("mcp".to_string(), Value::Object(mcp));
    Value::Object(root)
}

fn opencode_ctx_mcp_server_value(repo_root: &Path) -> Value {
    json!({
        "type": "local",
        "enabled": true,
        "command": [
            "ctx",
            "--repo-root",
            repo_root.to_string_lossy(),
            "mcp",
            "stdio"
        ]
    })
}

fn claude_mcp_config_value(repo_root: &Path) -> Value {
    json!({
        "mcpServers": {
            "ctx": claude_ctx_mcp_server_value(repo_root)
        }
    })
}

fn claude_ctx_mcp_server_value(repo_root: &Path) -> Value {
    json!({
        "command": "ctx",
        "args": [
            "--repo-root",
            repo_root.to_string_lossy(),
            "mcp",
            "stdio"
        ]
    })
}

fn render_codex_mcp_config(repo_root: &Path) -> Result<String> {
    let value = codex_ctx_mcp_server_value(repo_root);
    let rendered = toml::to_string_pretty(&TomlValue::Table(toml::map::Map::from_iter([(
        "mcp_servers".to_string(),
        TomlValue::Table(toml::map::Map::from_iter([("ctx".to_string(), value)])),
    )])))?;
    Ok(rendered)
}

fn codex_ctx_mcp_server_value(repo_root: &Path) -> TomlValue {
    let mut table = toml::map::Map::new();
    table.insert("command".to_string(), TomlValue::String("ctx".to_string()));
    table.insert(
        "args".to_string(),
        TomlValue::Array(vec![
            TomlValue::String("--repo-root".to_string()),
            TomlValue::String(repo_root.to_string_lossy().to_string()),
            TomlValue::String("mcp".to_string()),
            TomlValue::String("stdio".to_string()),
        ]),
    );
    TomlValue::Table(table)
}

#[derive(Clone, Copy)]
struct HostActionTemplate {
    slug: &'static str,
    description: &'static str,
    body: &'static str,
}

fn shared_action_templates() -> &'static [HostActionTemplate] {
    &[
        HostActionTemplate {
            slug: "ctx",
            description: "Menu | Open the CTX command center and quickstart",
            body: r#"Show a clean, terminal-friendly **CTX Command Center** for the current repository.

Start with the current repository status:
!`ctx doctor`

Then present this menu in English using short sections, aligned bullets, and clear next steps.

# CTX Command Center

## Recommended Start
- `/ctx-doctor` - check repo health and next step
- `/ctx-index` - build or refresh the graph
- `/ctx-memory-bootstrap` - import AGENTS-style project rules
- `/ctx-pack <task>` - build the smallest useful context pack

## Setup
- `/ctx-init`
- `/ctx-index`
- `/ctx-reindex`
- `/ctx-opencode-install`

## Context
- `/ctx-pack <task>`
- `/ctx-ask <task>`
- `/ctx-retrieve <query>`
- `/ctx-graph-query <query>`
- `/ctx-explain <task>`

## Memory
- `/ctx-memory-bootstrap`
- `/ctx-memory-search <topic>`
- `/ctx-memory-list`
- `/ctx-memory-get <key>`
- `/ctx-memory-set <key> <body>`
- `/ctx-memory-export <file>`

## Debug
- `/ctx-prune-logs <topic>`
- `/ctx-prune-diff <topic>`
- `/ctx-hook <task>`

## Benchmark
- `/ctx-benchmark-memory-ab ...`
- `/ctx-benchmark-memory-suite ...`
- `/ctx-stats`

## MCP
- `/ctx-mcp-stdio`
- `/ctx-mcp-serve`
- `/ctx-mcp-config-opencode`

End with:
1. the single best next command for the current repo state
2. one copy-paste example
3. a one-line explanation of why that command should come next"#,
        },
        HostActionTemplate {
            slug: "ctx-help",
            description: "Menu | Show the full CTX CLI command guide",
            body: r#"Current CTX command guide:

!`ctx help`

Summarize the most relevant next CTX commands for the current task."#,
        },
        HostActionTemplate {
            slug: "ctx-init",
            description: "Setup | Initialize CTX runtime for this repository",
            body: r#"Initialize CTX in the current repository.

Run `ctx init`.
Then show the output and tell the user the next recommended command."#,
        },
        HostActionTemplate {
            slug: "ctx-index",
            description: "Setup | Index this repository or selected paths into CTX",
            body: r#"Index this repository into CTX.

Arguments:
- `$ARGUMENTS`: optional path arguments

Run `ctx index $ARGUMENTS` in the current repository root.
Then show the output and explain what was indexed."#,
        },
        HostActionTemplate {
            slug: "ctx-reindex",
            description: "Setup | Reindex selected paths into CTX",
            body: r#"Reindex selected paths in the current repository.

Arguments:
- `$ARGUMENTS`: optional path arguments

Run `ctx reindex $ARGUMENTS`.
Then show the output and explain what changed."#,
        },
        HostActionTemplate {
            slug: "ctx-graph-build",
            description: "Setup | Build the CTX graph from this repository",
            body: r#"Build the CTX graph for the current repository.

Run `ctx graph build`.
Then show the output and explain the result."#,
        },
        HostActionTemplate {
            slug: "ctx-graph-rebuild",
            description: "Setup | Rebuild the CTX graph explicitly",
            body: r#"Rebuild the CTX graph for the current repository.

Run `ctx graph rebuild`.
Then show the output and explain the result."#,
        },
        HostActionTemplate {
            slug: "ctx-doctor",
            description: "Setup | Check CTX repo health and next steps",
            body: r#"Current CTX doctor report:

!`ctx doctor`

Explain whether CTX is ready for this repository.
If something is missing, give the next exact command to run."#,
        },
        HostActionTemplate {
            slug: "ctx-pack",
            description: "Context | Build a compact CTX task context pack",
            body: r#"Build a compact CTX context pack for this task:

$ARGUMENTS

Run `ctx pack "$ARGUMENTS"` in the current repository root.
Show the compact context first, then explain how it should guide the next step."#,
        },
        HostActionTemplate {
            slug: "ctx-retrieve",
            description: "Context | Search CTX retrieval results for a query",
            body: r#"Use CTX retrieval for this query:

$ARGUMENTS

Run `ctx retrieve "$ARGUMENTS" --limit 8` in the current repository root.
Show the ranked hits and explain which files or symbols matter most."#,
        },
        HostActionTemplate {
            slug: "ctx-graph-query",
            description: "Context | Query the CTX graph for files and symbols",
            body: r#"Query the CTX graph for:

$ARGUMENTS

Run `ctx graph query "$ARGUMENTS"` in the current repository root.
Show the graph matches and explain the most relevant relationships."#,
        },
        HostActionTemplate {
            slug: "ctx-prune-logs",
            description: "Debug | Prune noisy logs and keep root-cause signal",
            body: r#"Prune noisy logs with CTX.

Arguments:
- `$ARGUMENTS`: the shell command that produces logs

Run the provided shell command in the current repository and pipe its combined output into `ctx prune logs`.
Then show the pruned output and explain the highest-signal root cause lines."#,
        },
        HostActionTemplate {
            slug: "ctx-prune-diff",
            description: "Debug | Prune the current git diff for a task",
            body: r#"Prune the current git diff with CTX.

Arguments:
- `$ARGUMENTS`: the query to use for diff pruning

Run `git diff | ctx prune diff --query "$ARGUMENTS"` in the current repository.
Then show the compact diff and explain why the remaining hunks matter."#,
        },
        HostActionTemplate {
            slug: "ctx-ask",
            description: "Context | Build compact CTX context without another agent",
            body: r#"Build compact CTX context for this task without invoking another agent.

Arguments:
- `$ARGUMENTS`: the task query

Run `ctx ask "$ARGUMENTS"`.
Then show the result and explain how it should guide the next step."#,
        },
        HostActionTemplate {
            slug: "ctx-hook",
            description: "Debug | Generate a CTX hook or pre-prompt payload",
            body: r#"Generate a CTX hook payload for this task.

Arguments:
- `$ARGUMENTS`: the task query

Run `ctx hook "$ARGUMENTS"`.
Then show the generated payload and explain where it should be used."#,
        },
        HostActionTemplate {
            slug: "ctx-explain",
            description: "Context | Explain likely intent and relevant context",
            body: r#"Explain likely CTX intent and likely context for this task.

Arguments:
- `$ARGUMENTS`: the task query

Run `ctx explain "$ARGUMENTS"`.
Then show the result and summarize the intent classification."#,
        },
        HostActionTemplate {
            slug: "ctx-memory-set",
            description: "Memory | Create or update a CTX memory directive",
            body: r#"Create or update a CTX memory directive in the current repository.

Arguments:
- `$1`: directive key
- `$2`: directive body
- `$3`: optional scope, default `project`
- `$4`: optional source, default `manual`

Run the matching `ctx memory set` command.
Then confirm what was stored and show the exact command used."#,
        },
        HostActionTemplate {
            slug: "ctx-memory-get",
            description: "Memory | Read one CTX memory directive by key",
            body: r#"Read a CTX memory directive from the current repository.

Argument:
- `$1`: directive key

Run `ctx memory get "$1"` and show the result.
If the directive is missing, say that clearly and suggest the matching CTX memory set action."#,
        },
        HostActionTemplate {
            slug: "ctx-memory-list",
            description: "Memory | List CTX memory directives for this repository",
            body: r#"List CTX memory directives in the current repository.

Arguments:
- `$1`: optional scope
- `$2`: optional limit

Run `ctx memory list` with the provided filters.
Show the directives first, then summarize any patterns you notice."#,
        },
        HostActionTemplate {
            slug: "ctx-memory-search",
            description: "Memory | Search CTX memory directives by topic",
            body: r#"Search CTX graph memory for a specific topic.

Arguments:
- `$1`: required search query
- `$2`: optional scope
- `$3`: optional limit

Run the matching `ctx memory search` command.
Show only the relevant directives and explain why they matter for the task."#,
        },
        HostActionTemplate {
            slug: "ctx-memory-delete",
            description: "Memory | Delete one CTX memory directive by key",
            body: r#"Delete a CTX memory directive from the current repository.

Argument:
- `$1`: directive key

Run `ctx memory delete "$1"`.
Then confirm whether the directive was deleted or not found."#,
        },
        HostActionTemplate {
            slug: "ctx-memory-import",
            description: "Memory | Import AGENTS-style guidance into CTX memory",
            body: r#"Import markdown guidance into CTX graph memory.

Arguments:
- `$1`: markdown file path
- `$2`: optional scope, default `project`
- `$3`: optional source, default `markdown`
- `$4`: optional prefix

Run the matching `ctx memory import` command.
Then report how many directives were imported and from which file."#,
        },
        HostActionTemplate {
            slug: "ctx-memory-bootstrap",
            description: "Memory | Bootstrap graph memory from AGENTS-style markdown",
            body: r#"Bootstrap CTX graph memory from conventional markdown rule files.

Arguments:
- `$ARGUMENTS`: optional explicit file paths

If no arguments are provided, run `ctx memory bootstrap` so CTX scans common files such as:
- `AGENTS.md`
- `CLAUDE.md`
- `CODEX.md`
- `.github/copilot-instructions.md`

Then show how many files and directives were imported."#,
        },
        HostActionTemplate {
            slug: "ctx-memory-export",
            description: "Memory | Export CTX memory directives to markdown",
            body: r#"Export CTX graph memory to a markdown file.

Arguments:
- `$1`: output file path
- `$2`: optional scope
- `$3`: optional limit
- `$4`: optional title

Run the matching `ctx memory export` command.
Then confirm the output file path and the number of exported directives."#,
        },
        HostActionTemplate {
            slug: "ctx-benchmark-memory-ab",
            description: "Benchmark | Compare markdown memory vs CTX graph memory",
            body: r#"Run the CTX memory A/B benchmark in the current repository.

Arguments:
- `$1`: task query
- `$2`: markdown file path
- `$3`: optional limit
- `$4`: optional checklist path
- `$5`: optional markdown answer path
- `$6`: optional graph answer path

Run the matching `ctx benchmark memory-ab` command.
Then explain the token delta and which side won on quality if that data is present."#,
        },
        HostActionTemplate {
            slug: "ctx-benchmark-memory-suite",
            description: "Benchmark | Run a reusable CTX memory benchmark suite",
            body: r#"Run the CTX memory benchmark suite in the current repository.

Arguments:
- `$1`: required spec path
- `$2`: optional markdown report path, default `benchmark-report.md`
- `$3`: optional JSON report path

Run:
- `ctx benchmark memory-suite --spec <spec> --report-out <report>`
- include `--json-out <json>` when structured output is also needed

Then summarize the suite KPIs and point to the generated report files."#,
        },
        HostActionTemplate {
            slug: "ctx-stats",
            description: "Benchmark | Show the latest CTX token and runtime stats",
            body: r#"Show the latest local CTX stats for this repository.

!`ctx stats`

Explain the last run briefly, including token reduction and any recorded runtime metadata."#,
        },
        HostActionTemplate {
            slug: "ctx-opencode-install",
            description: "Setup | Refresh CTX integration files for OpenCode",
            body: r#"Refresh the current repository's OpenCode integration.

Run `ctx opencode install`.
Then show the output and summarize which files were written or updated."#,
        },
        HostActionTemplate {
            slug: "ctx-mcp-serve",
            description: "MCP | Show or start the CTX MCP HTTP server",
            body: r#"Prepare the CTX MCP HTTP server for this repository.

Arguments:
- `$1`: optional port, default `8765`

If the user wants the server started in this session, run `ctx mcp serve --port <port>`.
Otherwise, show the exact command to run and explain that it is a long-running local process."#,
        },
        HostActionTemplate {
            slug: "ctx-mcp-stdio",
            description: "MCP | Show the CTX MCP stdio launch command",
            body: r#"Show the CTX MCP stdio launch command for the current repository.

Use the current repository root and explain how a host CLI can launch `ctx --repo-root <repo> mcp stdio` locally."#,
        },
        HostActionTemplate {
            slug: "ctx-mcp-config-claude",
            description: "MCP | Generate CTX MCP config for Claude Code",
            body: r#"Generate the CTX MCP config snippet for Claude Code.

Run `ctx mcp config claude`.
Then show the output and explain how to use it."#,
        },
        HostActionTemplate {
            slug: "ctx-mcp-config-opencode",
            description: "MCP | Generate CTX MCP config for OpenCode",
            body: r#"Generate the CTX MCP config snippet for OpenCode.

Run `ctx mcp config opencode`.
Then show the output and explain how to use it."#,
        },
    ]
}

fn opencode_instruction_files() -> [(&'static str, &'static str); 1] {
    [(
        "ctx-host-first.md",
        r#"# CTX Host-First Rules For OpenCode

CTX is the local context runtime for this repository.

## Primary Workflow

- Stay inside OpenCode for normal work.
- Prefer CTX slash commands and CTX MCP tools before broad file dumping.
- Keep the current OpenCode-selected model and agent in control.
- Do not revive wrapper-style workflows like `ctx wrap`, `ctx codex`, `ctx claude`, or `ctx opencode run`.

## Automatic CTX Usage

For normal prompts, prefer CTX-first behavior:

1. If repository readiness is unclear, run `/ctx-doctor`.
2. If graph/index state is stale or missing, run `/ctx-index` or `/ctx-reindex`.
3. For code understanding, prefer `/ctx-retrieve`, `/ctx-graph-query`, and CTX MCP tools before manually reading many files.
4. For debugging logs, prefer `/ctx-prune-logs`.
5. For debugging diffs, prefer `/ctx-prune-diff`.
6. For project habits or persistent rules, bootstrap markdown habits once with `/ctx-memory-bootstrap`, then prefer `/ctx-memory-search`, `/ctx-memory-list`, `/ctx-memory-get`, and `/ctx-memory-set` instead of large markdown habit files.
7. For context construction, prefer `/ctx-pack` or `/ctx-ask` before assembling large prompts manually.
8. For prompt scaffolding, use `/ctx-hook`.
9. For ambiguity about likely scope or intent, use `/ctx-explain`.
10. For validation of graph-memory token savings, use `/ctx-benchmark-memory-ab` or `/ctx-benchmark-memory-suite`.

## Memory And Rules

- Treat graph memory as the primary structured replacement for AGENTS-style project habits.
- Use `/ctx-memory-bootstrap` to migrate conventional markdown files into graph memory without leaving OpenCode.
- Only export markdown memory when compatibility or auditing is explicitly needed.
- Prefer updating graph memory directives over adding new large instruction files.

## Retrieval Discipline

- Start with the smallest high-signal CTX command that answers the task.
- Avoid loading many files when CTX already exposes the relevant graph or retrieval context.
- Use CTX compact context before broad scans whenever the task involves debugging, implementation, or review.

## Safety

- Respect CTX privacy defaults and sensitive file blocking behavior.
- Keep all project data local unless the host or the user explicitly chooses otherwise.
"#,
    )]
}
