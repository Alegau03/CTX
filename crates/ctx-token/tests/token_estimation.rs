use ctx_token::estimate_tokens;

#[test]
fn empty_text_has_zero_tokens() {
    assert_eq!(estimate_tokens(""), 0);
}

#[test]
fn token_estimator_scales_with_word_count() {
    let short = estimate_tokens("hello world");
    let long = estimate_tokens("hello world this is a much longer input with many words");
    assert!(long > short);
}

#[test]
fn punctuation_and_whitespace_are_handled() {
    let estimate = estimate_tokens("error: failed\n\ntraceback line 2");
    assert!(estimate >= 4);
}
