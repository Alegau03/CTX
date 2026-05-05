use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use ctx_core::{
    ReadMode, run_graph_query, run_memory_bootstrap_markdown, run_memory_delete, run_memory_get,
    run_memory_import_markdown, run_memory_list, run_memory_search, run_memory_set, run_pack,
    run_prune_diff, run_read,
};
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
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StdioEnvelope {
    ContentLength,
    BareJson,
}

#[derive(Debug, Clone)]
struct StdioRpcMessage {
    body: String,
    envelope: StdioEnvelope,
}

pub fn default_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "get_relevant_context",
            description: "Return compact context for current query",
            input_schema: json_schema(
                &[
                    ("query", "string"),
                    ("budget", "integer"),
                    ("attach", "string"),
                ],
                &["query"],
            ),
        },
        McpTool {
            name: "read_path",
            description: "Read one repository file using full, outline, or digest mode with reread cache awareness",
            input_schema: json_schema(&[("path", "string"), ("mode", "string")], &["path"]),
        },
        McpTool {
            name: "project_map",
            description: "Return top-level repository map",
            input_schema: json_schema(&[("depth", "integer")], &[]),
        },
        McpTool {
            name: "search_symbols",
            description: "Search indexed symbols by keyword",
            input_schema: json_schema(&[("query", "string")], &["query"]),
        },
        McpTool {
            name: "related_failures",
            description: "Return failures connected to symbols/tasks",
            input_schema: json_schema(&[("limit", "integer")], &[]),
        },
        McpTool {
            name: "recent_decisions",
            description: "Return recent pruning/decision notes",
            input_schema: json_schema(&[("limit", "integer")], &[]),
        },
        McpTool {
            name: "get_compact_diff",
            description: "Return query-focused compact diff",
            input_schema: json_schema(
                &[
                    ("input", "string"),
                    ("query", "string"),
                    ("max_lines", "integer"),
                ],
                &["input"],
            ),
        },
        McpTool {
            name: "memory_list",
            description: "List graph-backed memory directives",
            input_schema: json_schema(&[("scope", "string"), ("limit", "integer")], &[]),
        },
        McpTool {
            name: "memory_set",
            description: "Create/update one graph memory directive",
            input_schema: json_schema(
                &[
                    ("key", "string"),
                    ("body", "string"),
                    ("scope", "string"),
                    ("source", "string"),
                ],
                &["key", "body"],
            ),
        },
        McpTool {
            name: "memory_get",
            description: "Get one graph memory directive",
            input_schema: json_schema(&[("key", "string")], &["key"]),
        },
        McpTool {
            name: "memory_search",
            description: "Search graph memory directives by topic",
            input_schema: json_schema(
                &[
                    ("query", "string"),
                    ("scope", "string"),
                    ("limit", "integer"),
                ],
                &["query"],
            ),
        },
        McpTool {
            name: "memory_delete",
            description: "Delete one graph memory directive",
            input_schema: json_schema(&[("key", "string")], &["key"]),
        },
        McpTool {
            name: "memory_import_markdown",
            description: "Import AGENTS/CLAUDE/CODEX markdown rules into graph memory",
            input_schema: json_schema(
                &[
                    ("path", "string"),
                    ("scope", "string"),
                    ("source", "string"),
                    ("prefix", "string"),
                ],
                &["path"],
            ),
        },
        McpTool {
            name: "memory_bootstrap_markdown",
            description: "Auto-import conventional markdown rule files into graph memory",
            input_schema: json_schema(
                &[
                    ("paths", "array"),
                    ("scope", "string"),
                    ("source", "string"),
                ],
                &[],
            ),
        },
    ]
}

