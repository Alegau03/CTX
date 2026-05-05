use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde_json::{Map, Value, json};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpencodeInstallProfile {
    Full,
    Core,
}

impl OpencodeInstallProfile {
    pub fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "full" => Ok(Self::Full),
            "core" => Ok(Self::Core),
            other => Err(anyhow!(
                "unknown OpenCode install profile '{other}'. Expected: full or core"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Core => "core",
        }
    }
}

pub fn render_mcp_config(repo_root: &Path, client: &str, port: u16) -> Result<String> {
    match client.to_ascii_lowercase().as_str() {
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
            "unknown MCP config client '{other}'. Expected: opencode or http"
        )),
    }
}

pub fn install_opencode_integration(
    repo_root: &Path,
    profile: OpencodeInstallProfile,
) -> Result<Value> {
    let config_path = upsert_opencode_project_config(repo_root)?;
    let commands_dir = repo_root.join(".opencode/commands");
    let ctx_binary = current_ctx_binary();
    let templates = action_templates_for_profile(profile);
    remove_stale_asset_files(&commands_dir, shared_action_templates(), &templates, "md")?;
    write_markdown_assets(&commands_dir, repo_root, &ctx_binary, &templates)?;

    let instructions_dir = repo_root.join(".opencode/instructions");
    fs::create_dir_all(&instructions_dir)
        .with_context(|| format!("failed to create {}", instructions_dir.display()))?;
    let profile_marker = repo_root.join(".opencode/ctx-profile.txt");
    fs::write(&profile_marker, format!("{}\n", profile.as_str()))
        .with_context(|| format!("failed to write {}", profile_marker.display()))?;

    let mut instruction_paths = Vec::new();
    for (filename, body) in opencode_instruction_files(profile) {
        let path = instructions_dir.join(filename);
        fs::write(&path, body).with_context(|| format!("failed to write {}", path.display()))?;
        instruction_paths.push(path.display().to_string());
    }

    let command_paths = asset_file_paths(&commands_dir, &templates, "md");
    let sidebar = sync_opencode_sidebar_assets(repo_root, profile, &ctx_binary)?;

    Ok(json!({
        "host": "opencode",
        "display_name": "OpenCode",
        "profile": profile.as_str(),
        "config_path": config_path.display().to_string(),
        "commands_dir": commands_dir.display().to_string(),
        "instructions_dir": instructions_dir.display().to_string(),
        "profile_path": profile_marker.display().to_string(),
        "commands_written": command_paths.len(),
        "command_files": command_paths,
        "instruction_files": instruction_paths,
        "sidebar": sidebar,
        "available_profiles": ["core", "full"],
        "next_step": match profile {
            OpencodeInstallProfile::Full => {
                "open this repo in OpenCode, check the CTX panel in the right sidebar, then run /ctx-doctor or /ctx-pack <task>"
            }
            OpencodeInstallProfile::Core => {
                "open this repo in OpenCode and start with /ctx-doctor, or rerun `ctx opencode install --profile full` to unlock the full CTX surface and the live sidebar dashboard"
            }
        }
    }))
}

