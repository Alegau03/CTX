use ctx_graph::GraphStore;
use tempfile::tempdir;

#[test]
fn upsert_symbol_and_query_by_term() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("graph.db");
    let store = GraphStore::open(&db).expect("open");
    store.init_schema().expect("schema");

    let symbol_id = store
        .upsert_symbol(
            "src/auth.rs",
            "validate_refresh_token",
            "function",
            "fn validate_refresh_token()",
        )
        .expect("symbol");

    assert!(symbol_id > 0);
    let hits = store.search_symbols("refresh").expect("search");
    assert!(hits.iter().any(|h| h.name == "validate_refresh_token"));
}

#[test]
fn link_symbols_and_list_neighbors() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("graph.db");
    let store = GraphStore::open(&db).expect("open");
    store.init_schema().expect("schema");

    let src = store
        .upsert_symbol(
            "src/auth.rs",
            "decode_token",
            "function",
            "fn decode_token()",
        )
        .expect("src");
    let dst = store
        .upsert_symbol(
            "src/auth.rs",
            "validate_refresh_token",
            "function",
            "fn validate_refresh_token()",
        )
        .expect("dst");

    store.link_symbols(src, dst, "calls", None).expect("link");

    let neighbors = store
        .related_symbols("decode_token", 10)
        .expect("neighbors");
    assert!(neighbors.iter().any(|n| n.name == "validate_refresh_token"));
}

#[test]
fn snippet_fts_search_returns_relevant_snippet() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("graph.db");
    let store = GraphStore::open(&db).expect("open");
    store.init_schema().expect("schema");

    store
        .upsert_symbol(
            "src/auth.rs",
            "decode_token",
            "function",
            "fn decode_token()",
        )
        .expect("symbol");
    store
        .add_snippet(
            "src/auth.rs",
            Some("decode_token"),
            "decode token and validate audience",
        )
        .expect("snippet");

    let hits = store.search_snippets("decode", 10).expect("fts");
    assert!(!hits.is_empty());
    assert!(hits[0].content.contains("decode token"));
}

#[test]
fn record_failure_and_recent_decisions_are_queryable() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("graph.db");
    let store = GraphStore::open(&db).expect("open");
    store.init_schema().expect("schema");

    let run_id = store.record_run("pytest -q", "failed").expect("run");
    store
        .record_failure(run_id, "traceback in auth", Some("decode token"))
        .expect("failure");
    store
        .record_decision("auth-fix", "keep public signature stable")
        .expect("decision");

    let failures = store.recent_failures(10).expect("failures");
    let decisions = store.recent_decisions(10).expect("decisions");

    assert!(failures.iter().any(|f| f.message.contains("auth")));
    assert!(decisions.iter().any(|d| d.contains("auth-fix")));
}

#[test]
fn memory_directives_support_crud_and_search() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("graph.db");
    let store = GraphStore::open(&db).expect("open");
    store.init_schema().expect("schema");

    let id = store
        .upsert_memory_directive(
            "testing.always_run",
            "Every change must run targeted tests before completion.",
            "project",
            "manual",
        )
        .expect("upsert");
    assert!(id > 0);

    let loaded = store
        .get_memory_directive("testing.always_run")
        .expect("get")
        .expect("existing");
    assert_eq!(loaded.scope, "project");
    assert_eq!(loaded.source, "manual");

    store
        .upsert_memory_directive(
            "testing.always_run",
            "Every change must run targeted and smoke tests before completion.",
            "project",
            "model",
        )
        .expect("update");

    let hits = store
        .search_memory_directives("smoke tests completion", 10)
        .expect("search");
    assert!(!hits.is_empty());
    assert_eq!(hits[0].key, "testing.always_run");

    let all = store
        .list_memory_directives(Some("project"), 10)
        .expect("list");
    assert!(all.iter().any(|d| d.key == "testing.always_run"));

    let deleted = store
        .delete_memory_directive("testing.always_run")
        .expect("delete");
    assert!(deleted);
    assert!(
        store
            .get_memory_directive("testing.always_run")
            .expect("reload")
            .is_none()
    );
}
