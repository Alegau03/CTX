use crate::{Candidate, PruneReport};
use regex::Regex;
use std::collections::{HashMap, HashSet};

pub(crate) fn fallback_log_candidates(input: &str) -> Vec<Candidate> {
    let keep_patterns = [
        (
            Regex::new(r"(?i)\berror\b").expect("valid regex"),
            "heuristic:error",
            95,
        ),
        (
            Regex::new(r"(?i)\bfail(ed|ure)?\b").expect("valid regex"),
            "heuristic:failure",
            95,
        ),
        (
            Regex::new(r"(?i)traceback").expect("valid regex"),
            "heuristic:traceback",
            100,
        ),
        (
            Regex::new(r"(?i)exception").expect("valid regex"),
            "heuristic:exception",
            95,
        ),
        (
            Regex::new(r"(?i)warning").expect("valid regex"),
            "heuristic:warning",
            60,
        ),
        (
            Regex::new(r"(?i)panic").expect("valid regex"),
            "heuristic:panic",
            100,
        ),
    ];

    input
        .lines()
        .enumerate()
        .filter_map(|(order, raw)| {
            let line = raw.trim();
            if line.is_empty() {
                return None;
            }

            keep_patterns
                .iter()
                .find(|(rx, _, _)| rx.is_match(line))
                .map(|(_, reason, priority)| Candidate::new(order, line, *reason, *priority))
        })
        .collect()
}

pub(crate) fn finalize_report(
    input: &str,
    candidates: Vec<Candidate>,
    max_lines: usize,
    scope: &str,
) -> PruneReport {
    let original_lines = input.lines().count();
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    let mut excluded = Vec::new();
    let mut duplicate_count = 0usize;

    for candidate in candidates {
        let normalized = candidate.line.trim().to_string();
        if normalized.is_empty() {
            continue;
        }

        if !seen.insert(normalized.clone()) {
            duplicate_count += 1;
            continue;
        }

        deduped.push(Candidate {
            line: normalized,
            ..candidate
        });
    }

    if duplicate_count > 0 {
        excluded.push(format!("duplicate lines removed: {duplicate_count}"));
    }

    let before_budget = deduped.len();
    let mut selected = if deduped.len() > max_lines {
        let mut by_priority = deduped.clone();
        by_priority.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.order.cmp(&b.order))
        });
        by_priority.truncate(max_lines);
        by_priority.sort_by_key(|candidate| candidate.order);
        excluded.push(format!(
            "line budget enforced: max_lines={max_lines}, dropped={}",
            before_budget.saturating_sub(max_lines)
        ));
        by_priority
    } else {
        deduped
    };

    selected.sort_by_key(|candidate| candidate.order);
    let kept_lines = selected.len();
    let included = selected
        .iter()
        .map(|candidate| {
            format!(
                "kept {scope} signal [{} p{}]: {}",
                candidate.reason, candidate.priority, candidate.line
            )
        })
        .collect::<Vec<_>>();
    let output = selected
        .into_iter()
        .map(|candidate| candidate.line)
        .collect::<Vec<_>>()
        .join("\n");

    let input_non_empty = input.lines().filter(|line| !line.trim().is_empty()).count();
    let removed_as_noise = input_non_empty.saturating_sub(before_budget + duplicate_count);
    if removed_as_noise > 0 {
        excluded.push(format!("noise lines removed: {removed_as_noise}"));
    }

    PruneReport {
        original_lines,
        kept_lines,
        output,
        included: collapse_reasons(included),
        excluded,
    }
}

fn collapse_reasons(reasons: Vec<String>) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut order = Vec::new();

    for reason in reasons {
        let key = reason_key(&reason);
        if !counts.contains_key(&key) {
            order.push(key.clone());
        }
        *counts.entry(key).or_insert(0) += 1;
    }

    order
        .into_iter()
        .map(|key| {
            let count = counts.get(&key).copied().unwrap_or(0);
            if count == 1 {
                key
            } else {
                format!("{key}: {count} lines")
            }
        })
        .collect()
}

fn reason_key(reason: &str) -> String {
    if let Some(start) = reason.find('[') {
        if let Some(end) = reason[start + 1..].find(']') {
            let prefix = reason[..start].trim();
            let parser = &reason[start + 1..start + 1 + end];
            return format!("{prefix} [{parser}]");
        }
    }

    reason.to_string()
}
