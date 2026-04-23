use ctx_graph::GraphStore;
use tempfile::tempdir;

#[test]
fn initializes_schema_and_indexes_file() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("graph.db");
    let store = GraphStore::open(&db_path).expect("open");
    store.init_schema().expect("schema");

    store.index_file("src/auth.rs").expect("index");
    let matches = store.query_files("auth").expect("query");

    assert!(matches.iter().any(|p| p == "src/auth.rs"));
}

#[test]
fn upsert_file_does_not_duplicate_entries() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("graph.db");
    let store = GraphStore::open(&db_path).expect("open");
    store.init_schema().expect("schema");

    store.index_file("src/a.rs").expect("index1");
    store.index_file("src/a.rs").expect("index2");

    let all = store.query_files("src/a.rs").expect("query");
    assert_eq!(all.len(), 1);
}
