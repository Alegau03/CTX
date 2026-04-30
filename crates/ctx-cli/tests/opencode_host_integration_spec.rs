use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn opencode_host_first_docs_capture_the_product_pivot() {
    let root = repo_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("readme");
    let guidelines = fs::read_to_string(root.join("docs/guidelines.md")).expect("guidelines");
    let integration =
        fs::read_to_string(root.join("docs/opencode-integration.md")).expect("integration doc");
    let guide = fs::read_to_string(root.join("guide.md")).expect("guide");

    assert!(readme.contains("OpenCode-first"));
    assert!(readme.contains("guide.md"));
    assert!(readme.contains("Graph Memory"));
    assert!(!readme.contains("docs/superpowers/plans/"));
    assert!(guidelines.contains("OpenCode-first is the highest-priority integration target."));
    assert!(guidelines.contains("OpenCode-native commands are the product surface"));
    assert!(guidelines.contains("wrapper-first UX as legacy"));
    assert!(integration.contains("Make CTX live inside OpenCode"));
    assert!(integration.contains("should open `opencode`"));
    assert!(guide.contains("Recommended Order"));
    assert!(guide.contains("Graph Memory Workflow"));
}

#[test]
fn opencode_project_bootstrap_generates_local_mcp_and_command_assets() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["opencode", "install"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("installed OpenCode integration"));

    assert!(tmp.path().join("opencode.json").exists());
    assert!(tmp.path().join(".opencode/commands").is_dir());
}

#[test]
fn opencode_native_commands_cover_ctx_surface_area_without_wrappers() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["opencode", "install"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let commands_dir = tmp.path().join(".opencode/commands");

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
        assert!(commands_dir.join(command).exists(), "missing {command}");
    }
}

#[test]
fn opencode_host_selected_model_remains_owner_while_ctx_provides_tools() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["opencode", "install"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let config = fs::read_to_string(tmp.path().join("opencode.json")).expect("opencode config");
    assert!(config.contains("\"mcp\""));
    assert!(!config.contains("\"model\": \"ctx/"));
    assert!(config.contains("\"instructions\""));
    assert!(config.contains(".opencode/instructions/ctx-host-first.md"));

    let command = fs::read_to_string(tmp.path().join(".opencode/commands/ctx-pack.md"))
        .expect("ctx-pack command");
    assert!(command.contains("description:"));
    assert!(command.contains("Context |"));
    assert!(!command.contains("\nagent:"));
    assert!(!command.contains("\nmodel:"));
    assert!(command.contains("pack \"$ARGUMENTS\" --json"));
    assert!(command.contains("Print `compact_context` first"));
    assert!(command.contains("at most one short sentence"));

    let retrieve_command =
        fs::read_to_string(tmp.path().join(".opencode/commands/ctx-retrieve.md"))
            .expect("ctx-retrieve command");
    assert!(retrieve_command.contains("retrieve \"$ARGUMENTS\" --limit 8 --json"));
    assert!(retrieve_command.contains("Start with the useful result immediately"));
    assert!(retrieve_command.contains("Keep any follow-up summary to one short sentence"));

    let hook_command = fs::read_to_string(tmp.path().join(".opencode/commands/ctx-hook.md"))
        .expect("ctx-hook command");
    assert!(hook_command.contains("hook \"$ARGUMENTS\" --json"));
    assert!(hook_command.contains("Print `hook_prompt` first"));
    assert!(hook_command.contains("single compact metadata line"));

    let memory_search_command =
        fs::read_to_string(tmp.path().join(".opencode/commands/ctx-memory-search.md"))
            .expect("ctx-memory-search command");
    assert!(memory_search_command.contains("memory search \"$1\" --json"));
    assert!(memory_search_command.contains("Show only the matching directives"));

    let stats_command = fs::read_to_string(tmp.path().join(".opencode/commands/ctx-stats.md"))
        .expect("ctx-stats command");
    assert!(stats_command.contains("Show the stats payload first"));
    assert!(stats_command.contains("one short sentence"));

    let menu =
        fs::read_to_string(tmp.path().join(".opencode/commands/ctx.md")).expect("ctx menu command");
    assert!(menu.contains("deterministic CTX menu command"));
    assert!(menu.contains("do not inspect files manually"));
    assert!(menu.contains("menu"));
    assert!(menu.contains("--repo-root"));
    assert!(menu.contains("!`"));

    let instructions =
        fs::read_to_string(tmp.path().join(".opencode/instructions/ctx-host-first.md"))
            .expect("ctx host-first instructions");
    assert!(instructions.contains("Primary Workflow"));
    assert!(instructions.contains("Automatic CTX Usage"));
    assert!(instructions.contains("/ctx-memory-bootstrap"));
    assert!(instructions.contains("/ctx-memory-search"));

    let index_command = fs::read_to_string(tmp.path().join(".opencode/commands/ctx-index.md"))
        .expect("ctx-index command");
    assert!(index_command.contains("!`"));
    assert!(index_command.contains("--repo-root"));
    assert!(index_command.contains("do not glob files"));
    assert!(index_command.contains("indexed_files:"));
}
