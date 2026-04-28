use std::fs;

use ctx_core::{
    init_repo, run_memory_ab_benchmark, run_memory_ab_benchmark_suite,
    run_memory_bootstrap_markdown, run_memory_delete, run_memory_export_markdown, run_memory_get,
    run_memory_import_markdown, run_memory_list, run_memory_search, run_memory_set, run_pack,
};
use tempfile::tempdir;

#[test]
fn memory_directive_crud_roundtrip() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    run_memory_set(
        tmp.path(),
        "testing.always_run",
        "Run unit tests after every implementation change.",
        "project",
        "manual",
    )
    .expect("set");

    let loaded = run_memory_get(tmp.path(), "testing.always_run")
        .expect("get")
        .expect("existing");
    assert_eq!(loaded.scope, "project");
    assert_eq!(loaded.source, "manual");

    run_memory_set(
        tmp.path(),
        "testing.always_run",
        "Run unit and smoke tests after every implementation change.",
        "project",
        "model",
    )
    .expect("update");

    let listed = run_memory_list(tmp.path(), Some("project"), 10).expect("list");
    assert!(
        listed
            .iter()
            .any(|d| { d.key == "testing.always_run" && d.body.contains("unit and smoke tests") })
    );

    let removed = run_memory_delete(tmp.path(), "testing.always_run").expect("delete");
    assert!(removed);
    assert!(
        run_memory_get(tmp.path(), "testing.always_run")
            .expect("get after delete")
            .is_none()
    );
}

#[test]
fn run_pack_includes_memory_directives() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    run_memory_set(
        tmp.path(),
        "testing.mandatory",
        "Always run targeted tests before claiming task completion.",
        "project",
        "manual",
    )
    .expect("set");

    let packed =
        run_pack(tmp.path(), "run targeted tests for auth", Some(200), None).expect("pack");
    assert!(packed.compact_context.contains("testing.mandatory"));
    assert!(packed.compact_context.contains("Always run targeted tests"));
}

#[test]
fn memory_ab_benchmark_compares_graph_and_markdown_tokens() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    run_memory_set(
        tmp.path(),
        "tests.mandatory",
        "Run test suite before merge.",
        "project",
        "manual",
    )
    .expect("set");
    run_memory_set(
        tmp.path(),
        "quality.no-shortcuts",
        "Never skip failing tests; fix root cause.",
        "project",
        "manual",
    )
    .expect("set");

    let markdown_path = tmp.path().join("AGENTS.md");
    fs::write(
        &markdown_path,
        r#"
# Engineering Rules
- Run test suite before merge.
- Never skip failing tests; fix root cause.
- Keep backward compatibility unless explicitly requested.
"#,
    )
    .expect("write markdown");

    let result = run_memory_ab_benchmark(
        tmp.path(),
        "run tests and fix root cause",
        &markdown_path,
        10,
        None,
        None,
        None,
    )
    .expect("benchmark");

    assert!(result.markdown_tokens > 0);
    assert!(result.graph_memory_tokens > 0);
    assert!(result.graph_directives_count >= 2);
}

#[test]
fn memory_import_and_export_markdown_roundtrip() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    let agents = tmp.path().join("AGENTS.md");
    fs::write(
        &agents,
        r#"
# Team Rules
- Always run tests.
- Never bypass root cause fixes.
"#,
    )
    .expect("write");

    let imported =
        run_memory_import_markdown(tmp.path(), &agents, "project", "markdown", Some("agents"))
            .expect("import");
    assert!(imported.imported >= 2);

    let exported = tmp.path().join("AGENTS.generated.md");
    let report = run_memory_export_markdown(tmp.path(), &exported, Some("project"), 100, None)
        .expect("export");
    assert!(report.directives >= 2);
    let body = fs::read_to_string(&exported).expect("read export");
    assert!(body.contains("Graph Memory Directives"));
    assert!(body.contains("Always run tests"));
}

