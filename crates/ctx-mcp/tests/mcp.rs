use std::io::Cursor;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;
use std::time::Duration;

use ctx_core::init_repo;
use ctx_mcp::{
    McpServerConfig, default_tools, mcp_banner, process_rpc_message, serve_http, serve_stdio_with,
};
use serde_json::{Value, json};
use tempfile::tempdir;

#[test]
fn exposes_required_pdf_tools() {
    let tools = default_tools();
    for required in [
        "get_relevant_context",
        "read_path",
        "project_map",
        "search_symbols",
        "related_failures",
        "recent_decisions",
        "get_compact_diff",
        "memory_list",
        "memory_set",
        "memory_get",
        "memory_search",
        "memory_delete",
        "memory_import_markdown",
        "memory_bootstrap_markdown",
    ] {
        assert!(
            tools.iter().any(|t| t.name == required),
            "missing {required}"
        );
    }
}

#[test]
fn banner_mentions_localhost_security_model() {
    let banner = mcp_banner(8765);
    assert!(banner.contains("127.0.0.1"));
    assert!(banner.contains("8765"));
}

#[test]
fn initialize_rpc_returns_server_info() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    let response = roundtrip_once(
        tmp.path(),
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"]["serverInfo"]["name"] == "ctx-mcp");
}

#[test]
fn tools_list_rpc_returns_required_tools() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    let response = roundtrip_once(
        tmp.path(),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
    );

    let names = response["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();

    assert!(names.contains(&"get_relevant_context".to_string()));
    assert!(names.contains(&"project_map".to_string()));
    assert!(
        response["result"]["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .all(|tool| tool["inputSchema"].is_object())
    );
}

#[test]
fn stdio_rpc_message_uses_same_initialize_contract() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let cfg = McpServerConfig {
        repo_root: tmp.path().to_path_buf(),
        port: 8765,
        once: false,
    };

    let response = process_rpc_message(
        &cfg,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    )
    .expect("initialize should return a response");
    let value: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(value["jsonrpc"], "2.0");
    assert_eq!(value["id"], 1);
    assert_eq!(value["result"]["serverInfo"]["name"], "ctx-mcp");
}

#[test]
fn initialized_notification_returns_no_response_and_tools_list_still_works() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let cfg = McpServerConfig {
        repo_root: tmp.path().to_path_buf(),
        port: 8765,
        once: false,
    };

    let initialize = process_rpc_message(
        &cfg,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    )
    .expect("initialize should return a response");
    let initialize_value: Value = serde_json::from_str(&initialize).expect("json response");
    assert_eq!(initialize_value["result"]["serverInfo"]["name"], "ctx-mcp");

    let initialized = process_rpc_message(
        &cfg,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"#,
    );
    assert!(
        initialized.is_none(),
        "notifications must not emit JSON-RPC responses"
    );

    let tools_list = process_rpc_message(
        &cfg,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
    )
    .expect("tools/list should return a response");
    let tools_value: Value = serde_json::from_str(&tools_list).expect("json response");
    assert!(
        tools_value["result"]["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .any(|tool| tool["name"] == "get_relevant_context")
    );
}

#[test]
fn bare_json_stdio_roundtrip_emits_plain_json_response() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let cfg = McpServerConfig {
        repo_root: tmp.path().to_path_buf(),
        port: 8765,
        once: false,
    };

    let mut input = Cursor::new(
        b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-11-25\"}}\n"
            .to_vec(),
    );
    let mut output = Vec::new();

    serve_stdio_with(&cfg, &mut input, &mut output).expect("serve stdio");
    let text = String::from_utf8(output).expect("utf8 output");

    assert!(text.trim_start().starts_with('{'));
    assert!(!text.contains("Content-Length:"));

    let response: Value = serde_json::from_str(text.trim()).expect("json response");
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["protocolVersion"], "2025-11-25");
}

#[test]
fn content_length_stdio_roundtrip_emits_framed_response() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let cfg = McpServerConfig {
        repo_root: tmp.path().to_path_buf(),
        port: 8765,
        once: false,
    };

    let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let request = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let mut input = Cursor::new(request.into_bytes());
    let mut output = Vec::new();

    serve_stdio_with(&cfg, &mut input, &mut output).expect("serve stdio");
    let text = String::from_utf8(output).expect("utf8 output");

    assert!(text.starts_with("Content-Length: "));
    let (_, response_body) = text.split_once("\r\n\r\n").expect("framed stdio response");
    let response: Value = serde_json::from_str(response_body).expect("json response");
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["serverInfo"]["name"], "ctx-mcp");
}

