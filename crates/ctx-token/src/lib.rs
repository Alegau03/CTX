pub fn estimate_tokens(text: &str) -> usize {
    // Deterministic local heuristic tuned for relative budgeting, not exact tokenizer parity.
    let words = text.split_whitespace().count();
    if words == 0 {
        return 0;
    }

    // Approximate BPE-ish overhead for punctuation/newlines.
    let extra = text
        .matches(|ch: char| !ch.is_alphanumeric() && !ch.is_whitespace())
        .count()
        / 4;
    words + extra + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_is_non_zero_for_single_word() {
        assert!(estimate_tokens("hello") > 0);
    }
}
