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
    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["explain", "fix failing pytest"])
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
        .stdout(predicate::str::contains("ctx mcp serve --port 8765"));
}

#[test]
#[cfg(unix)]
fn codex_wrapper_invokes_real_codex_binary_when_available() {
    let tmp = tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("mkdir");
    write_shell_script(
        &bin_dir.join("codex"),
        "#!/bin/sh\nprintf 'FAKE-CODEX\\n'\nprintf '%s\\n' \"$1\"\n",
    );

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["codex", "review risky diff"])
        .current_dir(tmp.path())
        .env("PATH", path_with_bin(&bin_dir))
        .assert()
        .success()
        .stdout(predicate::str::contains("FAKE-CODEX"))
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
