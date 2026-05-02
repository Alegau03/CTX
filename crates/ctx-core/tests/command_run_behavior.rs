use std::fs;

use ctx_core::{init_repo, run_command};
use tempfile::tempdir;

#[test]
fn command_run_captures_raw_logs_and_exit_code() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    let report = run_command(tmp.path(), "printf 'build ok\\n'").expect("run command");

    assert_eq!(report.exit_code, 0);
    assert!(report.pruned_output.contains("build ok"));
    assert!(fs::metadata(&report.raw_log_path).is_ok());
}

#[test]
fn command_run_prunes_failure_output_to_root_cause() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    let report = run_command(
        tmp.path(),
        "printf 'PASS precheck\\nERROR token decode failed\\nTraceback line 2\\n'; exit 1",
    )
    .expect("run failing command");

    assert_eq!(report.exit_code, 1);
    assert!(report.pruned_output.contains("ERROR token decode failed"));
    assert!(!report.pruned_output.contains("PASS precheck"));
    assert!(report.summary.contains("ERROR token decode failed"));
}