#[test]
fn memory_bootstrap_imports_default_agents_style_files() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    fs::write(
        tmp.path().join("AGENTS.md"),
        "# Team Rules\n- Run targeted tests before completion.\n- Fix root cause before merge.\n",
    )
    .expect("write agents");
    fs::write(
        tmp.path().join("CLAUDE.md"),
        "# Claude Rules\n- Keep route, session, and token semantics aligned.\n",
    )
    .expect("write claude");
    fs::write(
        tmp.path().join("CODEX.md"),
        "# Codex Rules\n- Preserve strong assertions in refresh token tests.\n",
    )
    .expect("write codex");
    fs::create_dir_all(tmp.path().join(".github")).expect("create .github");
    fs::write(
        tmp.path().join(".github/copilot-instructions.md"),
        "# Copilot Instructions\n- Prefer auth-focused fixtures for token tests.\n",
    )
    .expect("write copilot instructions");

    let report = run_memory_bootstrap_markdown(tmp.path(), &[], "project", "markdown")
        .expect("bootstrap markdown");

    assert_eq!(report.imported_files, 4);
    assert!(report.imported_directives >= 5);
    assert!(
        report
            .reports
            .iter()
            .any(|item| item.markdown_path.ends_with("AGENTS.md"))
    );
    assert!(
        report
            .reports
            .iter()
            .any(|item| item.markdown_path.ends_with("CLAUDE.md"))
    );
    assert!(
        report
            .reports
            .iter()
            .any(|item| item.markdown_path.ends_with("CODEX.md"))
    );
    assert!(report.reports.iter().any(|item| {
        item.markdown_path
            .ends_with(".github/copilot-instructions.md")
    }));

    let directives = run_memory_list(tmp.path(), Some("project"), 20).expect("list directives");
    assert!(
        directives
            .iter()
            .any(|item| item.body.contains("Run targeted tests before completion"))
    );
    assert!(directives.iter().any(|item| {
        item.body
            .contains("Keep route, session, and token semantics aligned")
    }));
    assert!(
        directives
            .iter()
            .any(|item| item.body.contains("Preserve strong assertions"))
    );
    assert!(
        directives
            .iter()
            .any(|item| item.body.contains("Prefer auth-focused fixtures"))
    );
}

#[test]
fn memory_search_returns_relevant_directives_for_a_topic() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    run_memory_set(
        tmp.path(),
        "testing.always_run",
        "Run targeted tests before completion and before merge.",
        "project",
        "manual",
    )
    .expect("set tests rule");
    run_memory_set(
        tmp.path(),
        "auth.root_cause",
        "Fix auth root cause instead of bypassing refresh token failures.",
        "project",
        "manual",
    )
    .expect("set auth rule");
    run_memory_set(
        tmp.path(),
        "style.docs",
        "Keep guides concise and update examples when behavior changes.",
        "project",
        "manual",
    )
    .expect("set docs rule");

    let results = run_memory_search(tmp.path(), "auth tests root cause", Some("project"), 10)
        .expect("search memory");

    assert!(results.len() >= 2);
    assert_eq!(results[0].key, "auth.root_cause");
    assert!(results.iter().any(|item| item.key == "testing.always_run"));
    assert!(results.iter().all(|item| item.scope == "project"));
}

#[test]
fn reimporting_markdown_replaces_stale_directives_for_the_same_prefix() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    let agents = tmp.path().join("AGENTS.md");
    fs::write(
        &agents,
        "# Rules\n- Run targeted tests before completion.\n- Fix auth root cause before merge.\n",
    )
    .expect("write agents");

    run_memory_import_markdown(tmp.path(), &agents, "project", "markdown", Some("agents"))
        .expect("first import");

    fs::write(
        &agents,
        "# Rules\n- Run targeted tests before completion.\n",
    )
    .expect("rewrite agents");

    run_memory_import_markdown(tmp.path(), &agents, "project", "markdown", Some("agents"))
        .expect("second import");

    let directives = run_memory_list(tmp.path(), Some("project"), 20).expect("list directives");
    assert_eq!(
        directives
            .iter()
            .filter(|item| item.key.starts_with("agents."))
            .count(),
        1
    );
    assert!(
        directives
            .iter()
            .all(|item| !item.body.contains("Fix auth root cause"))
    );
}

