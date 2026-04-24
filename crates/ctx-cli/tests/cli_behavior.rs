use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn init_creates_ctx_config() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join(".ctx/config.toml").exists());
}

#[test]
fn doctor_reports_missing_first_run_state_and_next_step() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("doctor")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("CTX Doctor"))
        .stdout(predicate::str::contains("config: missing"))
        .stdout(predicate::str::contains("next: ctx init"));
}

#[test]
fn doctor_reports_ready_repo_after_init() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("doctor")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("config: ok"))
        .stdout(predicate::str::contains("graph: ok"))
        .stdout(predicate::str::contains("audit_log: ok"))
        .stdout(predicate::str::contains("local_only: true"))
        .stdout(predicate::str::contains("remote_upload_enabled: false"))
        .stdout(predicate::str::contains("next: ctx index"));
}

#[test]
fn prune_logs_reads_stdin_and_outputs_error_lines() {
    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["prune", "logs"])
        .write_stdin("PASS ok\nERROR broken\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("ERROR broken"));
}

#[test]
fn prune_diff_accepts_query_flag_and_keeps_matching_hunks() {
    let diff = r#"
diff --git a/src/auth.rs b/src/auth.rs
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,1 +1,1 @@
-fn old_auth() {}
+fn validate_refresh_token() {}
diff --git a/src/other.rs b/src/other.rs
--- a/src/other.rs
+++ b/src/other.rs
@@ -1,1 +1,1 @@
-fn old() {}
+fn noop() {}
"#;

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["prune", "diff", "--query", "refresh token"])
        .write_stdin(diff)
        .assert()
        .success()
        .stdout(predicate::str::contains("validate_refresh_token"))
        .stdout(predicate::str::contains("noop").not());
}

#[test]
fn pack_json_outputs_expected_shape() {
    let tmp = tempdir().expect("tempdir");
    fs::write(tmp.path().join("fail.txt"), "Traceback: boom").expect("write");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args([
            "pack",
            "fix auth",
            "--json",
            "--attach",
            tmp.path().join("fail.txt").to_string_lossy().as_ref(),
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("packed_tokens"));
}

#[test]
fn explain_returns_intent_information() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["explain", "fix failing pytest"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("intent: debug"));
}

#[test]
fn stats_shows_latest_snapshot_after_pack() {
    let tmp = tempdir().expect("tempdir");
    fs::write(tmp.path().join("fail.txt"), "Traceback: boom").expect("write");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args([
            "pack",
            "fix auth",
            "--attach",
            tmp.path().join("fail.txt").to_string_lossy().as_ref(),
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("stats")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("packed_tokens"));
}

#[test]
fn claude_wrapper_uses_real_adapter_path_and_fallback_output() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["claude", "explain flaky test"])
        .current_dir(tmp.path())
        .env("PATH", "")
        .assert()
        .success()
        .stdout(predicate::str::contains("adapter=claude"))
        .stdout(predicate::str::contains("command=claude -p"))
        .stdout(predicate::str::contains("[CTX COMPACT CONTEXT]"));

    assert!(tmp.path().join(".ctx/stats/latest.json").exists());
}

#[test]
fn adapter_wrapper_json_outputs_run_report() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["codex", "review risky diff", "--json"])
        .current_dir(tmp.path())
        .env("PATH", "")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"agent\": \"codex\""))
        .stdout(predicate::str::contains("\"status\": \"fallback\""))
        .stdout(predicate::str::contains("\"fallback_used\": true"));
}

#[test]
fn stats_after_adapter_run_includes_agent_latency_and_fallback() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["claude", "explain flaky test"])
        .current_dir(tmp.path())
        .env("PATH", "")
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("stats")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("original_tokens"))
        .stdout(predicate::str::contains("packed_tokens"))
        .stdout(predicate::str::contains("latency_ms"))
        .stdout(predicate::str::contains("agent"))
        .stdout(predicate::str::contains("fallback_used"));
}

#[test]
fn adapter_json_contract_contains_required_fields() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = Command::cargo_bin("ctx")
        .expect("bin")
        .args(["opencode", "run", "explain this diff", "--json"])
        .current_dir(tmp.path())
        .env("PATH", "")
        .output()
        .expect("run ctx");

    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["agent"], "opencode");
    assert!(value["command"].as_str().unwrap().contains("opencode run"));
    assert_eq!(value["status"], "fallback");
    assert_eq!(value["fallback_used"], true);
    assert!(value["original_tokens"].as_u64().is_some());
    assert!(value["packed_tokens"].as_u64().is_some());
    assert!(value["reduction_pct"].is_number());
}

