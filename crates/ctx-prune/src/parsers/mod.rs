mod diff;
mod logs;

pub(crate) use diff::prune_git_diff;
pub(crate) use logs::parse_log_candidates;