fn json_schema(properties: &[(&str, &str)], required: &[&str]) -> Value {
    let properties = properties
        .iter()
        .map(|(name, kind)| {
            let schema = if *kind == "array" {
                json!({"type":"array","items":{"type":"string"}})
            } else {
                json!({"type":kind})
            };
            ((*name).to_string(), schema)
        })
        .collect::<serde_json::Map<String, Value>>();

    json!({
        "type":"object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
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

pub fn serve_stdio(cfg: McpServerConfig) -> Result<()> {
    let stdin = io::stdin();
    let mut stdin = io::BufReader::new(stdin.lock());
    let mut stdout = io::stdout();

    serve_stdio_with(&cfg, &mut stdin, &mut stdout)
}

pub fn serve_stdio_with<R: BufRead, W: Write>(
    cfg: &McpServerConfig,
    reader: &mut R,
    writer: &mut W,
) -> Result<()> {
    while let Some(message) = read_stdio_rpc_message(reader)? {
        if let Some(response) = process_rpc_message(cfg, &message.body) {
            write_stdio_rpc_response(writer, &response, message.envelope)?;
        }
    }

    Ok(())
}

pub fn process_rpc_message(cfg: &McpServerConfig, body: &str) -> Option<String> {
    let parsed = serde_json::from_str::<RpcRequest>(body);
    match parsed {
        Ok(rpc) => {
            let is_notification = rpc.id.is_none();
            if is_notification {
                let _ = process_rpc(cfg, &rpc.method, rpc.params.as_ref());
                None
            } else {
                let id = rpc.id.unwrap_or(Value::Null);
                match process_rpc(cfg, &rpc.method, rpc.params.as_ref()) {
                    Ok(result) => Some(rpc_success(id, result).to_string()),
                    Err(err) => Some(rpc_error(id, -32000, &format!("{err:#}")).to_string()),
                }
            }
        }
        Err(err) => Some(
            rpc_error(
                Value::Null,
                -32700,
                &format!("invalid rpc json body: {err}"),
            )
            .to_string(),
        ),
    }
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

            match process_rpc_message(cfg, &body) {
                Some(response) => {
                    let response: Value =
                        serde_json::from_str(&response).context("rpc response")?;
                    respond_json(request, StatusCode(200), response)
                }
                None => {
                    let response = Response::empty(StatusCode(204));
                    request
                        .respond(response)
                        .context("failed to send empty rpc response")
                }
            }
        }
        _ => {
            let payload = json!({"error":"not found"});
            respond_json(request, StatusCode(404), payload)
        }
    }
}

fn read_stdio_rpc_message<R: BufRead>(reader: &mut R) -> Result<Option<StdioRpcMessage>> {
    let mut first_line = String::new();
    loop {
        first_line.clear();
        let bytes = reader
            .read_line(&mut first_line)
            .context("failed to read stdio rpc line")?;
        if bytes == 0 {
            return Ok(None);
        }
        if !first_line.trim().is_empty() {
            break;
        }
    }

    if first_line.trim_start().starts_with('{') {
        return Ok(Some(StdioRpcMessage {
            body: first_line.trim_end().to_string(),
            envelope: StdioEnvelope::BareJson,
        }));
    }

    let mut content_length = None;
    let mut header_line = first_line;
    loop {
        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            break;
        }

        let (name, value) = trimmed
            .split_once(':')
            .ok_or_else(|| anyhow!("invalid stdio rpc header: {trimmed}"))?;
        if name.eq_ignore_ascii_case("Content-Length") {
            content_length = Some(value.trim().parse::<usize>().with_context(|| {
                format!("invalid Content-Length header value: {}", value.trim())
            })?);
        }

        header_line = String::new();
        let bytes = reader
            .read_line(&mut header_line)
            .context("failed to read stdio rpc header")?;
        if bytes == 0 {
            bail!("unexpected EOF while reading stdio rpc headers");
        }
    }

    let content_length =
        content_length.context("missing Content-Length header in stdio rpc request")?;
    let mut payload = vec![0u8; content_length];
    reader
        .read_exact(&mut payload)
        .context("failed to read stdio rpc body")?;

    String::from_utf8(payload)
        .context("stdio rpc body is not valid UTF-8")
        .map(|body| {
            Some(StdioRpcMessage {
                body,
                envelope: StdioEnvelope::ContentLength,
            })
        })
}

fn write_stdio_rpc_response<W: Write>(
    writer: &mut W,
    response: &str,
    envelope: StdioEnvelope,
) -> Result<()> {
    match envelope {
        StdioEnvelope::ContentLength => {
            write!(
                writer,
                "Content-Length: {}\r\n\r\n{}",
                response.len(),
                response
            )
            .context("failed to write stdio rpc response")?;
        }
        StdioEnvelope::BareJson => {
            writeln!(writer, "{response}").context("failed to write stdio rpc response")?;
        }
    }
    writer
        .flush()
        .context("failed to flush stdio rpc response")?;
    Ok(())
}

