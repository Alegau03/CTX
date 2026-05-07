use std::path::Path;

use globset::{GlobBuilder, GlobMatcher};

enum PathRule {
    Exact(String),
    Contains(String),
    Glob(GlobMatcher),
}

pub struct SegmentMatcher {
    rules: Vec<PathRule>,
}

impl SegmentMatcher {
    pub fn new(patterns: &[String]) -> Self {
        Self {
            rules: patterns
                .iter()
                .filter_map(|pattern| build_rule(pattern, MatchMode::Exact))
                .collect(),
        }
    }

    pub fn matches_path(&self, path: &Path) -> bool {
        path.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .map(normalize_str)
                .map(|name| self.matches_component(&name))
                .unwrap_or(false)
        })
    }

    fn matches_component(&self, component: &str) -> bool {
        self.rules.iter().any(|rule| match rule {
            PathRule::Exact(expected) => component == expected,
            PathRule::Contains(expected) => component.contains(expected),
            PathRule::Glob(matcher) => matcher.is_match(component),
        })
    }
}

pub struct PathMatcher {
    rules: Vec<PathRule>,
}

impl PathMatcher {
    pub fn exact_or_glob(patterns: &[String]) -> Self {
        Self {
            rules: patterns
                .iter()
                .filter_map(|pattern| build_rule(pattern, MatchMode::Exact))
                .collect(),
        }
    }

    pub fn contains_or_glob(patterns: &[String]) -> Self {
        Self {
            rules: patterns
                .iter()
                .filter_map(|pattern| build_rule(pattern, MatchMode::Contains))
                .collect(),
        }
    }

    pub fn matches_path(&self, repo_root: Option<&Path>, path: &Path) -> bool {
        let normalized_path = normalize_path(repo_root, path);
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(normalize_str);
        self.rules.iter().any(|rule| match rule {
            PathRule::Exact(expected) => {
                file_name.as_deref() == Some(expected.as_str())
                    || normalized_path == *expected
                    || normalized_path.ends_with(&format!("/{expected}"))
            }
            PathRule::Contains(expected) => normalized_path.contains(expected),
            PathRule::Glob(matcher) => {
                matcher.is_match(&normalized_path)
                    || file_name
                        .as_deref()
                        .map(|name| matcher.is_match(name))
                        .unwrap_or(false)
            }
        })
    }
}

enum MatchMode {
    Exact,
    Contains,
}

fn build_rule(pattern: &str, mode: MatchMode) -> Option<PathRule> {
    let normalized = normalize_str(pattern);
    if normalized.is_empty() {
        return None;
    }
    if has_glob_meta(&normalized) {
        return GlobBuilder::new(&normalized)
            .case_insensitive(true)
            .literal_separator(false)
            .build()
            .ok()
            .map(|glob| PathRule::Glob(glob.compile_matcher()));
    }

    Some(match mode {
        MatchMode::Exact => PathRule::Exact(normalized),
        MatchMode::Contains => PathRule::Contains(normalized),
    })
}

fn has_glob_meta(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?') || pattern.contains('[') || pattern.contains('{')
}

fn normalize_path(repo_root: Option<&Path>, path: &Path) -> String {
    let target = repo_root
        .and_then(|root| path.strip_prefix(root).ok())
        .map(Path::to_path_buf)
        .or_else(|| {
            let root = repo_root?.canonicalize().ok()?;
            let path = path.canonicalize().ok()?;
            path.strip_prefix(&root).ok().map(Path::to_path_buf)
        })
        .unwrap_or_else(|| path.to_path_buf());
    normalize_str(&target.to_string_lossy())
}

fn normalize_str(value: impl AsRef<str>) -> String {
    value.as_ref().replace('\\', "/").to_ascii_lowercase()
}