#[test]
fn tools_call_get_relevant_context_returns_pack_data() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    let response = roundtrip_once(
        tmp.path(),
        json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"tools/call",
            "params":{
                "name":"get_relevant_context",
                "arguments":{
                    "query":"fix auth failure",
                    "budget":120
                }
            }
        }),
    );

    assert!(response["result"]["content"].is_array());
    assert!(
        response["result"]["structuredContent"]["packed_tokens"]
            .as_u64()
            .unwrap_or(0)
            > 0
    );
    assert!(
        response["result"]["structuredContent"]["compact_context"]
            .as_str()
            .unwrap_or_default()
            .contains("query:")
    );
}

#[test]
fn tools_call_read_path_returns_cached_read_shape() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    std::fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    std::fs::write(
        tmp.path().join("src/auth.ts"),
        "export function rotateRefreshToken(token: string) {\n  return token.trim();\n}\n",
    )
    .expect("write file");

    let response = roundtrip_once(
        tmp.path(),
        json!({
            "jsonrpc":"2.0",
            "id":30,
            "method":"tools/call",
            "params":{
                "name":"read_path",
                "arguments":{
                    "path":"src/auth.ts",
                    "mode":"outline"
                }
            }
        }),
    );

    assert!(response["result"]["content"].is_array());
    assert_eq!(
        response["result"]["structuredContent"]["mode"]
            .as_str()
            .unwrap_or_default(),
        "outline"
    );
    assert_eq!(
        response["result"]["structuredContent"]["path"]
            .as_str()
            .unwrap_or_default(),
        "src/auth.ts"
    );
    assert!(
        response["result"]["structuredContent"]["output"]
            .as_str()
            .unwrap_or_default()
            .contains("rotateRefreshToken")
    );
}

#[test]
fn tools_call_memory_set_and_list_work() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    let set_response = roundtrip_once(
        tmp.path(),
        json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"tools/call",
            "params":{
                "name":"memory_set",
                "arguments":{
                    "key":"testing.always_run",
                    "body":"Run tests before completion.",
                    "scope":"project",
                    "source":"manual"
                }
            }
        }),
    );
    assert_eq!(
        set_response["result"]["structuredContent"]["directive"]["key"]
            .as_str()
            .unwrap_or_default(),
        "testing.always_run"
    );

    let list_response = roundtrip_once(
        tmp.path(),
        json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"tools/call",
            "params":{
                "name":"memory_list",
                "arguments":{"scope":"project","limit":10}
            }
        }),
    );
    assert!(list_response["result"]["content"].is_array());
    let directives = list_response["result"]["structuredContent"]["directives"]
        .as_array()
        .expect("directives array");
    assert!(!directives.is_empty());
}

#[test]
fn tools_call_memory_bootstrap_and_search_work() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    std::fs::write(
        tmp.path().join("AGENTS.md"),
        "# Rules\n- Run targeted tests before completion.\n- Fix auth root cause before merge.\n",
    )
    .expect("write agents");

    let bootstrap = roundtrip_once(
        tmp.path(),
        json!({
            "jsonrpc":"2.0",
            "id":7,
            "method":"tools/call",
            "params":{
                "name":"memory_bootstrap_markdown",
                "arguments":{}
            }
        }),
    );
    assert!(bootstrap["result"]["content"].is_array());
    assert_eq!(
        bootstrap["result"]["structuredContent"]["report"]["imported_files"],
        1
    );
    assert!(
        bootstrap["result"]["structuredContent"]["report"]["imported_directives"]
            .as_u64()
            .unwrap_or(0)
            >= 2
    );

    let search = roundtrip_once(
        tmp.path(),
        json!({
            "jsonrpc":"2.0",
            "id":8,
            "method":"tools/call",
            "params":{
                "name":"memory_search",
                "arguments":{
                    "query":"auth root cause",
                    "scope":"project",
                    "limit":10
                }
            }
        }),
    );
    let directives = search["result"]["structuredContent"]["directives"]
        .as_array()
        .expect("directives array");
    assert!(!directives.is_empty());
    assert!(
        directives[0]["body"]
            .as_str()
            .unwrap_or_default()
            .contains("Fix auth root cause")
    );
}

fn roundtrip_once(repo_root: &Path, rpc_payload: Value) -> Value {
    let port = free_port();
    let cfg = McpServerConfig {
        repo_root: repo_root.to_path_buf(),
        port,
        once: true,
    };

    let handle = thread::spawn(move || serve_http(cfg));

    wait_for_port(port);

    let response = send_rpc_request(port, &rpc_payload);

    let server_result = handle.join().expect("thread join");
    server_result.expect("server should exit cleanly");

    response
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    port
}

fn wait_for_port(port: u16) {
    for _ in 0..20 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("server did not start in time on port {port}");
}

fn send_rpc_request(port: u16, payload: &Value) -> Value {
    let body = payload.to_string();
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );

    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream.write_all(request.as_bytes()).expect("write request");
    stream.flush().expect("flush");

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read response body");

    let body = response
        .split("\r\n\r\n")
        .nth(1)
        .expect("http body should exist");

    serde_json::from_str(body).expect("valid json response")
}
