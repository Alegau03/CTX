use std::fs;

use ctx_core::{ReadMode, init_repo, run_read};
use tempfile::tempdir;

#[test]
fn full_read_returns_file_content_on_first_access() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let path = tmp.path().join("src/auth.ts");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(
        &path,
        "export function validateRefreshToken(token: string) {\n  return token.length > 10;\n}\n",
    )
    .expect("write file");

    let report = run_read(tmp.path(), "src/auth.ts", ReadMode::Full).expect("run read");

    assert_eq!(report.mode, ReadMode::Full);
    assert!(!report.cache_hit);
    assert!(report.output.contains("validateRefreshToken"));
    assert!(report.output.contains("return token.length > 10"));
}

#[test]
fn outline_read_surfaces_symbols_without_full_body_dump() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let path = tmp.path().join("src/session.ts");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(
        &path,
        "export function hydrateSession(token: string) {\n  const refresh = token.trim();\n  return refresh;\n}\n",
    )
    .expect("write file");

    let report = run_read(tmp.path(), "src/session.ts", ReadMode::Outline).expect("run read");

    assert_eq!(report.mode, ReadMode::Outline);
    assert!(!report.cache_hit);
    assert!(report.output.contains("hydrateSession"));
    assert!(!report.output.contains("const refresh = token.trim()"));
}

#[test]
fn digest_reread_hits_cache_for_unchanged_file() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let path = tmp.path().join("docs/runbook.md");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(
        &path,
        "# Docker Compose\n\n## Services\n\nUse docker compose up.\n",
    )
    .expect("write file");

    let first = run_read(tmp.path(), "docs/runbook.md", ReadMode::Full).expect("seed read");
    let reread = run_read(tmp.path(), "docs/runbook.md", ReadMode::Digest).expect("digest reread");

    assert!(!first.cache_hit);
    assert!(reread.cache_hit);
    assert_eq!(first.fingerprint, reread.fingerprint);
    assert!(reread.output.contains("cache: hit"));
    assert!(reread.output.contains("Docker Compose"));
    assert!(!reread.output.contains("Use docker compose up."));
}

#[test]
fn digest_reread_invalidates_after_file_change() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    let path = tmp.path().join("src/login.ts");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, "export const login = () => true;\n").expect("write file");

    let first = run_read(tmp.path(), "src/login.ts", ReadMode::Full).expect("seed read");
    fs::write(&path, "export const login = () => false;\n").expect("rewrite file");

    let reread = run_read(tmp.path(), "src/login.ts", ReadMode::Digest).expect("digest reread");

    assert!(!reread.cache_hit);
    assert_ne!(first.fingerprint, reread.fingerprint);
    assert!(reread.output.contains("cache: miss"));
}
