mod heuristic;
mod parsers;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneReport {
    pub original_lines: usize,
    pub kept_lines: usize,
    pub output: String,
    pub included: Vec<String>,
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Candidate {
    pub order: usize,
    pub line: String,
    pub reason: String,
    pub priority: u8,
}

impl Candidate {
    pub(crate) fn new(
        order: usize,
        line: impl Into<String>,
        reason: impl Into<String>,
        priority: u8,
    ) -> Self {
        Self {
            order,
            line: line.into(),
            reason: reason.into(),
            priority,
        }
    }
}

/// Prune noisy command output while preserving diagnostic root-cause signals.
///
/// The implementation is deterministic and parser-pack based: known tool outputs
/// are recognized first, then a conservative heuristic fallback keeps generic
/// error/warning/failure lines. The output stays line-oriented so it can be piped
/// directly into agent prompts.
pub fn prune_logs(input: &str, max_lines: usize) -> PruneReport {
    let mut candidates = parsers::parse_log_candidates(input);

    if candidates.is_empty() {
        candidates = heuristic::fallback_log_candidates(input);
    }

    heuristic::finalize_report(input, candidates, max_lines.max(1), "log")
}

/// Prune a git diff to query-relevant file headers and hunks.
pub fn prune_diff(input: &str, query: &str, max_lines: usize) -> PruneReport {
    parsers::prune_git_diff(input, query, max_lines.max(1))
}

pub(crate) fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '-')
        .map(|part| part.trim().to_lowercase())
        .filter(|part| part.len() > 2)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenization_skips_short_words() {
        let tokens = tokenize_query("fix refresh in auth");
        assert_eq!(tokens, vec!["fix", "refresh", "auth"]);
    }
}