#[cfg(unix)]
#[test]
fn claude_wrapper_invokes_fake_claude_binary_and_records_success() {
    let tmp = tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("mkdir bin");
    write_shell_script(&bin_dir.join("claude"), "#!/bin/sh\nexit 0\n");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["claude", "explain flaky test"])
        .current_dir(tmp.path())
        .env("PATH", path_with_bin(&bin_dir))
        .assert()
        .success();

    let stats = fs::read_to_string(tmp.path().join(".ctx/stats/latest.json")).expect("stats");
    assert!(stats.contains("claude"));
    assert!(stats.contains("succeeded"));

    let audit = fs::read_to_string(tmp.path().join(".ctx/audit.log")).expect("audit");
    assert!(audit.contains("adapter_invocation"));
    assert!(audit.contains("claude"));
}

#[cfg(unix)]
#[test]
fn codex_wrapper_invokes_fake_codex_binary_and_records_success() {
    let tmp = tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("mkdir bin");
    write_shell_script(&bin_dir.join("codex"), "#!/bin/sh\nexit 0\n");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["codex", "review diff"])
        .current_dir(tmp.path())
        .env("PATH", path_with_bin(&bin_dir))
        .assert()
        .success();

    let stats = fs::read_to_string(tmp.path().join(".ctx/stats/latest.json")).expect("stats");
    assert!(stats.contains("codex"));
    assert!(stats.contains("succeeded"));
}

#[cfg(unix)]
#[test]
fn opencode_wrapper_invokes_fake_opencode_binary_and_records_success() {
    let tmp = tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("mkdir bin");
    write_shell_script(&bin_dir.join("opencode"), "#!/bin/sh\nexit 0\n");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["opencode", "run", "explain diff"])
        .current_dir(tmp.path())
        .env("PATH", path_with_bin(&bin_dir))
        .assert()
        .success();

    let stats = fs::read_to_string(tmp.path().join(".ctx/stats/latest.json")).expect("stats");
    assert!(stats.contains("opencode"));
    assert!(stats.contains("succeeded"));
}

#[test]
fn mcp_serve_once_handles_rpc_tools_list() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let port = free_port();
    let bin = assert_cmd::cargo::cargo_bin("ctx");

    let mut child = std::process::Command::new(bin)
        .args(["mcp", "serve", "--once", "--port", &port.to_string()])
        .current_dir(tmp.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp server");

    let body = rpc_tools_list(port);
    assert!(body.contains("get_relevant_context"));
    assert!(body.contains("project_map"));

    for _ in 0..80 {
        if let Some(status) = child.try_wait().expect("wait") {
            assert!(status.success());
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }

    let _ = child.kill();
    panic!("mcp server did not exit in time");
}

#[test]
fn graph_rebuild_alias_works() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(tmp.path().join("src/auth.rs"), "fn x() {}").expect("write");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["graph", "rebuild"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("graph_build_indexed_files:"));
}

#[test]
fn retrieve_returns_ranked_hits() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(
        tmp.path().join("src/auth.rs"),
        "fn validate_refresh_token(token: &str) -> bool { !token.is_empty() }",
    )
    .expect("write");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("index")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["retrieve", "refresh token auth", "--limit", "3"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("validate_refresh_token"));
}

#[test]
fn ask_command_builds_compact_context_without_invoking_agent() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["ask", "where is retry logic"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("query: where is retry logic"));
}

#[test]
fn hook_command_outputs_pre_prompt_payload_for_agent_hooks() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["hook", "fix flaky test"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Task: fix flaky test"))
        .stdout(predicate::str::contains("Compact Context:"))
        .stdout(predicate::str::contains("Instruction:"));
}

#[test]
fn wrap_command_routes_to_selected_adapter_with_fallback() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["wrap", "claude", "--prompt", "explain auth failure"])
        .current_dir(tmp.path())
        .env("PATH", "")
        .assert()
        .success()
        .stdout(predicate::str::contains("adapter=claude"))
        .stdout(predicate::str::contains("command=claude -p"))
        .stdout(predicate::str::contains("[CTX COMPACT CONTEXT]"));
}

