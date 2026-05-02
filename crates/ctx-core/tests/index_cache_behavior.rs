use std::fs;

use ctx_core::{init_repo, run_index};
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn repeated_index_reuses_unchanged_files() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(
        tmp.path().join("src/auth.rs"),
        "fn validate_refresh_token() -> bool { true }\n",
    )
    .expect("write");

    let first = run_index(tmp.path(), &[]).expect("first index");
    let second = run_index(tmp.path(), &[]).expect("second index");

    assert_eq!(first, 1);
    assert_eq!(second, 0);

    let report: Value = serde_json::from_str(
        &fs::read_to_string(tmp.path().join(".ctx/cache/index-report.json")).expect("report"),
    )
    .expect("report json");
    assert_eq!(report["scanned_files"].as_u64(), Some(1));
    assert_eq!(report["indexed_files"].as_u64(), Some(0));
    assert_eq!(report["reused_files"].as_u64(), Some(1));
    assert_eq!(report["changed_files"].as_u64(), Some(0));
    assert_eq!(report["new_files"].as_u64(), Some(0));
}

#[test]
fn changed_file_invalidates_index_cache() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    let path = tmp.path().join("src/session.ts");
    fs::write(&path, "export const login = () => true;\n").expect("write");

    run_index(tmp.path(), &[]).expect("seed index");
    fs::write(&path, "export const login = () => false;\n").expect("rewrite");

    let changed = run_index(tmp.path(), &[]).expect("changed index");
    assert_eq!(changed, 1);

    let report: Value = serde_json::from_str(
        &fs::read_to_string(tmp.path().join(".ctx/cache/index-report.json")).expect("report"),
    )
    .expect("report json");
    assert_eq!(report["indexed_files"].as_u64(), Some(1));
    assert_eq!(report["reused_files"].as_u64(), Some(0));
    assert_eq!(report["changed_files"].as_u64(), Some(1));
    assert_eq!(report["new_files"].as_u64(), Some(0));
}