fn sync_opencode_sidebar_assets(
    repo_root: &Path,
    profile: OpencodeInstallProfile,
    ctx_binary: &str,
) -> Result<Value> {
    let plugins_dir = repo_root.join(".opencode/plugins");
    let plugin_path = plugins_dir.join("ctx-dashboard.tsx");
    let package_path = repo_root.join(".opencode/package.json");
    let tui_path = repo_root.join(".opencode/tui.json");
    let plugin_spec = "./plugins/ctx-dashboard.tsx";

    match profile {
        OpencodeInstallProfile::Full => {
            fs::create_dir_all(&plugins_dir)
                .with_context(|| format!("failed to create {}", plugins_dir.display()))?;
            fs::write(
                &plugin_path,
                render_opencode_sidebar_plugin(repo_root, ctx_binary)?,
            )
            .with_context(|| format!("failed to write {}", plugin_path.display()))?;
            upsert_opencode_package_json(&package_path)?;
            upsert_opencode_tui_json(&tui_path, plugin_spec)?;
            Ok(json!({
                "enabled": true,
                "plugin_path": plugin_path.display().to_string(),
                "package_path": package_path.display().to_string(),
                "tui_path": tui_path.display().to_string(),
                "plugin_spec": plugin_spec
            }))
        }
        OpencodeInstallProfile::Core => {
            if plugin_path.exists() {
                fs::remove_file(&plugin_path).with_context(|| {
                    format!(
                        "failed to remove stale sidebar plugin {}",
                        plugin_path.display()
                    )
                })?;
            }
            remove_opencode_tui_plugin(&tui_path, plugin_spec)?;
            Ok(json!({
                "enabled": false
            }))
        }
    }
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

fn write_markdown_assets(
    root: &Path,
    repo_root: &Path,
    ctx_binary: &str,
    templates: &[HostActionTemplate],
) -> Result<()> {
    fs::create_dir_all(root).with_context(|| format!("failed to create {}", root.display()))?;
    for template in templates {
        let path = root.join(format!("{}.md", template.slug));
        fs::write(
            &path,
            render_opencode_command_file(
                template.description,
                template.body,
                repo_root,
                ctx_binary,
            ),
        )
        .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

fn remove_stale_asset_files(
    root: &Path,
    all_templates: &[HostActionTemplate],
    active_templates: &[HostActionTemplate],
    extension: &str,
) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for template in all_templates {
        if active_templates
            .iter()
            .any(|item| item.slug == template.slug)
        {
            continue;
        }
        let path = root.join(format!("{}.{}", template.slug, extension));
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove stale asset {}", path.display()))?;
        }
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

fn render_opencode_command_file(
    description: &str,
    template: &str,
    repo_root: &Path,
    ctx_binary: &str,
) -> String {
    let rendered = template
        .replace("{{CTX_CMD}}", &ctx_command_string(repo_root, ctx_binary))
        .replace("{{CTX_BIN}}", &shell_quote(ctx_binary))
        .replace("{{REPO_ROOT}}", &shell_quote(&repo_root.to_string_lossy()));
    format!("---\ndescription: {description}\n---\n\n{rendered}\n")
}

fn upsert_opencode_package_json(path: &Path) -> Result<()> {
    let mut root = if path.is_file() {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str::<Value>(&raw)
            .with_context(|| format!("failed to parse {}", path.display()))?
    } else {
        json!({
            "private": true,
            "type": "module"
        })
    };

    let object = root
        .as_object_mut()
        .ok_or_else(|| anyhow!("{} must contain a top-level JSON object", path.display()))?;
    object.insert("private".to_string(), Value::Bool(true));
    object.insert("type".to_string(), Value::String("module".to_string()));

    let dependencies = object
        .entry("dependencies".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let dependency_object = dependencies.as_object_mut().ok_or_else(|| {
        anyhow!(
            "{} field 'dependencies' must be a JSON object",
            path.display()
        )
    })?;

    for (name, version) in [
        ("@opencode-ai/plugin", "^1.14.19"),
        ("@opentui/core", "^0.1.101"),
        ("@opentui/solid", "^0.1.101"),
        ("solid-js", "^1.9.10"),
    ] {
        dependency_object.insert(name.to_string(), Value::String(version.to_string()));
    }

    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&root)?))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn upsert_opencode_tui_json(path: &Path, plugin_spec: &str) -> Result<()> {
    let mut root = if path.is_file() {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str::<Value>(&raw)
            .with_context(|| format!("failed to parse {}", path.display()))?
    } else {
        json!({})
    };

    let object = root
        .as_object_mut()
        .ok_or_else(|| anyhow!("{} must contain a top-level JSON object", path.display()))?;
    object.insert(
        "$schema".to_string(),
        Value::String("https://opencode.ai/tui.json".to_string()),
    );

    let plugins = object
        .entry("plugin".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let array = plugins
        .as_array_mut()
        .ok_or_else(|| anyhow!("{} field 'plugin' must be an array", path.display()))?;

    if !array.iter().any(|item| item.as_str() == Some(plugin_spec)) {
        array.push(Value::String(plugin_spec.to_string()));
    }

    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&root)?))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn remove_opencode_tui_plugin(path: &Path, plugin_spec: &str) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }

    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut root = serde_json::from_str::<Value>(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let object = root
        .as_object_mut()
        .ok_or_else(|| anyhow!("{} must contain a top-level JSON object", path.display()))?;

    if let Some(plugins) = object.get_mut("plugin") {
        let array = plugins
            .as_array_mut()
            .ok_or_else(|| anyhow!("{} field 'plugin' must be an array", path.display()))?;
        array.retain(|item| item.as_str() != Some(plugin_spec));
        if array.is_empty() {
            object.remove("plugin");
        }
    }

    let only_schema = object.len() == 1 && object.contains_key("$schema");
    let empty = object.is_empty();
    if empty || only_schema {
        fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
        return Ok(());
    }

    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&root)?))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn render_opencode_sidebar_plugin(repo_root: &Path, ctx_binary: &str) -> Result<String> {
    let repo_root_js = serde_json::to_string(&repo_root.to_string_lossy().to_string())?;
    let ctx_binary_js = serde_json::to_string(ctx_binary)?;

    Ok(format!(
        r##"/** @jsxImportSource @opentui/solid */
import type {{ TuiPlugin, TuiPluginApi, TuiPluginModule }} from "@opencode-ai/plugin/tui";
import {{ createEffect, createSignal, onCleanup }} from "solid-js";
import {{ execFile }} from "node:child_process";
import {{ promisify }} from "node:util";

const id = "@ctx/sidebar-dashboard";
const SIDEBAR_ORDER = 140;
const REFRESH_INTERVAL_MS = 15000;
const REPO_ROOT = {repo_root_js};
const CTX_BIN = {ctx_binary_js};
const execFileAsync = promisify(execFile);

type Dashboard = any;
type Line = {{ text: string; fg?: string; bold?: boolean }};
const CTX_RED = "#ff375f";
const CTX_BLUE = "#4da3ff";

function shorten(value: string | null | undefined, limit = 28) {{
  if (!value) return "none";
  if (value.length <= limit) return value;
  return `${{value.slice(0, limit - 1)}}…`;
}}

function formatTokens(value: number | null | undefined) {{
  return `${{Number(value || 0).toLocaleString("en-US")}} tok`;
}}

function formatPct(value: number | null | undefined) {{
  return `${{Number(value || 0).toFixed(1)}}%`;
}}

function metric(label: string, value: string, fg?: string): Line {{
  return {{ text: `${{label.padEnd(10)}} ${{value}}`, fg }};
}}

async function loadDashboard() {{
  const {{ stdout }} = await execFileAsync(
    CTX_BIN,
    ["--json", "--repo-root", REPO_ROOT, "host-dashboard"],
    {{
      cwd: REPO_ROOT,
      maxBuffer: 1024 * 1024,
    }},
  );
  return JSON.parse(stdout || "{{}}");
}}

function buildLines(dashboard: Dashboard): Line[] {{
  const savings = dashboard?.savings || {{}};
  const cache = dashboard?.cache || {{}};
  const index = cache.index || {{}};
  const read = cache.read || {{}};
  const topWins = dashboard?.top_wins || {{}};
  const bestQuery = topWins.best_query || {{}};
  const latestPack = dashboard?.latest_activity?.latest_pack_path?.split("/").pop() || "none";

  return [
    {{ text: "CTX Dashboard", fg: CTX_RED, bold: true }},
    {{ text: `${{dashboard?.repo || "repo"}}`, fg: CTX_BLUE }},
    {{ text: "" }},
    {{ text: "Savings", fg: CTX_RED, bold: true }},
    metric("Saved", formatTokens(savings.estimated_tokens_saved)),
    metric("Avg/run", formatTokens(savings.average_tokens_saved_per_run)),
    metric("Avg red", formatPct(savings.average_reduction_pct)),
    metric("Latest", formatPct(savings.latest_reduction_pct)),
    metric("Runs", String(savings.sampled_runs || 0)),
    {{ text: "" }},
    {{ text: "Cache", fg: CTX_RED, bold: true }},
    metric("Read hit", formatPct(read.hit_rate_pct)),
    metric("Idx reuse", formatPct(index.reuse_ratio_pct)),
    metric("Reads", String(read.total_reads || 0)),
    metric("Tracked", String(read.tracked_files || 0)),
    {{ text: "" }},
    {{ text: "Top Win", fg: CTX_RED, bold: true }},
    {{ text: shorten(bestQuery.query || "none"), fg: CTX_BLUE }},
    metric("Saved", formatTokens(bestQuery.estimated_tokens_saved)),
    metric("Runs", String(bestQuery.runs || 0)),
    metric("Avg red", formatPct(bestQuery.average_reduction_pct)),
    {{ text: "" }},
    {{ text: "Artifact", fg: CTX_RED, bold: true }},
    {{ text: shorten(latestPack, 34), fg: CTX_BLUE }},
  ];
}}

function colorFor(line: Line) {{
  return line.fg || CTX_BLUE;
}}

function SidebarContentView(props: {{ api: TuiPluginApi; sessionID: string }}) {{
  const [lines, setLines] = createSignal<Line[]>([
    {{ text: "CTX Dashboard", fg: CTX_RED, bold: true }},
    {{ text: "Loading dashboard…", fg: CTX_BLUE }},
  ]);

  let disposed = false;
  let loadVersion = 0;
  const timers = new Set<ReturnType<typeof setTimeout>>();

  const reload = () => {{
    const currentVersion = ++loadVersion;
    void loadDashboard()
      .then((dashboard) => {{
        if (disposed || currentVersion !== loadVersion) return;
        setLines(buildLines(dashboard));
      }})
      .catch((error) => {{
        if (disposed || currentVersion !== loadVersion) return;
        setLines([
          {{ text: "CTX Dashboard", fg: CTX_RED, bold: true }},
          {{ text: "Dashboard unavailable", fg: CTX_RED }},
          {{ text: shorten(String(error?.message || error), 30), fg: CTX_BLUE }},
        ]);
      }});
  }};

  const queueRefresh = (delay: number) => {{
    const timer = setTimeout(() => {{
      timers.delete(timer);
      reload();
    }}, delay);
    timers.add(timer);
  }};

  const scheduleRefresh = () => {{
    queueRefresh(150);
    queueRefresh(750);
  }};

  createEffect(() => {{
    props.sessionID;
    reload();
    queueRefresh(600);
    queueRefresh(1800);
  }});

  const interval = setInterval(reload, REFRESH_INTERVAL_MS);
  const unsubscribers = [
    props.api.event.on("session.updated", (event) => {{
      if (event.properties?.info?.id === props.sessionID) scheduleRefresh();
    }}),
    props.api.event.on("message.updated", (event) => {{
      if (event.properties?.info?.sessionID === props.sessionID) scheduleRefresh();
    }}),
    props.api.event.on("message.removed", (event) => {{
      if (event.properties?.sessionID === props.sessionID) scheduleRefresh();
    }}),
    props.api.event.on("tui.session.select", (event) => {{
      if (event.properties?.sessionID === props.sessionID) scheduleRefresh();
    }}),
  ];

  onCleanup(() => {{
    disposed = true;
    clearInterval(interval);
    for (const timer of timers) clearTimeout(timer);
    timers.clear();
    for (const unsubscribe of unsubscribers) unsubscribe();
  }});

  return (
    <box gap={{0}}>
      {{lines().map((line) => (
        <text fg={{colorFor(line)}} wrapMode="none">
          {{line.bold ? <b>{{line.text || " "}}</b> : line.text || " "}}
        </text>
      ))}}
    </box>
  );
}}

const tui: TuiPlugin = async (api) => {{
  api.slots.register({{
    order: SIDEBAR_ORDER,
    slots: {{
      sidebar_content(_ctx, props: {{ session_id: string }}) {{
        return <SidebarContentView api={{api}} sessionID={{props.session_id}} />;
      }},
    }},
  }});
}};

const pluginModule: TuiPluginModule & {{ id: string }} = {{
  id,
  tui,
}};

export default pluginModule;
"##
    ))
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
    let binary = current_ctx_binary();

    json!({
        "type": "local",
        "enabled": true,
        "command": [
            binary,
            "--repo-root",
            repo_root.to_string_lossy(),
            "mcp",
            "stdio"
        ]
    })
}