#[test]
fn mcp_config_claude_outputs_stdio_configuration() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["mcp", "config", "claude"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"mcpServers\""))
        .stdout(predicate::str::contains("\"ctx\""))
        .stdout(predicate::str::contains("\"mcp\""))
        .stdout(predicate::str::contains("\"stdio\""))
        .stdout(predicate::str::contains(
            tmp.path().to_string_lossy().as_ref(),
        ));
}

#[test]
fn mcp_stdio_handles_initialize_message() {
    let tmp = tempdir().expect("tempdir");
    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["mcp", "stdio"])
        .current_dir(tmp.path())
        .write_stdin("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"serverInfo\""))
        .stdout(predicate::str::contains("\"ctx-mcp\""));
}

#[test]
fn help_command_prints_command_guide_with_examples() {
    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains("CTX Command Guide"))
        .stdout(predicate::str::contains("ctx init"))
        .stdout(predicate::str::contains(
            "Example: ctx pack \"fix failing pytest in auth\" --json --attach /tmp/fail.txt",
        ))
        .stdout(predicate::str::contains("ctx mcp serve --port 8765"))
        .stdout(predicate::str::contains("ctx ask"))
        .stdout(predicate::str::contains("ctx wrap"))
        .stdout(predicate::str::contains("ctx hook"))
        .stdout(predicate::str::contains("ctx mcp stdio"))
        .stdout(predicate::str::contains("ctx memory set"))
        .stdout(predicate::str::contains("ctx benchmark memory-ab"))
        .stdout(predicate::str::contains("ctx doctor"));
}

#[test]
fn release_assets_are_present_and_document_install_paths() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let build_script = fs::read_to_string(root.join("scripts/release/build.sh"))
        .expect("release build script should exist");
    let smoke_script = fs::read_to_string(root.join("scripts/release/install-smoke.sh"))
        .expect("install smoke script should exist");
    let formula =
        fs::read_to_string(root.join("Formula/ctx.rb")).expect("homebrew formula should exist");
    let install_docs =
        fs::read_to_string(root.join("docs/install.md")).expect("install docs should exist");

    assert!(build_script.contains("build --release"));
    assert!(build_script.contains("SHA256SUMS"));
    assert!(build_script.contains("tar"));
    assert!(smoke_script.contains("doctor"));
    assert!(smoke_script.contains("mcp stdio"));
    assert!(formula.contains("class Ctx < Formula"));
    assert!(formula.contains("\"cargo\", \"install\""));
    assert!(formula.contains("\"doctor\""));
    assert!(install_docs.contains("Homebrew"));
    assert!(install_docs.contains("GitHub Releases"));
    assert!(install_docs.contains("cargo install"));
    assert!(install_docs.contains("ctx doctor"));
}

#[test]
#[cfg(unix)]
fn codex_wrapper_invokes_real_codex_binary_when_available() {
    let tmp = tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("mkdir");
    write_shell_script(
        &bin_dir.join("codex"),
        "#!/bin/sh\nprintf 'FAKE-CODEX\\n'\nprintf 'subcommand:%s\\n' \"$1\"\nprintf '%s\\n' \"$2\"\n",
    );

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["codex", "review risky diff"])
        .current_dir(tmp.path())
        .env("PATH", path_with_bin(&bin_dir))
        .assert()
        .success()
        .stdout(predicate::str::contains("FAKE-CODEX"))
        .stdout(predicate::str::contains("subcommand:exec"))
        .stdout(predicate::str::contains("[CTX COMPACT CONTEXT]"));
}

#[test]
#[cfg(unix)]
fn opencode_wrapper_invokes_real_opencode_binary_when_available() {
    let tmp = tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("mkdir");
    write_shell_script(
        &bin_dir.join("opencode"),
        "#!/bin/sh\nprintf 'FAKE-OPENCODE\\n'\nprintf 'subcommand:%s\\n' \"$1\"\nprintf '%s\\n' \"$2\"\n",
    );

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["opencode", "run", "explain this diff"])
        .current_dir(tmp.path())
        .env("PATH", path_with_bin(&bin_dir))
        .assert()
        .success()
        .stdout(predicate::str::contains("FAKE-OPENCODE"))
        .stdout(predicate::str::contains("subcommand:run"))
        .stdout(predicate::str::contains("[CTX COMPACT CONTEXT]"));
}

