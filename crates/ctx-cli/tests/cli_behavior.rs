use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

fn copy_dir_recursive(src: &Path, dst: &Path) {
    if !src.exists() {
        return;
    }
    fs::create_dir_all(dst).expect("create destination directory");
    for entry in fs::read_dir(src).expect("read source directory") {
        let entry = entry.expect("read directory entry");
        let name = entry.file_name();
        if matches!(
            name.to_str(),
            Some(".ctx") | Some(".opencode") | Some("opencode.json")
        ) {
            continue;
        }
        let file_type = entry.file_type().expect("read entry type");
        let target = dst.join(name);
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target);
        } else if file_type.is_file() {
            fs::copy(entry.path(), target).expect("copy file");
        }
    }
}

fn cloned_demo_fixture() -> tempfile::TempDir {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("demo/fixtures/opencode-auth-lab");
    let temp = tempdir().expect("create tempdir");
    copy_dir_recursive(&source, temp.path());
    temp
}

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
fn mcp_config_opencode_outputs_local_mcp_configuration() {
    let tmp = tempdir().expect("tempdir");

    let output = Command::cargo_bin("ctx")
        .expect("bin")
        .args(["mcp", "config", "opencode"])
        .current_dir(tmp.path())
        .output()
        .expect("run ctx");

    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(
        value["$schema"],
        serde_json::Value::String("https://opencode.ai/config.json".to_string())
    );
    assert_eq!(value["mcp"]["ctx"]["type"], "local");
    assert_eq!(value["mcp"]["ctx"]["enabled"], true);
    let command = value["mcp"]["ctx"]["command"]
        .as_array()
        .expect("command array");
    assert_eq!(command.first().and_then(|item| item.as_str()), Some("ctx"));
    let repo_root_index = command
        .iter()
        .position(|item| item == "--repo-root")
        .expect("repo-root flag");
    assert!(command.iter().any(|item| item == "mcp"));
    assert!(command.iter().any(|item| item == "stdio"));
    let rendered_repo_root = command
        .get(repo_root_index + 1)
        .and_then(|item| item.as_str())
        .expect("repo-root value");
    let rendered_repo_root = std::path::PathBuf::from(rendered_repo_root);
    assert!(rendered_repo_root.is_absolute());
    assert_eq!(
        std::fs::canonicalize(rendered_repo_root).expect("canonical rendered repo root"),
        std::fs::canonicalize(tmp.path()).expect("canonical temp repo root")
    );
}

#[test]
fn mcp_config_codex_outputs_toml_configuration() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["mcp", "config", "codex"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("[mcp_servers.ctx]"))
        .stdout(predicate::str::contains("command = \"ctx\""))
        .stdout(predicate::str::contains("args = ["))
        .stdout(predicate::str::contains("mcp"))
        .stdout(predicate::str::contains("stdio"))
        .stdout(predicate::str::contains(
            tmp.path().to_string_lossy().as_ref(),
        ));
}

