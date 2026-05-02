use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{StatsSnapshot, read_stats_history};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GainQuerySummary {
    pub query: String,
    pub runs: usize,
    pub average_reduction_pct: f64,
    pub estimated_tokens_saved: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GainReport {
    pub sampled_runs: usize,
    pub latest_reduction_pct: Option<f64>,
    pub average_reduction_pct: f64,
    pub max_reduction_pct: f64,
    pub total_original_tokens: usize,
    pub total_packed_tokens: usize,
    pub estimated_tokens_saved: usize,
    pub latest_pack_path: Option<String>,
    pub top_queries: Vec<GainQuerySummary>,
}

pub fn build_gain_report(stats_dir: &Path, limit: usize) -> Result<GainReport> {
    let history = read_stats_history(stats_dir, limit)?;
    Ok(build_gain_report_from_history(&history))
}

fn build_gain_report_from_history(history: &[StatsSnapshot]) -> GainReport {
    if history.is_empty() {
        return GainReport {
            sampled_runs: 0,
            latest_reduction_pct: None,
            average_reduction_pct: 0.0,
            max_reduction_pct: 0.0,
            total_original_tokens: 0,
            total_packed_tokens: 0,
            estimated_tokens_saved: 0,
            latest_pack_path: None,
            top_queries: Vec::new(),
        };
    }

    let total_original_tokens = history
        .iter()
        .map(|item| item.original_tokens)
        .sum::<usize>();
    let total_packed_tokens = history.iter().map(|item| item.packed_tokens).sum::<usize>();
    let estimated_tokens_saved = total_original_tokens.saturating_sub(total_packed_tokens);
    let average_reduction_pct =
        history.iter().map(|item| item.reduction_pct).sum::<f64>() / history.len() as f64;
    let max_reduction_pct = history
        .iter()
        .map(|item| item.reduction_pct)
        .fold(0.0, f64::max);

    let mut query_groups: HashMap<String, (usize, f64, usize)> = HashMap::new();
    for snapshot in history {
        if let Some(query) = snapshot.query.as_deref() {
            let entry = query_groups.entry(query.to_string()).or_insert((0, 0.0, 0));
            entry.0 += 1;
            entry.1 += snapshot.reduction_pct;
            entry.2 += snapshot
                .original_tokens
                .saturating_sub(snapshot.packed_tokens);
        }
    }

    let mut top_queries = query_groups
        .into_iter()
        .map(
            |(query, (runs, reduction_sum, estimated_tokens_saved))| GainQuerySummary {
                query,
                runs,
                average_reduction_pct: reduction_sum / runs as f64,
                estimated_tokens_saved,
            },
        )
        .collect::<Vec<_>>();
    top_queries.sort_by(|left, right| {
        right
            .runs
            .cmp(&left.runs)
            .then(
                right
                    .estimated_tokens_saved
                    .cmp(&left.estimated_tokens_saved),
            )
            .then_with(|| left.query.cmp(&right.query))
    });
    top_queries.truncate(5);

    GainReport {
        sampled_runs: history.len(),
        latest_reduction_pct: history.first().map(|item| item.reduction_pct),
        average_reduction_pct,
        max_reduction_pct,
        total_original_tokens,
        total_packed_tokens,
        estimated_tokens_saved,
        latest_pack_path: history.first().and_then(|item| item.pack_path.clone()),
        top_queries,
    }
}
