use ctx_token::estimate_tokens;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Query = 0,
    RootCause = 1,
    Symbol = 2,
    Test = 3,
    RecentDiff = 4,
    Dependency = 5,
    TaskMemory = 6,
    FailureMemory = 7,
    DirectiveMemory = 8,
    Docs = 9,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackSection {
    pub label: String,
    pub priority: Priority,
    pub content: String,
    pub source_ref: String,
    pub required: bool,
}

impl PackSection {
    pub fn rendered(&self) -> String {
        if self.source_ref.is_empty() || self.content.contains(&self.source_ref) {
            format!("{}: {}", self.label, self.content)
        } else {
            format!("{}: {} @{}", self.label, self.content, self.source_ref)
        }
    }

    pub fn tokens(&self) -> usize {
        estimate_tokens(&self.rendered())
    }
}

pub fn rewrite_query(query: &str) -> PackSection {
    PackSection {
        label: "query".to_string(),
        priority: Priority::Query,
        content: compact_words(query, 32),
        source_ref: String::new(),
        required: true,
    }
}

pub fn rewrite_root_cause(root_cause: &str) -> PackSection {
    PackSection {
        label: "root_cause".to_string(),
        priority: Priority::RootCause,
        content: compact_words(root_cause, 8),
        source_ref: String::new(),
        required: true,
    }
}

pub fn rewrite_symbol(raw: &str) -> PackSection {
    let trimmed = raw.trim();
    let (path, rest) = split_once_any(trimmed, &["::", ":"]).unwrap_or(("unknown", trimmed));
    let signature = extract_signature(rest);
    let imports = extract_significant_imports(trimmed);
    let line_ref = extract_line_ref(trimmed).unwrap_or_else(|| "unknown".to_string());
    let mut content = format!(
        "{} sig:{} lines:{} imports:{} relationships:{}",
        path.trim(),
        signature,
        line_ref,
        imports,
        "direct"
    );
    content = compact_words(&content, 10);

    PackSection {
        label: "symbols".to_string(),
        priority: Priority::Symbol,
        content,
        source_ref: path.trim().to_string(),
        required: false,
    }
}

pub fn rewrite_test(raw: &str) -> PackSection {
    PackSection {
        label: "tests".to_string(),
        priority: Priority::Test,
        content: compact_words(raw, 28),
        source_ref: String::new(),
        required: false,
    }
}

pub fn rewrite_diff(raw: &str) -> PackSection {
    let mut files = Vec::new();
    let mut additions = 0usize;
    let mut removals = 0usize;

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            files.push(rest.replace(" a/", " ").replace(" b/", " "));
        } else if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            removals += 1;
        }
    }

    if files.is_empty() {
        files.extend(raw.lines().filter_map(stat_file_from_line).take(6));
    }

    let file_summary = if files.is_empty() {
        "unknown".to_string()
    } else {
        compact_words(&files.join(","), 2)
    };

    PackSection {
        label: "recent_diff".to_string(),
        priority: Priority::RecentDiff,
        content: format!(
            "diff_files:{} changes:+{}/-{}",
            file_summary, additions, removals
        ),
        source_ref: String::new(),
        required: false,
    }
}

pub fn rewrite_dependency(raw: &str) -> PackSection {
    PackSection {
        label: "dependencies".to_string(),
        priority: Priority::Dependency,
        content: compact_dependency(raw),
        source_ref: String::new(),
        required: false,
    }
}

pub fn rewrite_memory(label: &str, priority: Priority, raw: &str) -> PackSection {
    PackSection {
        label: label.to_string(),
        priority,
        content: compact_words(strip_memory_prefix(raw), 10),
        source_ref: memory_source(raw),
        required: false,
    }
}

pub fn rewrite_doc(raw: &str) -> PackSection {
    PackSection {
        label: "docs".to_string(),
        priority: Priority::Docs,
        content: compact_words(raw, 16),
        source_ref: source_from_path_like(raw),
        required: false,
    }
}

pub fn compact_to_fit(section: &PackSection, max_tokens: usize) -> Option<PackSection> {
    if max_tokens == 0 {
        return None;
    }
    if section.tokens() <= max_tokens {
        return Some(section.clone());
    }

    let mut compacted = section.clone();
    for limit in [24usize, 16, 10, 6, 3, 2, 1] {
        compacted.content = compact_words(&section.content, limit);
        if compacted.tokens() <= max_tokens {
            return Some(compacted);
        }
    }
    None
}

fn compact_words(text: &str, max_words: usize) -> String {
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.len() <= max_words {
        return words.join(" ");
    }
    format!("{} ...", words[..max_words].join(" "))
}

fn compact_dependency(raw: &str) -> String {
    if raw.contains("->") {
        return raw
            .split("->")
            .map(str::trim)
            .take(2)
            .collect::<Vec<_>>()
            .join("->");
    }
    compact_words(raw, 8)
}

fn strip_memory_prefix(raw: &str) -> &str {
    raw.split_once(':')
        .map(|(_, rest)| rest.trim())
        .unwrap_or(raw.trim())
}

fn split_once_any<'a>(text: &'a str, needles: &[&str]) -> Option<(&'a str, &'a str)> {
    needles
        .iter()
        .filter_map(|needle| text.split_once(needle))
        .next()
}

fn extract_signature(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(fn_pos) = trimmed.find("fn ") {
        let sig = &trimmed[fn_pos..];
        if let Some(end) = sig.find(')') {
            let head = sig[..end]
                .split('(')
                .next()
                .unwrap_or(sig[..end].trim())
                .trim();
            return format!("{}(...)", head);
        }
        return sig.split('{').next().unwrap_or(sig).trim().to_string();
    }
    trimmed
        .split('{')
        .next()
        .unwrap_or(trimmed)
        .split_whitespace()
        .take(6)
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_significant_imports(text: &str) -> String {
    let imports = text
        .lines()
        .map(str::trim)
        .filter(|line| {
            line.starts_with("use ")
                || line.starts_with("import ")
                || line.starts_with("from ")
                || line.starts_with("require(")
        })
        .take(3)
        .collect::<Vec<_>>();
    if imports.is_empty() {
        "n/a".to_string()
    } else {
        compact_words(&imports.join(";"), 14)
    }
}

fn extract_line_ref(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        if token.contains(':') && token.chars().any(|ch| ch.is_ascii_digit()) {
            return Some(
                token
                    .trim_matches(|ch: char| ch == ',' || ch == ';')
                    .to_string(),
            );
        }
    }
    None
}

fn stat_file_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.contains('|') {
        return trimmed
            .split('|')
            .next()
            .map(|part| part.trim().to_string());
    }
    None
}

fn source_from_path_like(raw: &str) -> String {
    raw.split_whitespace()
        .find(|part| part.contains('/') || part.contains("::"))
        .unwrap_or("unknown")
        .trim_matches(|ch: char| ch == ',' || ch == ';')
        .to_string()
}

fn memory_source(raw: &str) -> String {
    raw.split(']')
        .next()
        .unwrap_or(raw)
        .trim_start_matches('[')
        .split(':')
        .next()
        .unwrap_or("memory")
        .to_string()
}
