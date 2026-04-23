use std::fs;

use ctx_core::{
    init_repo, run_memory_ab_benchmark, run_memory_delete, run_memory_export_markdown,
    run_memory_get, run_memory_import_markdown, run_memory_list, run_memory_set, run_pack,
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
