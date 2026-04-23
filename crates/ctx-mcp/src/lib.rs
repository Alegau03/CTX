use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use ctx_core::{run_graph_query, run_pack, run_prune_diff};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub repo_root: PathBuf,
    pub port: u16,
    pub once: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpTool {
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Deserialize)]
struct RpcRequest {
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
struct ResourceDescriptor {
    uri: &'static str,
    name: &'static str,
    description: &'static str,
    #[serde(rename = "mimeType")]
    mime_type: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct ProjectMapEntry {
    path: String,
    kind: String,
}

pub fn default_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "get_relevant_context",
            description: "Return compact context for current query",
        },
        McpTool {
            name: "project_map",
            description: "Return top-level repository map",
        },
        McpTool {
            name: "search_symbols",
            description: "Search indexed symbols by keyword",
        },
        McpTool {
            name: "related_failures",
            description: "Return failures connected to symbols/tasks",
        },
        McpTool {
            name: "recent_decisions",
            description: "Return recent pruning/decision notes",
        },
        McpTool {
            name: "get_compact_diff",
            description: "Return query-focused compact diff",
        },
    ]
}

pub fn mcp_banner(port: u16) -> String {
    format!("CTX MCP server listening on 127.0.0.1:{port} (localhost-only trust boundary)")
}

pub fn serve_http(cfg: McpServerConfig) -> Result<()> {
    let addr = format!("127.0.0.1:{}", cfg.port);
    let server = Server::http(&addr).map_err(|err| anyhow!("failed to bind {addr}: {err}"))?;

    eprintln!("{}", mcp_banner(cfg.port));

    for request in server.incoming_requests() {
        if let Err(err) = handle_http_request(&cfg, request) {
            eprintln!("mcp request error: {err:#}");
        }

        if cfg.once {
            break;
        }
    }

    Ok(())
}

fn handle_http_request(cfg: &McpServerConfig, mut request: Request) -> Result<()> {
    match (request.method(), request.url()) {
        (&Method::Get, "/health") => {
            let payload = json!({"status":"ok","service":"ctx-mcp","port":cfg.port});
            respond_json(request, StatusCode(200), payload)
        }
        (&Method::Post, "/rpc") => {
            let mut body = String::new();
            request
                .as_reader()
                .read_to_string(&mut body)
                .context("failed to read request body")?;

            let rpc: RpcRequest = serde_json::from_str(&body)
                .with_context(|| format!("invalid rpc json body: {body}"))?;
            let id = rpc.id.unwrap_or(Value::Null);

            let response = match process_rpc(cfg, &rpc.method, rpc.params.as_ref()) {
                Ok(result) => rpc_success(id, result),
                Err(err) => rpc_error(id, -32000, &format!("{err:#}")),
            };

            respond_json(request, StatusCode(200), response)
        }
        _ => {
            let payload = json!({"error":"not found"});
            respond_json(request, StatusCode(404), payload)
        }
    }
}

fn process_rpc(cfg: &McpServerConfig, method: &str, params: Option<&Value>) -> Result<Value> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion":"2025-03-26",
            "serverInfo":{"name":"ctx-mcp","version":"0.1.0"},
            "capabilities":{
                "tools":{"listChanged":false},
                "resources":{"subscribe":false,"listChanged":false}
            }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": default_tools()})),
        "resources/list" => Ok(json!({"resources": default_resources()})),
        "resources/read" => resources_read(cfg, params),
        "tools/call" => tools_call(cfg, params),
        _ => bail!("unknown rpc method: {method}"),
    }
}

fn tools_call(cfg: &McpServerConfig, params: Option<&Value>) -> Result<Value> {
    let params = params
        .and_then(Value::as_object)
        .context("tools/call expects object params")?;

    let name = params
        .get("name")
        .and_then(Value::as_str)
        .context("tools/call missing params.name")?;

    let args = params
        .get("arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    match name {
        "get_relevant_context" => {
            let query = args
                .get("query")
                .and_then(Value::as_str)
                .context("get_relevant_context requires arguments.query")?;
            let budget = args
                .get("budget")
                .and_then(Value::as_u64)
                .map(|v| v as usize);

            let attach = args
                .get("attach")
                .and_then(Value::as_str)
                .map(|raw| resolve_path(&cfg.repo_root, raw));

            let pack = run_pack(&cfg.repo_root, query, budget, attach.as_deref())?;
            serde_json::to_value(pack).context("failed to serialize pack result")
        }
        "project_map" => {
            let depth = args.get("depth").and_then(Value::as_u64).unwrap_or(2) as usize;
            let map = build_project_map(&cfg.repo_root, depth)?;
            Ok(json!({"entries": map}))
        }
        "search_symbols" => {
            let query = args
                .get("query")
                .and_then(Value::as_str)
                .context("search_symbols requires arguments.query")?;
            let matches = run_graph_query(&cfg.repo_root, query)?;
            Ok(json!({"matches": matches}))
        }
        "related_failures" => {
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
            let failures = read_related_failures(&cfg.repo_root, limit)?;
            Ok(json!({"failures": failures}))
        }
        "recent_decisions" => {
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
            let decisions = read_recent_decisions(&cfg.repo_root, limit)?;
            Ok(json!({"decisions": decisions}))
        }
        "get_compact_diff" => {
            let query = args
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let input = args
                .get("input")
                .and_then(Value::as_str)
                .context("get_compact_diff requires arguments.input")?;
            let max_lines = args.get("max_lines").and_then(Value::as_u64).unwrap_or(200) as usize;
            let report = run_prune_diff(input, query, max_lines);
            serde_json::to_value(report).context("failed to serialize diff report")
        }
        _ => bail!("unknown tool: {name}"),
    }
}

