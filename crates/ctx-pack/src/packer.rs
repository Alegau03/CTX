use ctx_token::estimate_tokens;
use serde::{Deserialize, Serialize};

use crate::rewriter::{
    PackSection, Priority, compact_to_fit, rewrite_dependency, rewrite_diff, rewrite_doc,
    rewrite_memory, rewrite_query, rewrite_root_cause, rewrite_symbol, rewrite_test,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInput {
    pub query: String,
    pub error_root_cause: Option<String>,
    pub symbols: Vec<String>,
    pub tests: Vec<String>,
    pub recent_diff: Option<String>,
    pub dependencies: Vec<String>,
    pub task_memory: Vec<String>,
    pub failure_memory: Vec<String>,
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
    pub pack_path: Option<String>,
    pub compact_context: String,
}

impl PackResult {
    pub fn with_pack_path(mut self, path: impl Into<String>) -> Self {
        self.pack_path = Some(path.into());
        self
    }
}

pub fn build_pack(input: &PackInput) -> PackResult {
    let original_tokens = estimate_tokens(&original_blob(input));
    let budget = input.budget.max(1);
    let mut sections = materialize_sections(input);
    sections.sort_by_key(|section| (section.priority, section.label.clone()));

    let mut rendered = Vec::new();
    let mut included = Vec::new();
    let mut excluded = Vec::new();

    for section in sections {
        let used = estimate_tokens(&rendered.join("\n"));
        let remaining = budget.saturating_sub(used);
        let Some(fitted) = compact_to_fit(&section, remaining) else {
            excluded.push(format!(
                "{} excluded: priority={:?} reason=token_budget remaining={} needed={} ref={}",
                section.label,
                section.priority,
                remaining,
                section.tokens(),
                section.source_ref
            ));
            continue;
        };

        let fitted_text = fitted.rendered();
        rendered.push(fitted_text);
        included.push(format!(
            "{} included: priority={:?} tokens={} ref={}",
            fitted.label,
            fitted.priority,
            fitted.tokens(),
            fitted.source_ref
        ));
    }

    let compact_context = rendered.join("\n");
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
        pack_path: None,
        compact_context,
    }
}

fn materialize_sections(input: &PackInput) -> Vec<PackSection> {
    let mut sections = Vec::new();
    sections.push(rewrite_query(&input.query));

    if let Some(root_cause) = &input.error_root_cause {
        if !root_cause.trim().is_empty() {
            sections.push(rewrite_root_cause(root_cause));
        }
    }

    sections.extend(input.symbols.iter().map(|value| rewrite_symbol(value)));
    sections.extend(input.tests.iter().map(|value| rewrite_test(value)));

    if let Some(diff) = &input.recent_diff {
        if !diff.trim().is_empty() {
            sections.push(rewrite_diff(diff));
        }
    }

    sections.extend(
        input
            .dependencies
            .iter()
            .map(|value| rewrite_dependency(value)),
    );
    sections.extend(
        input
            .task_memory
            .iter()
            .map(|value| rewrite_memory("task_memory", Priority::TaskMemory, value)),
    );
    sections.extend(
        input
            .failure_memory
            .iter()
            .map(|value| rewrite_memory("failure_memory", Priority::FailureMemory, value)),
    );
    sections.extend(
        input
            .memory
            .iter()
            .map(|value| rewrite_memory("memory", Priority::DirectiveMemory, value)),
    );
    sections.extend(input.docs.iter().map(|value| rewrite_doc(value)));
    sections
}

fn original_blob(input: &PackInput) -> String {
    [
        input.query.clone(),
        input.error_root_cause.clone().unwrap_or_default(),
        input.symbols.join("\n"),
        input.tests.join("\n"),
        input.recent_diff.clone().unwrap_or_default(),
        input.dependencies.join("\n"),
        input.task_memory.join("\n"),
        input.failure_memory.join("\n"),
        input.memory.join("\n"),
        input.docs.join("\n"),
    ]
    .join("\n")
}
