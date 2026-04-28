use ctx_telemetry::{StatsSnapshot, read_latest_stats, write_latest_stats};
use tempfile::tempdir;

#[test]
fn writes_and_reads_stats_snapshot() {
    let tmp = tempdir().expect("tempdir");
    let stats_dir = tmp.path().join(".ctx/stats");

    let snapshot = StatsSnapshot {
        original_tokens: 1000,
        packed_tokens: 200,
        reduction_pct: 80.0,
        latency_ms: 120,
        agent: None,
        command: None,
        status: None,
        exit_code: None,
        fallback_used: false,
        pack_path: None,
    };

    write_latest_stats(&stats_dir, &snapshot).expect("write");
    let loaded = read_latest_stats(&stats_dir).expect("read");

    assert_eq!(loaded.packed_tokens, 200);
    assert_eq!(loaded.reduction_pct, 80.0);
}

#[test]
fn reads_legacy_stats_snapshot_without_adapter_fields() {
    let tmp = tempdir().expect("tempdir");
    let stats_dir = tmp.path().join(".ctx/stats");
    std::fs::create_dir_all(&stats_dir).expect("mkdir");
    std::fs::write(
        stats_dir.join("latest.json"),
        r#"{"original_tokens":1000,"packed_tokens":250,"reduction_pct":75.0,"latency_ms":12}"#,
    )
    .expect("write legacy stats");

    let loaded = read_latest_stats(&stats_dir).expect("read legacy");
    assert_eq!(loaded.packed_tokens, 250);
    assert_eq!(loaded.agent, None);
    assert!(!loaded.fallback_used);
}

#[test]
fn writes_invocation_fields_in_latest_stats() {
    let tmp = tempdir().expect("tempdir");
    let stats_dir = tmp.path().join(".ctx/stats");

    let snapshot = StatsSnapshot {
        original_tokens: 1000,
        packed_tokens: 200,
        reduction_pct: 80.0,
        latency_ms: 44,
        agent: Some("opencode".to_string()),
        command: Some("/ctx-pack \"fix\"".to_string()),
        status: Some("succeeded".to_string()),
        exit_code: Some(0),
        fallback_used: false,
        pack_path: Some(".ctx/packs/1.json".to_string()),
    };

    write_latest_stats(&stats_dir, &snapshot).expect("write");
    let body = std::fs::read_to_string(stats_dir.join("latest.json")).expect("read body");
    assert!(body.contains("opencode"));
    assert!(body.contains("fallback_used"));
}
