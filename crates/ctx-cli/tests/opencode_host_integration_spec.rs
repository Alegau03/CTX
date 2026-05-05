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
    assert!(
        tmp.path()
            .join(".opencode/plugins/ctx-dashboard.tsx")
            .exists()
    );
    assert!(tmp.path().join(".opencode/package.json").exists());
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
        "ctx-read.md",
        "ctx-graph-query.md",
        "ctx-plan.md",
        "ctx-compare.md",
        "ctx-dashboard.md",
        "ctx-gain.md",
        "ctx-run.md",
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
        "ctx-toolbook-import.md",
        "ctx-toolbook-search.md",
        "ctx-toolbook-list.md",
        "ctx-toolbook-pack.md",
        "ctx-learn.md",
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
    assert!(retrieve_command.contains("## 🔎 CTX Retrieve"));
    assert!(retrieve_command.contains("**Top Hits**"));

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

    let compare_command = fs::read_to_string(tmp.path().join(".opencode/commands/ctx-compare.md"))
        .expect("ctx-compare command");
    assert!(compare_command.contains("OpenCode-only"));
    assert!(compare_command.contains("Before vs CTX"));
    assert!(compare_command.contains("original_estimated_tokens"));

    let gain_command = fs::read_to_string(tmp.path().join(".opencode/commands/ctx-gain.md"))
        .expect("ctx-gain command");
    assert!(gain_command.contains("OpenCode-only"));
    assert!(gain_command.contains("--json stats --history 20"));
    assert!(gain_command.contains("sampled_runs"));
    assert!(gain_command.contains("estimated_tokens_saved"));
    assert!(gain_command.contains("## 💸 CTX Gain"));
    assert!(gain_command.contains("**Savings**"));
    assert!(gain_command.contains("**Top Queries**"));

    let dashboard_command =
        fs::read_to_string(tmp.path().join(".opencode/commands/ctx-dashboard.md"))
            .expect("ctx-dashboard command");
    assert!(dashboard_command.contains("CTX Dashboard snapshot."));
    assert!(dashboard_command.contains("host-dashboard"));
    assert!(!dashboard_command.contains("--json host-dashboard"));
    assert!(dashboard_command.contains("present its output as-is"));
    assert!(dashboard_command.contains("do not rewrite the dashboard into another format"));

    let read_command = fs::read_to_string(tmp.path().join(".opencode/commands/ctx-read.md"))
        .expect("ctx-read command");
    assert!(read_command.contains("OpenCode-only"));
    assert!(read_command.contains("host-read"));
    assert!(read_command.contains("full`, `outline`, or `digest`"));
    assert!(read_command.contains("## 📖 CTX Read"));
    assert!(read_command.contains("**Content**"));
    assert!(read_command.contains("**Metadata**"));

    let run_command = fs::read_to_string(tmp.path().join(".opencode/commands/ctx-run.md"))
        .expect("ctx-run command");
    assert!(run_command.contains("OpenCode-only"));
    assert!(run_command.contains("--json host-run \"$ARGUMENTS\""));
    assert!(run_command.contains("CTX Run"));
    assert!(run_command.contains("raw_log_path"));
    assert!(run_command.contains("## 🧪 CTX Run"));
    assert!(run_command.contains("**Summary**"));
    assert!(run_command.contains("**Log**"));

    let plan_command = fs::read_to_string(tmp.path().join(".opencode/commands/ctx-plan.md"))
        .expect("ctx-plan command");
    assert!(plan_command.contains("OpenCode-only"));
    assert!(plan_command.contains("retrieve \"$ARGUMENTS\" --limit 8 --json"));
    assert!(plan_command.contains("memory search \"$ARGUMENTS\" --json"));
    assert!(plan_command.contains("graph query \"$ARGUMENTS\""));
    assert!(plan_command.contains("pack \"$ARGUMENTS\" --json"));
    assert!(plan_command.contains("Suggested First Action"));
    assert!(plan_command.contains("## 🧭 CTX Plan"));
    assert!(plan_command.contains("**Relevant Context**"));
    assert!(plan_command.contains("**Suggested Tests**"));

    let toolbook_pack =
        fs::read_to_string(tmp.path().join(".opencode/commands/ctx-toolbook-pack.md"))
            .expect("ctx-toolbook-pack command");
    assert!(toolbook_pack.contains("toolbook:$1"));
    assert!(toolbook_pack.contains("memory search"));
    assert!(toolbook_pack.contains("pack \"$2\" --json"));

    let learn_command = fs::read_to_string(tmp.path().join(".opencode/commands/ctx-learn.md"))
        .expect("ctx-learn command");
    assert!(learn_command.contains("memory set \"$1\" \"$2\""));
    assert!(learn_command.contains("--source learned"));

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
    assert!(instructions.contains("/ctx-toolbook-import"));
    assert!(instructions.contains("/ctx-plan"));
    assert!(instructions.contains("/ctx-compare"));
    assert!(instructions.contains("/ctx-dashboard"));
    assert!(instructions.contains("/ctx-gain"));
    assert!(instructions.contains("/ctx-read"));
    assert!(instructions.contains("/ctx-run"));
    assert!(instructions.contains("/ctx-learn"));

    let sidebar_plugin = fs::read_to_string(tmp.path().join(".opencode/plugins/ctx-dashboard.tsx"))
        .expect("ctx sidebar plugin");
    assert!(sidebar_plugin.contains("sidebar_content"));
    assert!(sidebar_plugin.contains("CTX Dashboard"));
    assert!(sidebar_plugin.contains("host-dashboard"));

    let opencode_package =
        fs::read_to_string(tmp.path().join(".opencode/package.json")).expect("opencode package");
    assert!(opencode_package.contains("@opencode-ai/plugin"));
    assert!(opencode_package.contains("@opentui/solid"));
    assert!(opencode_package.contains("solid-js"));
    assert!(opencode_package.contains("^1.14.19"));
    assert!(opencode_package.contains("^0.1.101"));

    let tui_config = fs::read_to_string(tmp.path().join(".opencode/tui.json")).expect("tui config");
    assert!(tui_config.contains("https://opencode.ai/tui.json"));
    assert!(tui_config.contains("./plugins/ctx-dashboard.tsx"));

    let index_command = fs::read_to_string(tmp.path().join(".opencode/commands/ctx-index.md"))
        .expect("ctx-index command");
    assert!(index_command.contains("!`"));
    assert!(index_command.contains("--repo-root"));
    assert!(index_command.contains("do not glob files"));
    assert!(index_command.contains("indexed_files:"));
}

