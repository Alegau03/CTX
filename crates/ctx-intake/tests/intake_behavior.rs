use ctx_intake::{Intent, QueryIntake, detect_intent};

#[test]
fn detects_debug_intent() {
    assert_eq!(detect_intent("fix failing pytest in auth"), Intent::Debug);
}

#[test]
fn detects_refactor_intent() {
    assert_eq!(detect_intent("refactor data loader"), Intent::Refactor);
}

#[test]
fn intake_normalization_captures_query_and_intent() {
    let intake = QueryIntake::new("Review the last diff and find risky changes", ".");
    assert_eq!(intake.task, "Review the last diff and find risky changes");
    assert_eq!(intake.intent, Intent::Review);
    assert_eq!(intake.repo_root, ".");
}