#[test]
#[cfg(unix)]
fn codex_wrapper_falls_back_to_printed_context_if_binary_missing() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["codex", "review risky diff"])
        .current_dir(tmp.path())
        .env(
            "PATH",
            tmp.path().join("missing-bin").to_string_lossy().to_string(),
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("adapter=codex"))
        .stdout(predicate::str::contains("command=codex"))
        .stdout(predicate::str::contains("[CTX COMPACT CONTEXT]"))
        .stderr(predicate::str::contains(
            "ctx warning: 'codex' not found in PATH",
        ));
}

#[test]
fn memory_commands_support_set_get_list_delete() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args([
            "memory",
            "set",
            "testing.always_run",
            "Run targeted tests before completion.",
            "--scope",
            "project",
            "--source",
            "manual",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "memory directive upserted: key=testing.always_run",
        ));

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["--json", "memory", "get", "testing.always_run"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"key\": \"testing.always_run\""));

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["memory", "list", "--scope", "project", "--limit", "10"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "testing.always_run [project:manual]",
        ));

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["memory", "delete", "testing.always_run"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "memory directive deleted: testing.always_run",
        ));
}

#[test]
fn benchmark_memory_ab_outputs_comparison_metrics() {
    let tmp = tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("AGENTS.md"),
        "# Rules\n- Run tests before merge.\n- Fix root cause, never bypass failures.\n",
    )
    .expect("write markdown");
    fs::write(
        tmp.path().join("checklist.md"),
        "- Run tests before merge.\n- Fix root cause, never bypass failures.\n",
    )
    .expect("write checklist");
    fs::write(
        tmp.path().join("markdown_answer.txt"),
        "I will run tests before merge.",
    )
    .expect("write markdown answer");
    fs::write(
        tmp.path().join("graph_answer.txt"),
        "I will run tests before merge and fix root cause, never bypass failures.",
    )
    .expect("write graph answer");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args([
            "memory",
            "set",
            "tests.required",
            "Run tests before merge.",
            "--scope",
            "project",
            "--source",
            "manual",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args([
            "--json",
            "benchmark",
            "memory-ab",
            "run tests and fix root cause",
            "--markdown",
            "AGENTS.md",
            "--limit",
            "10",
            "--checklist",
            "checklist.md",
            "--markdown-answer",
            "markdown_answer.txt",
            "--graph-answer",
            "graph_answer.txt",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"markdown_tokens\""))
        .stdout(predicate::str::contains("\"graph_memory_tokens\""))
        .stdout(predicate::str::contains("\"token_reduction_pct\""))
        .stdout(predicate::str::contains("\"quality_winner\""));
}

#[test]
fn memory_import_and_export_commands_work_with_markdown_files() {
    let tmp = tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("AGENTS.md"),
        "# Rules\n- Run tests before merge.\n- Fix root cause.\n",
    )
    .expect("write markdown");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args([
            "memory",
            "import",
            "--from",
            "AGENTS.md",
            "--scope",
            "project",
            "--source",
            "markdown",
            "--prefix",
            "agents",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("imported"));

    Command::cargo_bin("ctx")
        .expect("bin")
        .args([
            "memory",
            "export",
            "--to",
            "AGENTS.generated.md",
            "--scope",
            "project",
            "--limit",
            "50",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("exported"));

    let exported = fs::read_to_string(tmp.path().join("AGENTS.generated.md")).expect("read export");
    assert!(exported.contains("Graph Memory Directives"));
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    port
}

fn write_shell_script(path: &std::path::Path, content: &str) {
    fs::write(path, content).expect("write script");
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("chmod");
    }
}

fn path_with_bin(bin_dir: &std::path::Path) -> String {
    let mut parts = vec![bin_dir.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        parts.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(parts)
        .expect("join paths")
        .to_string_lossy()
        .to_string()
}

fn rpc_tools_list(port: u16) -> String {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );

    for _ in 0..80 {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
            if stream.write_all(request.as_bytes()).is_ok() && stream.flush().is_ok() {
                let mut response = String::new();
                if stream.read_to_string(&mut response).is_ok() {
                    if let Some(payload) = response.split("\r\n\r\n").nth(1) {
                        return payload.to_string();
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(25));
    }

    panic!("failed to complete mcp rpc call on port {port}");
}