fn process_rpc(cfg: &McpServerConfig, method: &str, params: Option<&Value>) -> Result<Value> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": initialize_protocol_version(params),
            "serverInfo":{"name":"ctx-mcp","version":env!("CARGO_PKG_VERSION")},
            "capabilities":{
                "tools":{"listChanged":false},
                "resources":{"subscribe":false,"listChanged":false}
            }
        })),
        "notifications/initialized" => Ok(json!({})),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": default_tools()})),
        "resources/list" => Ok(json!({"resources": default_resources()})),
        "resources/read" => resources_read(cfg, params),
        "tools/call" => tools_call(cfg, params),
        _ => bail!("unknown rpc method: {method}"),
    }
}

fn initialize_protocol_version(params: Option<&Value>) -> String {
    params
        .and_then(Value::as_object)
        .and_then(|items| items.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or("2025-03-26")
        .to_string()
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

    let result = match name {
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
        "read_path" => {
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .context("read_path requires arguments.path")?;
            let mode = args
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("digest")
                .parse::<ReadMode>()?;
            let report = run_read(&cfg.repo_root, path, mode)?;
            serde_json::to_value(report).context("failed to serialize read report")
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
        "memory_list" => {
            let scope = args.get("scope").and_then(Value::as_str);
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
            let directives = run_memory_list(&cfg.repo_root, scope, limit)?;
            Ok(json!({"directives": directives}))
        }
        "memory_set" => {
            let key = args
                .get("key")
                .and_then(Value::as_str)
                .context("memory_set requires arguments.key")?;
            let body = args
                .get("body")
                .and_then(Value::as_str)
                .context("memory_set requires arguments.body")?;
            let scope = args
                .get("scope")
                .and_then(Value::as_str)
                .unwrap_or("project");
            let source = args
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("manual");
            let directive = run_memory_set(&cfg.repo_root, key, body, scope, source)?;
            Ok(json!({"directive": directive}))
        }
        "memory_get" => {
            let key = args
                .get("key")
                .and_then(Value::as_str)
                .context("memory_get requires arguments.key")?;
            let directive = run_memory_get(&cfg.repo_root, key)?;
            Ok(json!({"directive": directive}))
        }
        "memory_search" => {
            let query = args
                .get("query")
                .and_then(Value::as_str)
                .context("memory_search requires arguments.query")?;
            let scope = args.get("scope").and_then(Value::as_str);
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
            let directives = run_memory_search(&cfg.repo_root, query, scope, limit)?;
            Ok(json!({"directives": directives}))
        }
        "memory_delete" => {
            let key = args
                .get("key")
                .and_then(Value::as_str)
                .context("memory_delete requires arguments.key")?;
            let deleted = run_memory_delete(&cfg.repo_root, key)?;
            Ok(json!({"deleted": deleted}))
        }
        "memory_import_markdown" => {
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .context("memory_import_markdown requires arguments.path")?;
            let scope = args
                .get("scope")
                .and_then(Value::as_str)
                .unwrap_or("project");
            let source = args
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("markdown");
            let prefix = args.get("prefix").and_then(Value::as_str);
            let report = run_memory_import_markdown(
                &cfg.repo_root,
                &resolve_path(&cfg.repo_root, path),
                scope,
                source,
                prefix,
            )?;
            Ok(json!({"report": report}))
        }
        "memory_bootstrap_markdown" => {
            let paths = args
                .get("paths")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|item| resolve_path(&cfg.repo_root, item))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let scope = args
                .get("scope")
                .and_then(Value::as_str)
                .unwrap_or("project");
            let source = args
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("markdown");
            let report = run_memory_bootstrap_markdown(&cfg.repo_root, &paths, scope, source)?;
            Ok(json!({"report": report}))
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
    }?;

    wrap_tool_result(result)
}

fn wrap_tool_result(value: Value) -> Result<Value> {
    let text = serde_json::to_string_pretty(&value).context("failed to serialize tool result")?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": text
        }],
        "structuredContent": value,
        "isError": false
    }))
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
        "ctx://memory-directives" => {
            let directives = run_memory_list(&cfg.repo_root, None, 100)?;
            let text = serde_json::to_string_pretty(&directives).context("serialize directives")?;
            Ok(json!({
                "contents":[{
                    "uri":"ctx://memory-directives",
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
        ResourceDescriptor {
            uri: "ctx://memory-directives",
            name: "memory_directives",
            description: "Graph-backed operational directives replacing markdown habit files",
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