#[test]
fn opencode_install_creates_project_config_and_command_files() {
    let tmp = tempdir().expect("tempdir");

    let output = Command::cargo_bin("ctx")
        .expect("bin")
        .args(["--json", "opencode", "install"])
        .current_dir(tmp.path())
        .output()
        .expect("run ctx");

    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(report["commands_written"], 32);
    assert_eq!(
        report["instruction_files"]
            .as_array()
            .map(|items| items.len()),
        Some(1)
    );

    let config =
        std::fs::read_to_string(tmp.path().join("opencode.json")).expect("opencode config");
    let value: serde_json::Value = serde_json::from_str(&config).expect("config json");
    assert_eq!(value["mcp"]["ctx"]["type"], "local");
    assert_eq!(value["mcp"]["ctx"]["enabled"], true);
    let instructions = value["instructions"]
        .as_array()
        .expect("instructions array");
    assert!(instructions.iter().any(|item| item == "docs/guidelines.md"));
    assert!(instructions.iter().any(|item| item == "docs/security.md"));
    assert!(
        instructions
            .iter()
            .any(|item| item == ".opencode/instructions/ctx-host-first.md")
    );

    for command in [
        "ctx.md",
        "ctx-help.md",
        "ctx-init.md",
        "ctx-index.md",
        "ctx-reindex.md",
        "ctx-graph-build.md",
        "ctx-graph-rebuild.md",
        "ctx-doctor.md",
        "ctx-pack.md",
        "ctx-ask.md",
        "ctx-hook.md",
        "ctx-explain.md",
        "ctx-retrieve.md",
        "ctx-graph-query.md",
        "ctx-prune-logs.md",
        "ctx-prune-diff.md",
        "ctx-opencode-install.md",
        "ctx-mcp-serve.md",
        "ctx-mcp-stdio.md",
        "ctx-mcp-config-claude.md",
        "ctx-mcp-config-opencode.md",
        "ctx-memory-set.md",
        "ctx-memory-get.md",
        "ctx-memory-list.md",
        "ctx-memory-search.md",
        "ctx-memory-delete.md",
        "ctx-memory-import.md",
        "ctx-memory-bootstrap.md",
        "ctx-memory-export.md",
        "ctx-benchmark-memory-ab.md",
        "ctx-benchmark-memory-suite.md",
        "ctx-stats.md",
    ] {
        assert!(
            tmp.path().join(".opencode/commands").join(command).exists(),
            "missing {command}"
        );
    }

    let host_first = tmp.path().join(".opencode/instructions/ctx-host-first.md");
    assert!(host_first.exists(), "missing ctx-host-first.md");
    let host_first_text =
        std::fs::read_to_string(host_first).expect("read host-first instructions");
    assert!(host_first_text.contains("Prefer CTX slash commands and CTX MCP tools"));
    assert!(host_first_text.contains("Do not revive wrapper-style workflows"));

    let menu_command = std::fs::read_to_string(tmp.path().join(".opencode/commands/ctx.md"))
        .expect("read ctx menu command");
    assert!(menu_command.contains("description: Menu |"));
    assert!(menu_command.contains("CTX Command Center"));
    assert!(menu_command.contains("Recommended Start"));
    assert!(menu_command.contains("/ctx-pack <task>"));
}

#[test]
fn codex_install_creates_project_config_and_skill_assets() {
    let tmp = tempdir().expect("tempdir");

    let output = Command::cargo_bin("ctx")
        .expect("bin")
        .args(["--json", "codex", "install"])
        .current_dir(tmp.path())
        .output()
        .expect("run ctx");

    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(report["skills_written"], 32);

    let config =
        std::fs::read_to_string(tmp.path().join(".codex/config.toml")).expect("codex config");
    assert!(config.contains("[mcp_servers.ctx]"));
    assert!(config.contains("command = \"ctx\""));
    assert!(config.contains("mcp"));
    assert!(config.contains("stdio"));

    let skill_path = tmp.path().join(".agents/skills/ctx-pack/SKILL.md");
    assert!(skill_path.exists(), "missing ctx-pack Codex skill");
    let skill = std::fs::read_to_string(skill_path).expect("read codex skill");
    assert!(skill.contains("name: ctx-pack"));
    assert!(skill.contains("Run `ctx pack"));
}

#[test]
fn claude_install_creates_project_mcp_and_skill_assets() {
    let tmp = tempdir().expect("tempdir");

    let output = Command::cargo_bin("ctx")
        .expect("bin")
        .args(["--json", "claude", "install"])
        .current_dir(tmp.path())
        .output()
        .expect("run ctx");

    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(report["skills_written"], 32);

    let config = std::fs::read_to_string(tmp.path().join(".mcp.json")).expect("claude mcp config");
    let value: serde_json::Value = serde_json::from_str(&config).expect("claude mcp json");
    assert!(value["mcpServers"]["ctx"].is_object());
    assert_eq!(value["mcpServers"]["ctx"]["command"], "ctx");
    assert!(
        value["mcpServers"]["ctx"]["args"]
            .as_array()
            .expect("claude args")
            .iter()
            .any(|item| item == "stdio")
    );

    let skill_path = tmp.path().join(".claude/skills/ctx-pack/SKILL.md");
    assert!(skill_path.exists(), "missing ctx-pack Claude skill");
    let skill = std::fs::read_to_string(skill_path).expect("read claude skill");
    assert!(skill.contains("name: ctx-pack"));
    assert!(skill.contains("/ctx-pack"));
}

