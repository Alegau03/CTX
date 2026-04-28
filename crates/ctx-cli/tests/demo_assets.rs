use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn docs_reference_the_opencode_auth_lab_demo_fixture() {
    let root = repo_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("readme");
    let guide = fs::read_to_string(root.join("guide.md")).expect("guide");

    assert!(readme.contains("demo/fixtures/opencode-auth-lab"));
    assert!(guide.contains("opencode-auth-lab"));
}

#[test]
fn demo_fixture_contains_graph_relevant_source_and_test_files() {
    let root = repo_root().join("demo/fixtures/opencode-auth-lab");
    for path in [
        "src/auth/tokens.ts",
        "src/auth/session.ts",
        "src/http/refresh-route.ts",
        "src/lib/retry.ts",
        "tests/auth/refresh-route.test.ts",
        "tests/auth/session.test.ts",
    ] {
        assert!(root.join(path).exists(), "missing {path}");
    }
}

#[test]
fn demo_fixture_contains_agents_style_memory_seed_files() {
    let root = repo_root().join("demo/fixtures/opencode-auth-lab");
    for path in ["AGENTS.md", ".github/copilot-instructions.md"] {
        assert!(root.join(path).exists(), "missing {path}");
    }
}

#[test]
fn demo_fixture_contains_logs_diff_and_benchmark_inputs() {
    let root = repo_root().join("demo/fixtures/opencode-auth-lab");
    for path in [
        "logs/vitest-refresh-failure.log",
        "logs/noisy-ci.log",
        "diff/refresh-route.patch",
        "benchmarks/memory-suite.toml",
        "checklists/graph-memory-quality.md",
        "answers/markdown-answer.txt",
        "answers/graph-answer.txt",
        "expected/doctor.txt",
        "expected/memory-search-auth.txt",
        "expected/prune-logs.txt",
        "expected/pack-fragments.txt",
    ] {
        assert!(root.join(path).exists(), "missing {path}");
    }
}

#[test]
fn demo_scripts_exist_and_target_the_opencode_auth_lab_fixture() {
    let root = repo_root();
    for path in [
        "scripts/demo/opencode-auth-lab-smoke.sh",
        "scripts/demo/opencode-auth-lab-mcp-smoke.sh",
        "scripts/demo/opencode-auth-lab-benchmark.sh",
    ] {
        let body = fs::read_to_string(root.join(path)).expect("script exists");
        assert!(body.contains("demo/fixtures/opencode-auth-lab"));
    }
}

#[test]
fn docs_link_the_demo_walkthrough_and_script() {
    let root = repo_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("readme");
    let guide = fs::read_to_string(root.join("guide.md")).expect("guide");

    assert!(readme.contains("docs/demo-walkthrough.md"));
    assert!(guide.contains("docs/demo-script.md"));
}

#[test]
fn demo_fixture_contains_versioned_benchmark_reports() {
    let root = repo_root().join("demo/fixtures/opencode-auth-lab/benchmarks");
    assert!(root.join("report.md").exists());
    assert!(root.join("report.json").exists());
}
