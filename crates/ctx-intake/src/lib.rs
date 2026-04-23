use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    Debug,
    Refactor,
    Review,
    Explain,
    Ask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signals {
    pub recent_files: Vec<String>,
    pub command_output: Option<String>,
    pub git_status: bool,
}

impl Default for Signals {
    fn default() -> Self {
        Self {
            recent_files: Vec::new(),
            command_output: None,
            git_status: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryIntake {
    pub task: String,
    pub intent: Intent,
    pub repo_root: String,
    pub signals: Signals,
}

impl QueryIntake {
    pub fn new(task: &str, repo_root: &str) -> Self {
        Self {
            task: task.to_string(),
            intent: detect_intent(task),
            repo_root: repo_root.to_string(),
            signals: Signals::default(),
        }
    }
}

pub fn detect_intent(task: &str) -> Intent {
    let lower = task.to_lowercase();

    if ["fix", "failing", "error", "bug", "traceback", "flaky"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        return Intent::Debug;
    }

    if ["refactor", "separate", "cleanup", "modularize"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        return Intent::Refactor;
    }

    if ["review", "risky", "audit", "security"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        return Intent::Review;
    }

    if ["explain", "where is", "why is", "map"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        return Intent::Explain;
    }

    Intent::Ask
}
