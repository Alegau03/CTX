use ctx_adapters::{
    AdapterInvocation, AdapterRunStatus, Agent, build_invocation, execute_invocation_with_result,
    prepare_generic_invocation, prepare_invocation,
};

#[test]
fn builds_codex_invocation() {
    let cmd = build_invocation(Agent::Codex, "fix auth", "context");
    assert!(cmd.contains("codex"));
    assert!(cmd.contains("exec"));
    assert!(cmd.contains("fix auth"));
    assert!(cmd.contains("[CTX COMPACT CONTEXT]"));
}

#[test]
fn builds_opencode_invocation() {
    let cmd = build_invocation(Agent::OpenCode, "explain diff", "ctx");
    assert!(cmd.contains("opencode"));
    assert!(cmd.contains("run"));
}

#[test]
fn prepares_opencode_program_and_subcommand() {
    let invocation = prepare_invocation(Agent::OpenCode, "fix build", "ctx body");
    assert_eq!(invocation.program, "opencode");
    assert_eq!(invocation.args, vec!["run".to_string()]);
    assert!(invocation.prompt.contains("fix build"));
    assert!(invocation.prompt.contains("ctx body"));
}

#[test]
fn codex_uses_non_interactive_exec_template() {
    let invocation = prepare_invocation(Agent::Codex, "review diff", "compact ctx");
    assert_eq!(invocation.program, "codex");
    assert_eq!(invocation.args, vec!["exec".to_string()]);
    assert!(invocation.prompt.contains("review diff"));
    assert!(invocation.prompt.contains("[CTX COMPACT CONTEXT]"));
    assert!(invocation.prompt.contains("compact ctx"));
}

#[test]
fn claude_uses_print_mode_without_bare_mode() {
    let invocation = prepare_invocation(Agent::Claude, "fix flaky test", "compact ctx");
    assert_eq!(invocation.program, "claude");
    assert_eq!(invocation.args, vec!["-p".to_string()]);
    assert!(!invocation.args.iter().any(|arg| arg == "--bare"));
    assert!(invocation.prompt.contains("fix flaky test"));
    assert!(invocation.prompt.contains("compact ctx"));
}

#[test]
fn opencode_uses_run_template() {
    let invocation = prepare_invocation(Agent::OpenCode, "explain build", "compact ctx");
    assert_eq!(invocation.program, "opencode");
    assert_eq!(invocation.args, vec!["run".to_string()]);
    assert!(invocation.prompt.contains("explain build"));
}

#[test]
fn generic_adapter_uses_default_agent_when_no_command_is_configured() {
    let invocation = prepare_invocation(Agent::Generic, "summarize", "compact ctx");
    assert_eq!(invocation.program, "agent");
    assert!(invocation.args.is_empty());
}

#[test]
fn generic_adapter_parses_explicit_command_template() {
    let invocation = prepare_generic_invocation(
        "custom-agent --one-shot --format text",
        "summarize",
        "compact ctx",
    );
    assert_eq!(invocation.program, "custom-agent");
    assert_eq!(
        invocation.args,
        vec![
            "--one-shot".to_string(),
            "--format".to_string(),
            "text".to_string()
        ]
    );
    assert!(invocation.prompt.contains("summarize"));
}

#[cfg(unix)]
#[test]
fn execute_invocation_reports_success_with_fake_binary() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    let tmp = tempdir().expect("tempdir");
    let bin = tmp.path().join("fake-agent");
    fs::write(&bin, "#!/bin/sh\nexit 0\n").expect("write fake");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod");

    let invocation = AdapterInvocation {
        agent: Agent::Generic,
        program: bin.to_string_lossy().to_string(),
        args: vec!["run".to_string()],
        prompt: "hello ctx".to_string(),
    };

    let result = execute_invocation_with_result(&invocation).expect("execute");
    assert_eq!(result.status, AdapterRunStatus::Succeeded);
    assert_eq!(result.exit_code, Some(0));
    assert!(!result.fallback_used);
    assert!(result.command.contains("fake-agent"));
}

#[test]
fn missing_binary_returns_fallback_result() {
    let invocation = AdapterInvocation {
        agent: Agent::Claude,
        program: "ctx-definitely-missing-agent".to_string(),
        args: vec!["-p".to_string()],
        prompt: "hello ctx".to_string(),
    };

    let result = execute_invocation_with_result(&invocation).expect("fallback result");
    assert_eq!(result.status, AdapterRunStatus::Fallback);
    assert!(result.fallback_used);
    assert!(result.fallback_reason.unwrap().contains("not found"));
}