#[test]
fn memory_ab_benchmark_evaluates_quality_with_checklist_and_answers() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    run_memory_set(
        tmp.path(),
        "tests.required",
        "Run tests before merge.",
        "project",
        "manual",
    )
    .expect("set");
    run_memory_set(
        tmp.path(),
        "quality.root_cause",
        "Fix root cause and avoid temporary bypasses.",
        "project",
        "manual",
    )
    .expect("set");

    let markdown_path = tmp.path().join("AGENTS.md");
    fs::write(
        &markdown_path,
        "# Rules\n- Run tests before merge.\n- Keep backward compatibility.\n",
    )
    .expect("write markdown");

    let checklist = tmp.path().join("quality-checklist.md");
    fs::write(
        &checklist,
        "- Run tests before merge.\n- Fix root cause and avoid temporary bypasses.\n",
    )
    .expect("write checklist");

    let markdown_answer = tmp.path().join("markdown_answer.txt");
    fs::write(
        &markdown_answer,
        "I will run tests before merge but I may temporarily bypass root cause.",
    )
    .expect("write md answer");

    let graph_answer = tmp.path().join("graph_answer.txt");
    fs::write(
        &graph_answer,
        "I will run tests before merge and fix root cause and avoid temporary bypasses.",
    )
    .expect("write graph answer");

    let result = run_memory_ab_benchmark(
        tmp.path(),
        "run tests and fix root cause",
        &markdown_path,
        20,
        Some(&checklist),
        Some(&markdown_answer),
        Some(&graph_answer),
    )
    .expect("benchmark");

    assert!(result.markdown_success_rate.is_some());
    assert!(result.graph_success_rate.is_some());
    assert_eq!(result.quality_winner.as_deref(), Some("graph"));
}

#[test]
fn memory_ab_benchmark_suite_writes_publishable_reports() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    run_memory_set(
        tmp.path(),
        "tests.required",
        "Run tests before merge.",
        "project",
        "manual",
    )
    .expect("set");
    run_memory_set(
        tmp.path(),
        "quality.root_cause",
        "Fix root cause and avoid temporary bypasses.",
        "project",
        "manual",
    )
    .expect("set");

    fs::write(
        tmp.path().join("AGENTS.md"),
        "# Rules\n- Run tests before merge.\n- Keep backward compatibility.\n",
    )
    .expect("write markdown");
    fs::write(
        tmp.path().join("checklist.md"),
        "- Run tests before merge.\n- Fix root cause and avoid temporary bypasses.\n",
    )
    .expect("write checklist");
    fs::write(
        tmp.path().join("markdown_answer.txt"),
        "I will run tests before merge.",
    )
    .expect("write markdown answer");
    fs::write(
        tmp.path().join("graph_answer.txt"),
        "I will run tests before merge and fix root cause and avoid temporary bypasses.",
    )
    .expect("write graph answer");

    let spec = tmp.path().join("memory-suite.toml");
    fs::write(
        &spec,
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

    let report_md = tmp.path().join("benchmark-report.md");
    let report_json = tmp.path().join("benchmark-report.json");

    let report = run_memory_ab_benchmark_suite(tmp.path(), &spec, &report_md, Some(&report_json))
        .expect("suite");

    assert_eq!(report.summary.case_count, 1);
    assert!(report.summary.avg_token_reduction_pct.is_finite());
    assert!(
        report.summary.graph_quality_wins
            + report.summary.markdown_quality_wins
            + report.summary.ties
            <= report.summary.case_count
    );
    assert!(report_md.exists());
    assert!(report_json.exists());

    let markdown = fs::read_to_string(&report_md).expect("read markdown report");
    assert!(markdown.contains("# CTX Memory Benchmark"));
    assert!(markdown.contains("auth_rules"));
    assert!(markdown.contains("Token reduction"));

    let json = fs::read_to_string(&report_json).expect("read json report");
    assert!(json.contains("\"case_count\": 1"));
    assert!(json.contains("\"graph_quality_wins\":"));
}
