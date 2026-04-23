use ctx_token::estimate_tokens;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInput {
    pub query: String,
    pub error_root_cause: Option<String>,
    pub symbols: Vec<String>,
    pub tests: Vec<String>,
    pub recent_diff: Option<String>,
    pub dependencies: Vec<String>,
    pub memory: Vec<String>,
    pub docs: Vec<String>,
    pub budget: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackResult {
    pub original_estimated_tokens: usize,
    pub packed_tokens: usize,
    pub reduction_pct: f64,
    pub included: Vec<String>,
    pub excluded: Vec<String>,
    pub compact_context: String,
}

pub fn build_pack(input: &PackInput) -> PackResult {
    let mut included = Vec::new();
    let mut excluded = Vec::new();
    let mut sections = Vec::new();

    let original_blob = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        input.query,
        input.error_root_cause.clone().unwrap_or_default(),
        input.symbols.join("\n"),
        input.tests.join("\n"),
        input.recent_diff.clone().unwrap_or_default(),
        input.dependencies.join("\n"),
        input.memory.join("\n"),
        input.docs.join("\n")
    );

    let original_tokens = estimate_tokens(&original_blob);
    let budget = input.budget.max(1);

    let query_section = truncate_to_budget(&format!("query: {}", input.query), budget);
    sections.push(query_section);
    included.push("query".to_string());

    if let Some(root_cause) = &input.error_root_cause {
        let remaining = budget
            .saturating_sub(estimate_tokens(&sections.join("\n")))
            .max(1);
        let root_cause_section =
            truncate_to_budget(&format!("root_cause: {root_cause}"), remaining);
        sections.push(root_cause_section);
        included.push("root_cause".to_string());
    }

    push_many_if_budget(
        &mut sections,
        &mut included,
        &mut excluded,
        budget,
        "symbols",
        &input.symbols,
    );
    push_many_if_budget(
        &mut sections,
        &mut included,
        &mut excluded,
        budget,
        "tests",
        &input.tests,
    );

    if let Some(diff) = &input.recent_diff {
        push_if_budget(
            &mut sections,
            &mut included,
            &mut excluded,
            budget,
            "recent_diff",
            diff,
        );
    }

    push_many_if_budget(
        &mut sections,
        &mut included,
        &mut excluded,
        budget,
        "dependencies",
        &input.dependencies,
    );
    push_many_if_budget(
        &mut sections,
        &mut included,
        &mut excluded,
        budget,
        "memory",
        &input.memory,
    );
    push_many_if_budget(
        &mut sections,
        &mut included,
        &mut excluded,
        budget,
        "docs",
        &input.docs,
    );

    let compact_context = sections.join("\n");
    let packed_tokens = estimate_tokens(&compact_context).min(budget);

    let reduction_pct = if original_tokens == 0 {
        0.0
    } else {
        (1.0 - (packed_tokens as f64 / original_tokens as f64)) * 100.0
    };

    PackResult {
        original_estimated_tokens: original_tokens,
        packed_tokens,
        reduction_pct,
        included,
        excluded,
        compact_context,
    }
}

fn push_if_budget(
    sections: &mut Vec<String>,
    included: &mut Vec<String>,
    excluded: &mut Vec<String>,
    budget: usize,
    label: &str,
    value: &str,
) {
    let candidate = format!("{label}: {value}");
    let current = sections.join("\n");
    let current_tokens = estimate_tokens(&current);
    let candidate_tokens = estimate_tokens(&candidate);

    if current_tokens + candidate_tokens <= budget {
        sections.push(candidate);
        included.push(label.to_string());
    } else {
        excluded.push(format!("{label} excluded due to token budget"));
    }
}

fn truncate_to_budget(text: &str, budget: usize) -> String {
    if estimate_tokens(text) <= budget {
        return text.to_string();
    }

    let mut words = Vec::new();
    for word in text.split_whitespace() {
        words.push(word);
        let candidate = words.join(" ");
        if estimate_tokens(&candidate) >= budget {
            return candidate;
        }
    }
    text.to_string()
}

fn push_many_if_budget(
    sections: &mut Vec<String>,
    included: &mut Vec<String>,
    excluded: &mut Vec<String>,
    budget: usize,
    label: &str,
    values: &[String],
) {
    for value in values {
        let candidate_label = format!("{label}:{value}");
        push_if_budget(sections, included, excluded, budget, label, value);
        if !included.iter().any(|entry| entry == label) {
            excluded.push(format!("{candidate_label} excluded due to token budget"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_query_when_budget_tight() {
        let input = PackInput {
            query: "fix flaky login test".to_string(),
            error_root_cause: None,
            symbols: vec!["src/auth.rs::validate".to_string()],
            tests: vec![],
            recent_diff: None,
            dependencies: vec![],
            memory: vec![],
            docs: vec![],
            budget: 4,
        };

        let result = build_pack(&input);
        assert!(result.compact_context.contains("query:"));
    }
}
