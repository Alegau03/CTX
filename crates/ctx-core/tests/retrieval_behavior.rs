use std::fs;

use ctx_core::{init_repo, run_index, run_retrieve};
use tempfile::tempdir;

#[test]
fn retrieval_returns_relevant_auth_context() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(
        tmp.path().join("src/auth.rs"),
        r#"
use crate::tokens::decode_token;

fn validate_refresh_token(token: &str) -> bool {
    decode_token(token)
}

fn decode_token(token: &str) -> bool {
    !token.is_empty()
}
"#,
    )
    .expect("write");

    run_index(tmp.path(), &[]).expect("index");

    let hits = run_retrieve(tmp.path(), "fix refresh token decode failure", 5).expect("retrieve");
    assert!(!hits.is_empty());
    assert!(
        hits.iter()
            .any(|h| h.content.contains("validate_refresh_token"))
    );
}

#[test]
fn retrieval_respects_limit() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    fs::write(
        tmp.path().join("src/a.rs"),
        "fn a() {}\nfn b() {}\nfn c() {}\n",
    )
    .expect("write");

    run_index(tmp.path(), &[]).expect("index");

    let hits = run_retrieve(tmp.path(), "function", 2).expect("retrieve");
    assert!(hits.len() <= 2);
}