#[test]
fn opencode_install_merges_existing_mcp_config() {
    let tmp = tempdir().expect("tempdir");
    std::fs::write(
        tmp.path().join("opencode.json"),
        r#"{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "other": {
      "type": "local",
      "enabled": true,
      "command": ["echo", "hi"]
    }
  }
}"#,
    )
    .expect("write existing config");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["opencode", "install"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(tmp.path().join("opencode.json")).expect("config");
    let value: serde_json::Value = serde_json::from_str(&config).expect("json");
    assert!(value["mcp"]["other"].is_object());
    assert!(value["mcp"]["ctx"].is_object());
    assert_eq!(value["mcp"]["ctx"]["type"], "local");
    assert!(value["instructions"].is_array());
    assert!(
        value["instructions"]
            .as_array()
            .expect("instructions")
            .iter()
            .any(|item| item == ".opencode/instructions/ctx-host-first.md")
    );
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
        .stdout(predicate::str::contains("Primary OpenCode path:"))
        .stdout(predicate::str::contains(
            "legacy wrapper commands have been removed",
        ))
        .stdout(predicate::str::contains("ctx init"))
        .stdout(predicate::str::contains(
            "Example: ctx pack \"fix failing pytest in auth\" --json --attach /tmp/fail.txt",
        ))
        .stdout(predicate::str::contains("ctx mcp serve --port 8765"))
        .stdout(predicate::str::contains("ctx ask"))
        .stdout(predicate::str::contains("ctx wrap").not())
        .stdout(predicate::str::contains("ctx hook"))
        .stdout(predicate::str::contains("ctx mcp stdio"))
        .stdout(predicate::str::contains("ctx opencode install"))
        .stdout(predicate::str::contains("ctx codex install"))
        .stdout(predicate::str::contains("ctx claude install"))
        .stdout(predicate::str::contains("ctx mcp config codex"))
        .stdout(predicate::str::contains("ctx codex \"").not())
        .stdout(predicate::str::contains("ctx claude \"").not())
        .stdout(predicate::str::contains("ctx opencode run").not())
        .stdout(predicate::str::contains("ctx memory set"))
        .stdout(predicate::str::contains("ctx benchmark memory-ab"))
        .stdout(predicate::str::contains("ctx doctor"));
}

#[test]
fn legacy_wrapper_commands_are_removed_from_public_cli() {
    let tmp = tempdir().expect("tempdir");

    for args in [
        vec!["wrap", "claude", "--prompt", "explain auth failure"],
        vec!["codex", "review risky diff"],
        vec!["claude", "explain flaky test"],
        vec!["opencode", "run", "explain diff"],
    ] {
        Command::cargo_bin("ctx")
            .expect("bin")
            .args(&args)
            .current_dir(tmp.path())
            .assert()
            .failure()
            .stderr(predicate::str::contains("unrecognized"));
    }
}

#[test]
fn release_assets_are_present_and_document_install_paths() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let build_script = fs::read_to_string(root.join("scripts/release/build.sh"))
        .expect("release build script should exist");
    let smoke_script = fs::read_to_string(root.join("scripts/release/install-smoke.sh"))
        .expect("install smoke script should exist");
    let opencode_smoke = fs::read_to_string(root.join("scripts/release/opencode-smoke.sh"))
        .expect("opencode smoke script should exist");
    let verify_script = fs::read_to_string(root.join("scripts/release/verify-artifact.sh"))
        .expect("artifact verify script should exist");
    let formula =
        fs::read_to_string(root.join("Formula/ctx.rb")).expect("homebrew formula should exist");
    let install_docs =
        fs::read_to_string(root.join("docs/install.md")).expect("install docs should exist");
    let readme = fs::read_to_string(root.join("README.md")).expect("readme should exist");
    let guide = fs::read_to_string(root.join("guide.md")).expect("guide should exist");

    assert!(build_script.contains("build --release"));
    assert!(build_script.contains("SHA256SUMS"));
    assert!(build_script.contains("tar"));
    assert!(build_script.contains("release-manifest.json"));
    assert!(build_script.contains("opencode-smoke.sh"));
    assert!(build_script.contains("verify-artifact.sh"));
    assert!(smoke_script.contains("doctor"));
    assert!(smoke_script.contains("mcp stdio"));
    assert!(opencode_smoke.contains("opencode install"));
    assert!(opencode_smoke.contains(".opencode/commands/ctx-pack.md"));
    assert!(opencode_smoke.contains(".opencode/commands/ctx-memory-bootstrap.md"));
    assert!(opencode_smoke.contains(".opencode/commands/ctx-memory-search.md"));
    assert!(opencode_smoke.contains(".opencode/instructions/ctx-host-first.md"));
    assert!(opencode_smoke.contains("opencode.json"));
    assert!(verify_script.contains("SHA256SUMS"));
    assert!(verify_script.contains("tar -xzf"));
    assert!(verify_script.contains("install-smoke.sh"));
    assert!(verify_script.contains("opencode-smoke.sh"));
    assert!(verify_script.contains("opencode-auth-lab-benchmark.sh"));
    assert!(formula.contains("class Ctx < Formula"));
    assert!(formula.contains("\"cargo\", \"install\""));
    assert!(formula.contains("\"doctor\""));
    assert!(install_docs.contains("Homebrew"));
    assert!(install_docs.contains("GitHub Releases"));
    assert!(install_docs.contains("cargo install"));
    assert!(install_docs.contains("ctx doctor"));
    assert!(install_docs.contains("ctx codex install"));
    assert!(install_docs.contains("ctx claude install"));
    assert!(install_docs.contains("scripts/release/opencode-smoke.sh"));
    assert!(install_docs.contains("scripts/release/verify-artifact.sh"));
    assert!(install_docs.contains("release-manifest.json"));
    assert!(readme.contains("docs/codex-integration.md"));
    assert!(readme.contains("docs/claude-integration.md"));
    assert!(readme.contains("guide.md"));
    assert!(guide.contains("OpenCode-First Workflow"));
    assert!(guide.contains("Command Reference"));
}

