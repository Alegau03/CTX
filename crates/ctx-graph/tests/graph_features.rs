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
fn invocation_runs_persist_full_metadata() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("graph.db");
    let store = GraphStore::open(&db).expect("open");
    store.init_schema().expect("schema");

    let run_id = store
        .record_invocation_run(&ctx_graph::RunInsert {
            command: "/ctx-pack \"fix auth\"".to_string(),
            status: "succeeded".to_string(),
            agent: Some("opencode".to_string()),
            exit_code: Some(0),
            duration_ms: Some(42),
            original_tokens: Some(1200),
            packed_tokens: Some(240),
            reduction_pct: Some(80.0),
            fallback_used: false,
            pack_path: Some(".ctx/packs/123.json".to_string()),
        })
        .expect("record run");

    assert!(run_id > 0);
    let runs = store.recent_runs(5).expect("recent runs");
    assert_eq!(runs[0].agent.as_deref(), Some("opencode"));
    assert_eq!(runs[0].status, "succeeded");
    assert_eq!(runs[0].packed_tokens, Some(240));
    assert!(!runs[0].fallback_used);
}

#[test]
fn init_schema_migrates_existing_runs_table() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("graph.db");
    {
        let conn = rusqlite::Connection::open(&db).expect("open raw");
        conn.execute_batch(
            "CREATE TABLE runs (
              id INTEGER PRIMARY KEY,
              task_id INTEGER,
              command TEXT NOT NULL,
              status TEXT NOT NULL,
              created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .expect("old schema");
    }

    let store = GraphStore::open(&db).expect("open store");
    store.init_schema().expect("migrate schema");
    store
        .record_invocation_run(&ctx_graph::RunInsert {
            command: "/ctx-ask \"review\"".to_string(),
            status: "fallback".to_string(),
            agent: Some("opencode".to_string()),
            exit_code: None,
            duration_ms: Some(1),
            original_tokens: Some(100),
            packed_tokens: Some(20),
            reduction_pct: Some(80.0),
            fallback_used: true,
            pack_path: None,
        })
        .expect("record after migrate");

    let runs = store.recent_runs(1).expect("recent");
    assert_eq!(runs[0].agent.as_deref(), Some("opencode"));
    assert!(runs[0].fallback_used);
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

#[test]
fn exact_symbol_lookup_returns_only_exact_matches() {
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
        .expect("decode");
    store
        .upsert_symbol(
            "src/auth.rs",
            "decode_token_strict",
            "function",
            "fn decode_token_strict()",
        )
        .expect("decode strict");

    let hits = store
        .find_symbols_by_exact_name("decode_token", 10)
        .expect("exact lookup");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].name, "decode_token");
}
