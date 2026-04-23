pub fn apply_pre_prompt_hook(task: &str, compact_context: &str) -> String {
    format!(
        "Task: {task}\n\nCompact Context:\n{compact_context}\n\nInstruction: solve using only relevant evidence above."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_is_deterministic() {
        let a = apply_pre_prompt_hook("x", "y");
        let b = apply_pre_prompt_hook("x", "y");
        assert_eq!(a, b);
    }
}