#[test]
fn readme_is_product_focused_and_guide_holds_operational_details() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let readme = fs::read_to_string(root.join("README.md")).expect("readme should exist");
    let guide = fs::read_to_string(root.join("guide.md")).expect("guide should exist");

    assert!(readme.contains("What CTX Is"));
    assert!(readme.contains("OpenCode-First Usage"));
    assert!(!readme.contains("OpenCode Commands And Integration Tests"));
    assert!(!readme.contains("CLI Commands And Functional Tests"));
    assert!(guide.contains("OpenCode-First Workflow"));
    assert!(guide.contains("Command Reference"));
}

#[test]
fn opencode_release_smoke_script_bootstraps_host_first_assets() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("scripts/release/opencode-smoke.sh");
    let ctx_bin = assert_cmd::cargo::cargo_bin("ctx");

    std::process::Command::new(script)
        .arg(ctx_bin)
        .current_dir(&root)
        .output()
        .map(|output| {
            assert!(
                output.status.success(),
                "opencode smoke failed:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("CTX OpenCode smoke passed:"));
        })
        .expect("run opencode smoke script");
}

#[test]
fn demo_fixture_cli_smoke_runs_successfully() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("scripts/demo/opencode-auth-lab-smoke.sh");
    let ctx_bin = assert_cmd::cargo::cargo_bin("ctx");
    let fixture = cloned_demo_fixture();

    let output = std::process::Command::new(script)
        .arg(ctx_bin)
        .env("CTX_DEMO_FIXTURE", fixture.path())
        .current_dir(&root)
        .output()
        .expect("run demo smoke");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("CTX demo smoke passed:"));
}

#[test]
fn demo_fixture_benchmark_script_writes_reports() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("scripts/demo/opencode-auth-lab-benchmark.sh");
    let ctx_bin = assert_cmd::cargo::cargo_bin("ctx");
    let fixture = cloned_demo_fixture();

    let output = std::process::Command::new(script)
        .arg(ctx_bin)
        .env("CTX_DEMO_FIXTURE", fixture.path())
        .current_dir(&root)
        .output()
        .expect("run benchmark smoke");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fixture.path().join("benchmarks/report.md").exists());
    assert!(fixture.path().join("benchmarks/report.json").exists());
}

#[test]
fn release_verify_script_validates_packaged_artifact() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let verify_script = root.join("scripts/release/verify-artifact.sh");
    let ctx_bin = assert_cmd::cargo::cargo_bin("ctx");
    let temp = tempfile::tempdir().expect("create tempdir");
    let package_dir = temp.path().join("ctx-0.1.0-test-target");
    let tarball = temp.path().join("ctx-0.1.0-test-target.tar.gz");
    let checksums = temp.path().join("SHA256SUMS");

    fs::create_dir_all(&package_dir).expect("create package dir");
    fs::copy(&ctx_bin, package_dir.join("ctx")).expect("copy ctx binary into package dir");
    fs::write(package_dir.join("README.md"), "ctx test package\n").expect("write readme");
    fs::write(package_dir.join("INSTALL.md"), "ctx install docs\n").expect("write install");

    let status = std::process::Command::new("tar")
        .arg("-czf")
        .arg(&tarball)
        .arg("ctx-0.1.0-test-target")
        .current_dir(temp.path())
        .status()
        .expect("create tarball");
    assert!(status.success(), "tar should succeed");

    let digest_output = std::process::Command::new("shasum")
        .arg("-a")
        .arg("256")
        .arg(&tarball)
        .output()
        .expect("hash tarball");
    assert!(
        digest_output.status.success(),
        "shasum should succeed: {}",
        String::from_utf8_lossy(&digest_output.stderr)
    );
    fs::write(&checksums, digest_output.stdout).expect("write checksums");

    let output = std::process::Command::new(verify_script)
        .arg(&tarball)
        .arg(&checksums)
        .current_dir(&root)
        .output()
        .expect("run artifact verify script");

    assert!(
        output.status.success(),
        "verify artifact script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("CTX release artifact verification passed:"),
        "expected success banner from verify-artifact.sh"
    );
}

