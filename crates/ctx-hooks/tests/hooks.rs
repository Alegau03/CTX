use ctx_hooks::apply_pre_prompt_hook;

#[test]
fn hook_injects_compact_context_block() {
    let output = apply_pre_prompt_hook("fix test", "root cause here");
    assert!(output.contains("Task:"));
    assert!(output.contains("Compact Context:"));
    assert!(output.contains("root cause here"));
}