#[test]
fn opencode_core_profile_keeps_only_the_lean_command_set() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["opencode", "install", "--profile", "core"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let commands_dir = tmp.path().join(".opencode/commands");
    for command in [
        "ctx.md",
        "ctx-doctor.md",
        "ctx-plan.md",
        "ctx-retrieve.md",
        "ctx-pack.md",
        "ctx-run.md",
        "ctx-prune-logs.md",
        "ctx-stats.md",
        "ctx-gain.md",
    ] {
        assert!(commands_dir.join(command).exists(), "missing {command}");
    }

    for command in [
        "ctx-dashboard.md",
        "ctx-read.md",
        "ctx-compare.md",
        "ctx-toolbook-import.md",
        "ctx-memory-search.md",
        "ctx-benchmark-memory-ab.md",
    ] {
        assert!(
            !commands_dir.join(command).exists(),
            "unexpected core command {command}"
        );
    }

    let instructions =
        fs::read_to_string(tmp.path().join(".opencode/instructions/ctx-host-first.md"))
            .expect("ctx host-first instructions");
    assert!(instructions.contains("Install profile: `core`"));
    assert!(instructions.contains("ctx opencode install --profile full"));
    assert!(!instructions.contains("/ctx-dashboard"));
    assert!(!instructions.contains("/ctx-toolbook-import"));
    assert!(
        !tmp.path()
            .join(".opencode/plugins/ctx-dashboard.tsx")
            .exists()
    );
    assert!(!tmp.path().join(".opencode/package.json").exists());
    assert!(!tmp.path().join(".opencode/tui.json").exists());
}