#[test]
fn release_docs_cover_qa_and_community_assets() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let readme = fs::read_to_string(root.join("README.md")).expect("readme should exist");
    let install_docs =
        fs::read_to_string(root.join("docs/install.md")).expect("install docs should exist");
    let release_playbook = fs::read_to_string(root.join("docs/release-playbook.md"))
        .expect("release playbook should exist");
    let final_qa =
        fs::read_to_string(root.join("docs/final-qa.md")).expect("final QA doc should exist");
    let release_template = fs::read_to_string(root.join(".github/RELEASE_TEMPLATE.md"))
        .expect("release template should exist");
    let qa_script = fs::read_to_string(root.join("scripts/release/final-qa.sh"))
        .expect("final QA script should exist");

    assert!(readme.contains("docs/release-playbook.md"));
    assert!(readme.contains("docs/final-qa.md"));
    assert!(install_docs.contains("scripts/release/final-qa.sh"));
    assert!(release_playbook.contains("GitHub Release Title"));
    assert!(release_playbook.contains("OpenCode Demo"));
    assert!(release_playbook.contains("Benchmark Evidence"));
    assert!(final_qa.contains("OpenCode-Native Final QA"));
    assert!(final_qa.contains("/ctx-memory-bootstrap"));
    assert!(final_qa.contains("/ctx-pack"));
    assert!(release_template.contains("## Highlights"));
    assert!(release_template.contains("## Install"));
    assert!(qa_script.contains("verify-artifact.sh"));
    assert!(qa_script.contains("opencode-auth-lab-smoke.sh"));
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
fn benchmark_memory_suite_writes_markdown_and_json_reports() {
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
    fs::write(
        tmp.path().join("memory-suite.toml"),
        r#"
title = "CTX Memory Benchmark"

[[cases]]
name = "auth_rules"
query = "run tests and fix root cause"
markdown = "AGENTS.md"
limit = 20
checklist = "checklist.md"
markdown_answer = "markdown_answer.txt"
graph_answer = "graph_answer.txt"
"#,
    )
    .expect("write spec");

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
            "memory",
            "set",
            "quality.root_cause",
            "Fix root cause, never bypass failures.",
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
            "memory-suite",
            "--spec",
            "memory-suite.toml",
            "--report-out",
            "benchmark-report.md",
            "--json-out",
            "benchmark-report.json",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"case_count\": 1"))
        .stdout(predicate::str::contains("\"report_markdown_path\":"));

    let report_md = fs::read_to_string(tmp.path().join("benchmark-report.md")).expect("report md");
    assert!(report_md.contains("# CTX Memory Benchmark"));
    assert!(report_md.contains("auth_rules"));

    let report_json =
        fs::read_to_string(tmp.path().join("benchmark-report.json")).expect("report json");
    assert!(report_json.contains("\"graph_quality_wins\":"));
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

#[test]
fn memory_bootstrap_and_search_commands_cover_agents_to_graph_flow() {
    let tmp = tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("AGENTS.md"),
        "# Rules\n- Run targeted tests before completion.\n- Fix auth root cause before merge.\n",
    )
    .expect("write agents");
    fs::create_dir_all(tmp.path().join(".github")).expect("create .github");
    fs::write(
        tmp.path().join(".github/copilot-instructions.md"),
        "# Copilot\n- Prefer auth fixtures when debugging refresh token failures.\n",
    )
    .expect("write copilot instructions");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["memory", "bootstrap"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("imported_files=2"))
        .stdout(predicate::str::contains("imported_directives="));

    Command::cargo_bin("ctx")
        .expect("bin")
        .args([
            "memory",
            "search",
            "auth root cause",
            "--scope",
            "project",
            "--limit",
            "10",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Fix auth root cause before merge.",
        ))
        .stdout(predicate::str::contains("[project:markdown]"));
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    port
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
