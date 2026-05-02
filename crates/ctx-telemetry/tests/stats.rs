use ctx_telemetry::{
    GainReport, StatsSnapshot, build_gain_report, read_latest_stats, read_stats_history,
    write_latest_stats,
};
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
        query: None,
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
        query: Some("fix auth".to_string()),
    };

    write_latest_stats(&stats_dir, &snapshot).expect("write");
    let body = std::fs::read_to_string(stats_dir.join("latest.json")).expect("read body");
    assert!(body.contains("opencode"));
    assert!(body.contains("fallback_used"));
    assert!(body.contains("fix auth"));
}

#[test]
fn writes_history_snapshots_alongside_latest_stats() {
    let tmp = tempdir().expect("tempdir");
    let stats_dir = tmp.path().join(".ctx/stats");

    let snapshot = StatsSnapshot {
        original_tokens: 900,
        packed_tokens: 300,
        reduction_pct: 66.67,
        latency_ms: 12,
        agent: Some("opencode".to_string()),
        command: Some("/ctx-pack \"auth\"".to_string()),
        status: Some("succeeded".to_string()),
        exit_code: Some(0),
        fallback_used: false,
        pack_path: Some(".ctx/packs/1.json".to_string()),
        query: Some("auth".to_string()),
    };

    write_latest_stats(&stats_dir, &snapshot).expect("write");
    let history = read_stats_history(&stats_dir, 10).expect("history");

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].query.as_deref(), Some("auth"));
}

#[test]
fn gain_report_aggregates_recent_stats_history() {
    let tmp = tempdir().expect("tempdir");
    let stats_dir = tmp.path().join(".ctx/stats");

    for (query, original, packed, reduction) in [
        ("fix auth", 1000, 250, 75.0),
        ("fix auth", 900, 300, 66.67),
        ("plan login", 1200, 480, 60.0),
    ] {
        let snapshot = StatsSnapshot {
            original_tokens: original,
            packed_tokens: packed,
            reduction_pct: reduction,
            latency_ms: 20,
            agent: Some("opencode".to_string()),
            command: Some("/ctx-pack".to_string()),
            status: Some("succeeded".to_string()),
            exit_code: Some(0),
            fallback_used: false,
            pack_path: None,
            query: Some(query.to_string()),
        };
        write_latest_stats(&stats_dir, &snapshot).expect("write");
    }

    let report: GainReport = build_gain_report(&stats_dir, 20).expect("gain report");
    assert_eq!(report.sampled_runs, 3);
    assert!(report.estimated_tokens_saved > 0);
    assert!(report.average_reduction_pct > 0.0);
    assert_eq!(
        report.top_queries.first().map(|item| item.query.as_str()),
        Some("fix auth")
    );
    assert_eq!(report.top_queries.first().map(|item| item.runs), Some(2));
}
