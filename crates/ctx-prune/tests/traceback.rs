use ctx_prune::prune_logs;

#[test]
fn python_traceback_keeps_frames_code_and_root_cause() {
    let input = r#"
collected 12 items
...........
Traceback (most recent call last):
  File "app/service.py", line 44, in refresh
    return rotate_token(user)
  File "app/tokens.py", line 12, in rotate_token
    raise ValueError("expired token")
ValueError: expired token
PASSED tests/test_health.py::test_ok
"#;

    let report = prune_logs(input, 20);

    assert!(report.output.contains("Traceback (most recent call last):"));
    assert!(report.output.contains("File \"app/service.py\", line 44"));
    assert!(report.output.contains("return rotate_token(user)"));
    assert!(report.output.contains("File \"app/tokens.py\", line 12"));
    assert!(report.output.contains("raise ValueError"));
    assert!(report.output.contains("ValueError: expired token"));
    assert!(!report.output.contains("PASSED tests/test_health"));
}