fn current_ctx_binary() -> String {
    std::env::current_exe()
        .ok()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "ctx".to_string())
}

fn ctx_command_string(repo_root: &Path, ctx_binary: &str) -> String {
    format!(
        "{} --repo-root {}",
        shell_quote(ctx_binary),
        shell_quote(&repo_root.to_string_lossy())
    )
}

fn shell_quote(value: &str) -> String {
    let escaped = value.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}

#[derive(Clone, Copy)]
struct HostActionTemplate {
    slug: &'static str,
    description: &'static str,
    body: &'static str,
}

fn action_templates_for_profile(profile: OpencodeInstallProfile) -> Vec<HostActionTemplate> {
    match profile {
        OpencodeInstallProfile::Full => shared_action_templates().to_vec(),
        OpencodeInstallProfile::Core => shared_action_templates()
            .iter()
            .copied()
            .filter(|template| {
                matches!(
                    template.slug,
                    "ctx"
                        | "ctx-doctor"
                        | "ctx-plan"
                        | "ctx-retrieve"
                        | "ctx-pack"
                        | "ctx-run"
                        | "ctx-prune-logs"
                        | "ctx-stats"
                        | "ctx-gain"
                )
            })
            .collect(),
    }
}

fn shared_action_templates() -> &'static [HostActionTemplate] {
    &[
        HostActionTemplate {
            slug: "ctx",
            description: "Menu | Open the CTX command center and quickstart",
            body: r#"Run the deterministic CTX menu command below and present its output as-is.

Rules:
- do not inspect files manually
- do not call subagents
- do not infer repository state from anything except the command output
- do not rewrite slash commands into another format

!`{{CTX_CMD}} menu`"#,
        },
        HostActionTemplate {
            slug: "ctx-help",
            description: "Menu | Show the full CTX CLI command guide",
            body: r#"Current CTX command guide:

!`{{CTX_CMD}} help`

Summarize the most relevant next CTX commands for the current task."#,
        },
        HostActionTemplate {
            slug: "ctx-init",
            description: "Setup | Initialize CTX runtime for this repository",
            body: r#"Initialize CTX in the current repository.

!`{{CTX_CMD}} init`

Then show the output and tell the user the next recommended command."#,
        },
        HostActionTemplate {
            slug: "ctx-index",
            description: "Setup | Index this repository or selected paths into CTX",
            body: r#"Index this repository into CTX.

Arguments:
- `$ARGUMENTS`: optional path arguments

!`{{CTX_CMD}} index $ARGUMENTS`

Rules:
- run only the exact CTX command above
- do not glob files or inspect the filesystem manually
- do not infer indexed files from repository contents

Then show the output first.
If `indexed_files:` is present, explain that field in one short sentence only."#,
        },
        HostActionTemplate {
            slug: "ctx-reindex",
            description: "Setup | Reindex selected paths into CTX",
            body: r#"Reindex selected paths in the current repository.

Arguments:
- `$ARGUMENTS`: optional path arguments

!`{{CTX_CMD}} reindex $ARGUMENTS`

Then show the output and explain what changed."#,
        },
        HostActionTemplate {
            slug: "ctx-graph-build",
            description: "Setup | Build the CTX graph from this repository",
            body: r#"Build the CTX graph for the current repository.

!`{{CTX_CMD}} graph build`

Then show the output and explain the result."#,
        },
        HostActionTemplate {
            slug: "ctx-graph-rebuild",
            description: "Setup | Rebuild the CTX graph explicitly",
            body: r#"Rebuild the CTX graph for the current repository.

!`{{CTX_CMD}} graph rebuild`

Then show the output and explain the result."#,
        },
        HostActionTemplate {
            slug: "ctx-doctor",
            description: "Setup | Check CTX repo health and next steps",
            body: r#"Current CTX doctor report:

!`{{CTX_CMD}} doctor`

Interpret the report deterministically:
- if `ready: true`, say CTX is ready; treat `next:` as the recommended workflow step, not missing setup
- if `ready: false`, say CTX is not ready and print the exact `next:` command
- print the exact `next:` command verbatim
- do not inspect files manually
- do not contradict the `ready:` line"#,
        },
        HostActionTemplate {
            slug: "ctx-pack",
            description: "Context | Build a compact CTX task context pack",
            body: r#"Build a compact CTX context pack for this task:

$ARGUMENTS

!`{{CTX_CMD}} pack "$ARGUMENTS" --json`

Render exactly this compact markdown:
- `## 📦 CTX Pack`
- `**Context**`
- `**Stats**`

Print `compact_context` first under `**Context**`.
Then print one compact stats line under `**Stats**` with `packed_tokens`, `reduction_pct`, and `pack_path`.
Keep any follow-up explanation to at most one short sentence."#,
        },
        HostActionTemplate {
            slug: "ctx-compare",
            description: "Context | Show before-vs-CTX context density for a task",
            body: r#"OpenCode-only CTX comparison for this task:

$ARGUMENTS

!`{{CTX_CMD}} pack "$ARGUMENTS" --json`

Print a compact `Before vs CTX` table first using:
- `original_estimated_tokens` as the broad-context estimate
- `packed_tokens` as the CTX task pack size
- `reduction_pct` as the reduction
- `pack_path` as the saved artifact

Then list included and excluded categories in one compact block.
Do not claim benchmark quality from this command; it is a task-pack density check."#,
        },
        HostActionTemplate {
            slug: "ctx-plan",
            description: "Planning | Build a graph-backed low-token implementation plan",
            body: r#"OpenCode-only CTX implementation plan for this task:

$ARGUMENTS

Retrieval:
!`{{CTX_CMD}} retrieve "$ARGUMENTS" --limit 8 --json`

Relevant memory:
!`{{CTX_CMD}} memory search "$ARGUMENTS" --json`

Graph:
!`{{CTX_CMD}} graph query "$ARGUMENTS"`

Context pack:
!`{{CTX_CMD}} pack "$ARGUMENTS" --json`

Render exactly this markdown skeleton:
- `## 🧭 CTX Plan`
- `**Task**`
- `**Intent**`
- `**Relevant Context**`
- `**Token Efficiency**`
- `**Plan**`
- `**Suggested Tests**`
- `**Suggested First Action**`

Under each heading, keep the content concise:
- `**Task**`: one-sentence restatement
- `**Intent**`: classify the work, for example feature, bugfix, refactor, test, docs, or investigation
- `**Relevant Context**`: files, symbols, memory directives, and relationships from the CTX outputs only
- `**Token Efficiency**`: use `original_estimated_tokens`, `packed_tokens`, `reduction_pct`, and `pack_path`
- `**Plan**`: 4-7 ordered implementation steps
- `**Suggested Tests**`: focused verification commands or test files inferred from CTX outputs
- `**Suggested First Action**`: the first file or command OpenCode should use next

Rules:
- do not inspect files manually while planning
- do not implement code
- do not invent files that are not supported by CTX output
- keep the result compact and immediately actionable"#,
        },
        HostActionTemplate {
            slug: "ctx-retrieve",
            description: "Context | Search CTX retrieval results for a query",
            body: r#"Use CTX retrieval for this query:

$ARGUMENTS

!`{{CTX_CMD}} retrieve "$ARGUMENTS" --limit 8 --json`

Render exactly this compact markdown:
- `## 🔎 CTX Retrieve`
- `**Top Hits**`
- `**Next**`

Start with the useful result immediately under `**Top Hits**`.
Show the top hits in a clean, predictable format using the returned `source`, `score`, `id`, and `reason`.
Use `**Next**` for a single sentence about the most useful follow-up.
Keep any follow-up summary to one short sentence."#,
        },
        HostActionTemplate {
            slug: "ctx-read",
            description: "Context | Read one file with CTX cache-aware modes",
            body: r#"OpenCode-only CTX file read with session cache / re-read compression.

Arguments:
- `$1`: required file path
- `$2`: optional mode, one of `full`, `outline`, or `digest`

Usage:
- `/ctx-read src/auth.ts`
- `/ctx-read src/auth.ts outline`
- `/ctx-read docs/runbook.md digest`

If `$1` is missing, stop and show the usage above.

Run:
!`mode="${2:-digest}"; {{CTX_CMD}} --json host-read "$1" --mode "$mode"`

Render exactly this compact markdown:
- `## 📖 CTX Read`
- `**Content**`
- `**Metadata**`

Print `output` under `**Content**`.
Then print one compact metadata line under `**Metadata**` with `mode`, `cache_hit`, `fingerprint`, and `path`.
Keep any explanation to one short sentence."#,
        },
        HostActionTemplate {
            slug: "ctx-graph-query",
            description: "Context | Query the CTX graph for files and symbols",
            body: r#"Query the CTX graph for:

$ARGUMENTS

!`{{CTX_CMD}} graph query "$ARGUMENTS"`

Show the graph matches and explain the most relevant relationships."#,
        },
        HostActionTemplate {
            slug: "ctx-run",
            description: "Debug | Run a shell command and return the pruned root cause",
            body: r#"OpenCode-only CTX command runner for this repository.

Arguments:
- `$ARGUMENTS`: the exact shell command to execute

`$ARGUMENTS` must be a real shell command such as `npm test -- --grep "refresh"` or `cargo test auth_refresh`.
Do not treat `$ARGUMENTS` as a topic, label, or natural-language request.
If `$ARGUMENTS` does not look runnable, stop and tell the user to provide the exact shell command to execute.

!`{{CTX_CMD}} --json host-run "$ARGUMENTS"`

Render exactly this compact markdown:
- `## 🧪 CTX Run`
- `**Summary**`
- `**Output**`
- `**Log**`

Put `summary` under `**Summary**`.
Put `pruned_output` under `**Output**`.
Under `**Log**`, print one compact metadata line with `exit_code`, `latency_ms`, and `raw_log_path`.

Keep any explanation to one short sentence."#,
        },
        HostActionTemplate {
            slug: "ctx-prune-logs",
            description: "Debug | Prune noisy logs and keep root-cause signal",
            body: r#"Prune noisy logs with CTX.

Arguments:
- `$ARGUMENTS`: the exact shell command that produces logs

`$ARGUMENTS` must be a real shell command such as `npm test -- --grep "refresh"` or `pytest -k auth -q`.
Do not treat `$ARGUMENTS` as a topic, label, or search phrase.
If `$ARGUMENTS` does not look runnable, stop and tell the user to provide the exact shell command to execute.

Run the provided shell command in the current repository and pipe its combined output into `{{CTX_CMD}} prune logs --max-lines 50`.
Show the pruned output first.
Keep any root-cause explanation to one short sentence."#,
        },
        HostActionTemplate {
            slug: "ctx-prune-diff",
            description: "Debug | Prune the current git diff for a task",
            body: r#"Prune the current git diff with CTX.

Arguments:
- `$ARGUMENTS`: the query to use for diff pruning

Run `git diff | {{CTX_CMD}} prune diff --query "$ARGUMENTS"` in the current repository.
Show the compact diff first.
Keep any follow-up explanation to one short sentence."#,
        },
        HostActionTemplate {
            slug: "ctx-ask",
            description: "Context | Build compact CTX context without another agent",
            body: r#"Build compact CTX context for this task without invoking another agent.

Arguments:
- `$ARGUMENTS`: the task query

!`{{CTX_CMD}} ask "$ARGUMENTS"`

Then show the result and explain how it should guide the next step."#,
        },
        HostActionTemplate {
            slug: "ctx-hook",
            description: "Debug | Generate a CTX hook or pre-prompt payload",
            body: r#"Generate a CTX hook payload for this task.

Arguments:
- `$ARGUMENTS`: the task query

!`{{CTX_CMD}} hook "$ARGUMENTS" --json`

Print `hook_prompt` first.
Then print a single compact metadata line with `packed_tokens`, `reduction_pct`, and `pack_path`.
Keep any usage note to one short sentence."#,
        },
        HostActionTemplate {
            slug: "ctx-explain",
            description: "Context | Explain likely intent and relevant context",
            body: r#"Explain likely CTX intent and likely context for this task.

Arguments:
- `$ARGUMENTS`: the task query

!`{{CTX_CMD}} explain "$ARGUMENTS"`

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
Use `{{CTX_CMD}} memory set ...` as the command prefix.
Then confirm what was stored and show the exact command used."#,
        },
        HostActionTemplate {
            slug: "ctx-memory-get",
            description: "Memory | Read one CTX memory directive by key",
            body: r#"Read a CTX memory directive from the current repository.

Argument:
- `$1`: directive key

!`{{CTX_CMD}} memory get "$1"`

If the directive is missing, say that clearly and suggest the matching CTX memory set action."#,
        },
        HostActionTemplate {
            slug: "ctx-memory-list",
            description: "Memory | List CTX memory directives for this repository",
            body: r#"List CTX memory directives in the current repository.

Arguments:
- `$1`: optional scope
- `$2`: optional limit

Run `{{CTX_CMD}} memory list` with the provided filters.
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

Run the matching `{{CTX_CMD}} memory search "$1" --json` command, adding scope and limit only when they were provided.
Show only the matching directives in a compact, predictable format.
Do not add extra commentary beyond one short sentence if needed."#,
        },
        HostActionTemplate {
            slug: "ctx-memory-delete",
            description: "Memory | Delete one CTX memory directive by key",
            body: r#"Delete a CTX memory directive from the current repository.

Argument:
- `$1`: directive key

Run `{{CTX_CMD}} memory delete "$1"`.
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

Run the matching `{{CTX_CMD}} memory import` command.
Then report how many directives were imported and from which file."#,
        },
        HostActionTemplate {
            slug: "ctx-memory-bootstrap",
            description: "Memory | Bootstrap graph memory from AGENTS-style markdown",
            body: r#"Bootstrap CTX graph memory from conventional markdown rule files.

Arguments:
- `$ARGUMENTS`: optional explicit file paths

If no arguments are provided, run `{{CTX_CMD}} memory bootstrap` so CTX scans common files such as:
- `AGENTS.md`
- `CLAUDE.md`
- `CODEX.md`
- `.github/copilot-instructions.md`

Rules:
- run only the exact CTX command
- do not scan the repository manually to count files or directives

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

Run the matching `{{CTX_CMD}} memory export` command.
Then confirm the output file path and the number of exported directives."#,
        },
        HostActionTemplate {
            slug: "ctx-toolbook-import",
            description: "Toolbooks | Import a CLI manual or playbook into graph memory",
            body: r#"Import an external CLI manual, runbook, or tool cheat sheet as an OpenCode-only CTX toolbook.

Arguments:
- `$1`: toolbook name, for example `glab`
- `$2`: markdown file path

Usage:
- `/ctx-toolbook-import glab docs/glab.md`

If `$1` or `$2` is missing, stop and show the usage above.

!`{{CTX_CMD}} memory import --from "$2" --scope "toolbook:$1" --source toolbook --prefix "toolbook.$1"`

Show the import result first.
Then say that future searches should use `/ctx-toolbook-search $1 "<query>"` instead of loading the whole manual into AGENTS.md."#,
        },
        HostActionTemplate {
            slug: "ctx-toolbook-search",
            description: "Toolbooks | Search a stored CLI/tool manual without prompt bloat",
            body: r#"Search an OpenCode-only CTX toolbook.

Arguments:
- `$1`: toolbook name, for example `glab`
- `$2`: quoted query, for example `"merge request create"`

Usage:
- `/ctx-toolbook-search glab "merge request create"`

If `$1` or `$2` is missing, stop and show the usage above.

!`{{CTX_CMD}} memory search "$2" --scope "toolbook:$1" --json`

Show only the matching directives in a compact, predictable format.
Do not summarize the full manual or add unrelated CLI flags."#,
        },
        HostActionTemplate {
            slug: "ctx-toolbook-list",
            description: "Toolbooks | List stored directives for one toolbook",
            body: r#"List an OpenCode-only CTX toolbook.

Arguments:
- `$1`: toolbook name, for example `glab`
- `$2`: optional limit

Usage:
- `/ctx-toolbook-list glab`
- `/ctx-toolbook-list glab 30`

If `$1` is missing, stop and show the usage above.

Run `{{CTX_CMD}} memory list --scope "toolbook:$1"` and add `--limit "$2"` only when a limit was provided.
Show the stored directives first, then add one short sentence about how to search them."#,
        },
        HostActionTemplate {
            slug: "ctx-toolbook-pack",
            description: "Toolbooks | Pack task context plus relevant toolbook guidance",
            body: r#"Pack task context while also retrieving relevant OpenCode-only CTX toolbook guidance.

Arguments:
- `$1`: toolbook name, for example `glab`
- `$2`: quoted task/query, for example `"create merge request for auth fix"`

Usage:
- `/ctx-toolbook-pack glab "create merge request for auth fix"`

If `$1` or `$2` is missing, stop and show the usage above.

Toolbook matches:
!`{{CTX_CMD}} memory search "$2" --scope "toolbook:$1" --json`

Task context:
!`{{CTX_CMD}} pack "$2" --json`

Show the relevant toolbook matches first, then print `compact_context`, then a single metadata line with `packed_tokens`, `reduction_pct`, and `pack_path`.
Do not load or restate the full manual."#,
        },
        HostActionTemplate {
            slug: "ctx-learn",
            description: "Learning | Store a reusable project lesson in graph memory",
            body: r#"Store a reusable OpenCode-only CTX lesson in graph memory.

Arguments:
- `$1`: memory key, for example `auth.refresh_regression`
- `$2`: quoted lesson body

Usage:
- `/ctx-learn auth.refresh_regression "When auth refresh fails, check token rotation and stale session flags first."`

If `$1` or `$2` is missing, stop and show the usage above.

!`{{CTX_CMD}} memory set "$1" "$2" --scope project --source learned`

Confirm the stored key first.
Then say it can be found later with `/ctx-memory-search <topic>`."#,
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

Run the matching `{{CTX_CMD}} benchmark memory-ab` command.
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
- `{{CTX_CMD}} benchmark memory-suite --spec <spec> --report-out <report>`
- include `--json-out <json>` when structured output is also needed

Rules:
- run only the exact CTX benchmark command
- do not infer KPIs from source files manually

Then summarize the suite KPIs and point to the generated report files."#,
        },
        HostActionTemplate {
            slug: "ctx-stats",
            description: "Benchmark | Show the latest CTX token and runtime stats",
            body: r#"Show the latest local CTX stats for this repository.

!`{{CTX_CMD}} --json stats`

Render exactly this compact markdown:
- `## 📈 CTX Stats`
- `**Latest Stats**`
- `**Takeaway**`

Show the stats payload first under `**Latest Stats**`.
Then add one short sentence summarizing the latest run."#,
        },
        HostActionTemplate {
            slug: "ctx-gain",
            description: "Benchmark | Show recent CTX token savings and biggest wins",
            body: r#"OpenCode-only CTX gain report for this repository.

!`{{CTX_CMD}} --json stats --history 20`

Render exactly this compact markdown:
- `## 💸 CTX Gain`
- `**Savings**`
- `**Top Queries**`
- `**Artifacts**`

Under `**Savings**`, include:
- `sampled_runs`
- `estimated_tokens_saved`
- `latest_reduction_pct`
- `average_reduction_pct`
- `max_reduction_pct`

Under `**Top Queries**`, list `top_queries`.

If `latest_pack_path` is present, show it under `**Artifacts**` on one compact line.
Keep any follow-up explanation to one short sentence."#,
        },
        HostActionTemplate {
            slug: "ctx-dashboard",
            description: "Benchmark | Show the local CTX dashboard snapshot",
            body: r#"CTX Dashboard snapshot.

Run the deterministic CTX dashboard command below and present its output as-is.

Rules:
- do not inspect files manually
- do not call subagents
- do not rewrite the dashboard into another format

!`{{CTX_CMD}} host-dashboard`"#,
        },
        HostActionTemplate {
            slug: "ctx-opencode-install",
            description: "Setup | Refresh CTX integration files for OpenCode",
            body: r#"Refresh the current repository's OpenCode integration.

!`{{CTX_CMD}} opencode install`

Then show the output and summarize which files were written or updated."#,
        },
        HostActionTemplate {
            slug: "ctx-mcp-serve",
            description: "MCP | Show or start the CTX MCP HTTP server",
            body: r#"Prepare the CTX MCP HTTP server for this repository.

Arguments:
- `$1`: optional port, default `8765`

If the user wants the server started in this session, run `{{CTX_CMD}} mcp serve --port <port>`.
Otherwise, show the exact command to run and explain that it is a long-running local process."#,
        },
        HostActionTemplate {
            slug: "ctx-mcp-stdio",
            description: "MCP | Show the CTX MCP stdio launch command",
            body: r#"Show the CTX MCP stdio launch command for the current repository.

Use the current repository root and explain how a host CLI can launch `{{CTX_CMD}} mcp stdio` locally."#,
        },
        HostActionTemplate {
            slug: "ctx-mcp-config-opencode",
            description: "MCP | Generate CTX MCP config for OpenCode",
            body: r#"Generate the CTX MCP config snippet for OpenCode.

!`{{CTX_CMD}} mcp config opencode`

Then show the output and explain how to use it."#,
        },
    ]
}

