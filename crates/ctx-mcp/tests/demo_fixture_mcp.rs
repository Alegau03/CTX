use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

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

#[test]
fn demo_fixture_mcp_smoke_runs_successfully() {
    let root = repo_root();
    let script = root.join("scripts/demo/opencode-auth-lab-mcp-smoke.sh");
    let ctx_bin = assert_cmd::cargo::cargo_bin("ctx");
    let fixture_src = root.join("demo/fixtures/opencode-auth-lab");
    let fixture = tempdir().expect("create temp fixture");
    copy_dir_recursive(&fixture_src, fixture.path());

    let output = std::process::Command::new(script)
        .arg(ctx_bin)
        .env("CTX_DEMO_FIXTURE", fixture.path())
        .current_dir(&root)
        .output()
        .expect("run demo mcp smoke");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("CTX demo MCP smoke passed:"));
}
