use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneReport {
    pub original_lines: usize,
    pub kept_lines: usize,
    pub output: String,
    pub included: Vec<String>,
    pub excluded: Vec<String>,
}

pub fn prune_logs(input: &str, max_lines: usize) -> PruneReport {
    let mut seen = HashSet::new();
    let mut kept = Vec::new();
    let mut excluded = Vec::new();
    let mut included = Vec::new();

    let keep_patterns = [
        Regex::new(r"(?i)\berror\b").expect("valid regex"),
        Regex::new(r"(?i)\bfail(ed|ure)?\b").expect("valid regex"),
        Regex::new(r"(?i)traceback").expect("valid regex"),
        Regex::new(r"(?i)exception").expect("valid regex"),
        Regex::new(r"(?i)warning").expect("valid regex"),
    ];

    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if !seen.insert(line.to_string()) {
            excluded.push(format!("duplicate line removed: {line}"));
            continue;
        }

        let matched = keep_patterns.iter().any(|rx| rx.is_match(line));
        if matched {
            included.push(format!("kept diagnostic signal: {line}"));
            kept.push(line.to_string());
        } else {
            excluded.push(format!("noise line removed: {line}"));
        }
    }

    if kept.len() > max_lines {
        kept.truncate(max_lines);
        excluded.push(format!("line budget enforced: max_lines={max_lines}"));
    }

    PruneReport {
        original_lines: input.lines().count(),
        kept_lines: kept.len(),
        output: kept.join("\n"),
        included,
        excluded,
    }
}

pub fn prune_diff(input: &str, query: &str, max_lines: usize) -> PruneReport {
    let mut included = Vec::new();
    let mut excluded = Vec::new();
    let mut kept = Vec::new();

    let query_terms = tokenize_query(query);

    let mut current_block = Vec::new();
    let mut current_match = false;
    let mut any_match = false;

    let flush_block = |block: &mut Vec<String>,
                       matched: bool,
                       kept: &mut Vec<String>,
                       included: &mut Vec<String>,
                       excluded: &mut Vec<String>| {
        if block.is_empty() {
            return;
        }
        if matched {
            kept.extend(block.iter().cloned());
            included.push("kept diff block due to query match".to_string());
        } else {
            excluded.push("excluded diff block with no query overlap".to_string());
        }
        block.clear();
    };

    for raw_line in input.lines() {
        let line = raw_line.to_string();
        let starts_new_block = line.starts_with("diff --git ");

        if starts_new_block {
            flush_block(
                &mut current_block,
                current_match,
                &mut kept,
                &mut included,
                &mut excluded,
            );
            current_match = false;
        }

        if line.starts_with("diff --git") || line.starts_with("@@") {
            current_block.push(line);
            continue;
        }

        if line.starts_with('+') || line.starts_with('-') {
            if line.starts_with("+++") || line.starts_with("---") {
                current_block.push(line);
                continue;
            }

            if query_terms
                .iter()
                .any(|term| line.to_lowercase().contains(term))
            {
                current_match = true;
                any_match = true;
            }
        }

        current_block.push(line);
    }

    flush_block(
        &mut current_block,
        current_match || !any_match,
        &mut kept,
        &mut included,
        &mut excluded,
    );

    if kept.len() > max_lines {
        kept.truncate(max_lines);
        excluded.push(format!("line budget enforced: max_lines={max_lines}"));
    }

    PruneReport {
        original_lines: input.lines().count(),
        kept_lines: kept.len(),
        output: kept.join("\n"),
        included,
        excluded,
    }
}

fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split_whitespace()
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