fn resources_read(cfg: &McpServerConfig, params: Option<&Value>) -> Result<Value> {
    let uri = params
        .and_then(Value::as_object)
        .and_then(|p| p.get("uri"))
        .and_then(Value::as_str)
        .context("resources/read requires params.uri")?;

    match uri {
        "ctx://project-map" => {
            let map = build_project_map(&cfg.repo_root, 2)?;
            let text = serde_json::to_string_pretty(&map).context("serialize project map")?;
            Ok(json!({
                "contents":[{
                    "uri":"ctx://project-map",
                    "mimeType":"application/json",
                    "text": text
                }]
            }))
        }
        "ctx://recent-decisions" => {
            let decisions = read_recent_decisions(&cfg.repo_root, 20)?;
            let text = serde_json::to_string_pretty(&decisions).context("serialize decisions")?;
            Ok(json!({
                "contents":[{
                    "uri":"ctx://recent-decisions",
                    "mimeType":"application/json",
                    "text": text
                }]
            }))
        }
        _ => bail!("unknown resource uri: {uri}"),
    }
}

fn default_resources() -> Vec<ResourceDescriptor> {
    vec![
        ResourceDescriptor {
            uri: "ctx://project-map",
            name: "project_map",
            description: "Top-level project map",
            mime_type: "application/json",
        },
        ResourceDescriptor {
            uri: "ctx://recent-decisions",
            name: "recent_decisions",
            description: "Recent pruning and decision log entries",
            mime_type: "application/json",
        },
    ]
}

fn build_project_map(repo_root: &Path, depth: usize) -> Result<Vec<ProjectMapEntry>> {
    let mut entries = Vec::new();

    for entry in WalkDir::new(repo_root)
        .max_depth(depth)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !is_ignored(e))
        .filter_map(std::result::Result::ok)
    {
        let rel = entry
            .path()
            .strip_prefix(repo_root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .to_string();

        entries.push(ProjectMapEntry {
            path: rel,
            kind: if entry.file_type().is_dir() {
                "dir".to_string()
            } else {
                "file".to_string()
            },
        });
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

fn is_ignored(entry: &DirEntry) -> bool {
    entry
        .path()
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .any(|segment| matches!(segment, ".git" | ".ctx" | "target" | "node_modules"))
}

fn read_recent_decisions(repo_root: &Path, limit: usize) -> Result<Vec<String>> {
    let audit_path = repo_root.join(".ctx/audit.log");
    if !audit_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&audit_path)
        .with_context(|| format!("failed to read {}", audit_path.display()))?;
    let mut lines = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }

    Ok(lines)
}

fn read_related_failures(repo_root: &Path, limit: usize) -> Result<Vec<String>> {
    let db_path = repo_root.join(".ctx/graph.db");
    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let conn = rusqlite::Connection::open(db_path).context("failed to open graph db")?;
    let mut stmt = conn
        .prepare("SELECT message FROM failures ORDER BY id DESC LIMIT ?1")
        .context("failed to prepare failures query")?;

    let rows = stmt
        .query_map([limit as i64], |row| row.get::<_, String>(0))
        .context("failed to query failures")?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.context("failed to decode failure row")?);
    }

    Ok(out)
}

fn resolve_path(repo_root: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    }
}

fn rpc_success(id: Value, result: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":result})
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc":"2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        }
    })
}

fn respond_json(request: Request, status: StatusCode, body: Value) -> Result<()> {
    let payload = serde_json::to_string(&body).context("failed to serialize response")?;
    let content_type =
        Header::from_bytes(b"Content-Type".as_slice(), b"application/json".as_slice())
            .map_err(|_| anyhow!("failed to create content-type header"))?;

    let response = Response::from_string(payload)
        .with_status_code(status)
        .with_header(content_type);

    request.respond(response).context("failed to respond")
}
