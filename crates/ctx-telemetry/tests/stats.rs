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
    };

    write_latest_stats(&stats_dir, &snapshot).expect("write");
    let loaded = read_latest_stats(&stats_dir).expect("read");

    assert_eq!(loaded.packed_tokens, 200);
    assert_eq!(loaded.reduction_pct, 80.0);
}