fn opencode_instruction_files(profile: OpencodeInstallProfile) -> Vec<(&'static str, String)> {
    vec![("ctx-host-first.md", host_first_instructions(profile))]
}

fn host_first_instructions(profile: OpencodeInstallProfile) -> String {
    match profile {
        OpencodeInstallProfile::Full => r#"# CTX Host-First Rules For OpenCode

CTX is the local context runtime for this repository.

## Primary Workflow

- Stay inside OpenCode for normal work.
- Install profile: `full`
- Prefer CTX slash commands and CTX MCP tools before broad file dumping.
- Keep the current OpenCode-selected model and agent in control.
- Do not revive wrapper-style workflows like `ctx wrap` or `ctx opencode run`.

## Automatic CTX Usage

For normal prompts, prefer CTX-first behavior:

1. If repository readiness is unclear, run `/ctx-doctor`.
2. If graph/index state is stale or missing, run `/ctx-index` or `/ctx-reindex`.
3. For code understanding, prefer `/ctx-retrieve`, `/ctx-read`, `/ctx-graph-query`, and CTX MCP tools before manually reading many files.
4. For debugging logs, prefer `/ctx-run`, and use `/ctx-prune-logs` when the user already has raw output or explicitly wants pruning only.
5. For debugging diffs, prefer `/ctx-prune-diff`.
6. For project habits or persistent rules, bootstrap markdown habits once with `/ctx-memory-bootstrap`, then prefer `/ctx-memory-search`, `/ctx-memory-list`, `/ctx-memory-get`, and `/ctx-memory-set` instead of large markdown habit files.
7. For context construction, prefer `/ctx-pack` or `/ctx-ask` before assembling large prompts manually.
8. For prompt scaffolding, use `/ctx-hook`.
9. For ambiguity about likely scope or intent, use `/ctx-explain`.
10. For implementation planning, use `/ctx-plan` to combine retrieval, graph, memory, and pack signals before editing.
11. For quick before-vs-packed context density, use `/ctx-compare`.
12. For a local snapshot of savings, cache reuse, and runtime health, use `/ctx-dashboard`.
13. For recent token savings and biggest pack wins, use `/ctx-gain`.
14. For large CLI manuals or tool cheat sheets, import them once with `/ctx-toolbook-import`, then use `/ctx-toolbook-search` or `/ctx-toolbook-pack` instead of putting manuals in AGENTS.md.
15. For reusable lessons learned during work, use `/ctx-learn`.
16. For validation of graph-memory token savings, use `/ctx-benchmark-memory-ab` or `/ctx-benchmark-memory-suite`.

## Memory And Rules

- Treat graph memory as the primary structured replacement for AGENTS-style project habits.
- Use `/ctx-memory-bootstrap` to migrate conventional markdown files into graph memory without leaving OpenCode.
- Only export markdown memory when compatibility or auditing is explicitly needed.
- Prefer updating graph memory directives over adding new large instruction files.
- Treat toolbooks as scoped graph memory for external manuals, not as project-wide rules.

## Retrieval Discipline

- Start with the smallest high-signal CTX command that answers the task.
- Avoid loading many files when CTX already exposes the relevant graph or retrieval context.
- Use CTX compact context before broad scans whenever the task involves debugging, implementation, or review.

## Safety

- Respect CTX privacy defaults and sensitive file blocking behavior.
- Keep all project data local unless the host or the user explicitly chooses otherwise.
"#
        .to_string(),
        OpencodeInstallProfile::Core => r#"# CTX Host-First Rules For OpenCode

CTX is the local context runtime for this repository.

## Primary Workflow

- Stay inside OpenCode for normal work.
- Install profile: `core`
- Prefer the lean CTX slash-command surface before broad file dumping.
- Keep the current OpenCode-selected model and agent in control.
- Do not revive wrapper-style workflows like `ctx wrap` or `ctx opencode run`.
- If you need the wider command surface later, rerun `ctx opencode install --profile full`.

## Automatic CTX Usage

For normal prompts with the core profile, prefer this order:

1. If repository readiness is unclear, run `/ctx-doctor`.
2. For code understanding, start with `/ctx-retrieve`.
3. For implementation planning, use `/ctx-plan`.
4. For context construction, use `/ctx-pack`.
5. For debugging logs, prefer `/ctx-run`, and use `/ctx-prune-logs` when the user already has raw output or explicitly wants pruning only.
6. For local measurements, use `/ctx-stats` or `/ctx-gain`.

## Upgrade Path

- The core profile intentionally keeps only the smallest daily workflow surface.
- To unlock read cache tools, memory workflows, toolbooks, benchmarks, and the dashboard, rerun `ctx opencode install --profile full`.

## Safety

- Respect CTX privacy defaults and sensitive file blocking behavior.
- Keep all project data local unless the host or the user explicitly chooses otherwise.
"#
        .to_string(),
    }
}
