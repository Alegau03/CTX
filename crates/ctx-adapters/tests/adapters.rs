use ctx_adapters::{Agent, build_invocation, prepare_invocation};

#[test]
fn builds_codex_invocation() {
    let cmd = build_invocation(Agent::Codex, "fix auth", "context");
    assert!(cmd.contains("codex"));
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
