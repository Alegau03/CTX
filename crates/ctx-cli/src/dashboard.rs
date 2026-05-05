use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use ctx_core::run_gain;
use serde_json::{Value, json};

pub fn build_dashboard_value(repo_root: &Path) -> Result<Value> {
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
        .to_string();
    let gain = serde_json::to_value(run_gain(repo_root, 20)?).context("serialize gain report")?;
    let index_cache = read_json_if_exists(&repo_root.join(".ctx/cache/index-report.json"))?;
    let read_cache = read_json_if_exists(&repo_root.join(".ctx/cache/read-session.json"))?;
    let latest_stats = read_json_if_exists(&repo_root.join(".ctx/stats/latest.json"))?;
    let recent_audit = read_recent_audit(repo_root, 5)?;

    let savings = summarize_savings(&gain);
    let index_summary = summarize_index_cache(index_cache.as_ref());
    let read_summary = summarize_read_cache(read_cache.as_ref());
    let latest_activity = summarize_latest_activity(latest_stats.as_ref(), &recent_audit);
    let top_wins = summarize_top_wins(&gain, &index_summary, &read_summary);
    let warnings = build_warnings(&gain, index_cache.as_ref(), read_cache.as_ref());

    Ok(json!({
        "repo": repo_name,
        "savings": savings,
        "cache": {
            "index": index_summary,
            "read": read_summary
        },
        "latest_activity": latest_activity,
        "top_wins": top_wins,
        "latest_stats": latest_stats.unwrap_or_else(|| json!({})),
        "recent_audit": recent_audit,
        "warnings": warnings,
    }))
}

pub fn render_dashboard(repo_root: &Path) -> Result<String> {
    let value = build_dashboard_value(repo_root)?;
    let savings = &value["savings"];
    let index = &value["cache"]["index"];
    let read = &value["cache"]["read"];
    let latest_activity = &value["latest_activity"];
    let top_wins = &value["top_wins"];
    let warnings = value["warnings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| item.as_str().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    let recent_audit = value["recent_audit"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| item.as_str().map(ToOwned::to_owned))
        .collect::<Vec<_>>();

    let mut out = vec![
        "## 📊 CTX Dashboard".to_string(),
        format!("_repo: {}_", value["repo"].as_str().unwrap_or("repo")),
        String::new(),
        "**Savings**".to_string(),
        markdown_table(
            &["Metric", "Value"],
            &[
                vec![
                    "Sampled runs".to_string(),
                    savings["sampled_runs"].as_u64().unwrap_or(0).to_string(),
                ],
                vec![
                    "Total saved".to_string(),
                    format_tokens(savings["estimated_tokens_saved"].as_u64().unwrap_or(0)),
                ],
                vec![
                    "Avg saved / run".to_string(),
                    format_tokens(
                        savings["average_tokens_saved_per_run"]
                            .as_u64()
                            .unwrap_or(0),
                    ),
                ],
                vec![
                    "Avg reduction".to_string(),
                    format_pct(savings["average_reduction_pct"].as_f64()),
                ],
                vec![
                    "Latest reduction".to_string(),
                    format_pct(savings["latest_reduction_pct"].as_f64()),
                ],
                vec![
                    "Max reduction".to_string(),
                    format_pct(savings["max_reduction_pct"].as_f64()),
                ],
            ],
        ),
        String::new(),
        "**Cache**".to_string(),
        markdown_table(
            &["Metric", "Value"],
            &[
                vec![
                    "Read hit rate".to_string(),
                    format_pct(read["hit_rate_pct"].as_f64()),
                ],
                vec![
                    "Read cache hits".to_string(),
                    read["cache_hits"].as_u64().unwrap_or(0).to_string(),
                ],
                vec![
                    "Read cache misses".to_string(),
                    read["cache_misses"].as_u64().unwrap_or(0).to_string(),
                ],
                vec![
                    "Tracked files".to_string(),
                    read["tracked_files"].as_u64().unwrap_or(0).to_string(),
                ],
                vec![
                    "Index reuse".to_string(),
                    format_pct(index["reuse_ratio_pct"].as_f64()),
                ],
                vec![
                    "Indexed files".to_string(),
                    index["indexed_files"].as_u64().unwrap_or(0).to_string(),
                ],
                vec![
                    "Reused files".to_string(),
                    index["reused_files"].as_u64().unwrap_or(0).to_string(),
                ],
            ],
        ),
        String::new(),
        "**Latest Activity**".to_string(),
    ];

    let mut has_latest_activity = false;
    if let Some(query) = latest_activity["latest_query"].as_str() {
        if !query.is_empty() {
            has_latest_activity = true;
            out.push(format!("- Query: `{query}`"));
        }
    }
    if let Some(command) = latest_activity["latest_command"].as_str() {
        if !command.is_empty() {
            has_latest_activity = true;
            out.push(format!("- Command: `{command}`"));
        }
    }
    if let Some(pack_path) = latest_activity["latest_pack_path"].as_str() {
        if !pack_path.is_empty() {
            has_latest_activity = true;
            out.push(format!("- Pack: `{pack_path}`"));
        }
    }
    if !has_latest_activity {
        out.push("- Nothing recorded yet.".to_string());
    }

    out.push(String::new());
    out.push("**Top Win**".to_string());
    if let Some(best_query) = top_wins["best_query"]["query"].as_str() {
        out.push(format!("- Query: `{best_query}`"));
        out.push(format!(
            "- Runs: {}",
            top_wins["best_query"]["runs"].as_u64().unwrap_or(0)
        ));
        out.push(format!(
            "- Saved: {}",
            format_tokens(
                top_wins["best_query"]["estimated_tokens_saved"]
                    .as_u64()
                    .unwrap_or(0)
            )
        ));
        out.push(format!(
            "- Avg reduction: {}",
            format_pct(top_wins["best_query"]["average_reduction_pct"].as_f64())
        ));
    } else {
        out.push("- No query wins recorded yet.".to_string());
    }
    out.push(format!(
        "- Cache blend: read {} | index {}",
        format_pct(top_wins["read_hit_rate_pct"].as_f64()),
        format_pct(top_wins["index_reuse_ratio_pct"].as_f64()),
    ));

    out.push(String::new());
    out.push("**Warnings**".to_string());
    if warnings.is_empty() {
        out.push("- None.".to_string());
    } else {
        out.extend(warnings.into_iter().map(|warning| format!("- ⚠ {warning}")));
    }

    out.push(String::new());
    out.push("**Recent Audit**".to_string());
    if recent_audit.is_empty() {
        out.push("- No audit events yet.".to_string());
    } else {
        out.extend(recent_audit.into_iter().map(|line| format!("- {line}")));
    }

    Ok(out.join("\n"))
}

fn read_json_if_exists(path: &Path) -> Result<Option<Value>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(value))
}

fn summarize_savings(gain: &Value) -> Value {
    let total_original_tokens = gain
        .get("total_original_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_packed_tokens = gain
        .get("total_packed_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let estimated_tokens_saved = gain
        .get("estimated_tokens_saved")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let savings_ratio_pct = if total_original_tokens == 0 {
        0.0
    } else {
        (estimated_tokens_saved as f64 / total_original_tokens as f64) * 100.0
    };

    json!({
        "sampled_runs": gain.get("sampled_runs").and_then(Value::as_u64).unwrap_or(0),
        "estimated_tokens_saved": estimated_tokens_saved,
        "average_tokens_saved_per_run": if history_run_count(gain) == 0 {
            0
        } else {
            estimated_tokens_saved / history_run_count(gain)
        },
        "latest_reduction_pct": gain.get("latest_reduction_pct").cloned().unwrap_or(Value::Null),
        "average_reduction_pct": gain.get("average_reduction_pct").cloned().unwrap_or(Value::from(0.0)),
        "max_reduction_pct": gain.get("max_reduction_pct").cloned().unwrap_or(Value::from(0.0)),
        "total_original_tokens": total_original_tokens,
        "total_packed_tokens": total_packed_tokens,
        "savings_ratio_pct": savings_ratio_pct,
        "latest_pack_path": gain.get("latest_pack_path").cloned().unwrap_or(Value::Null),
        "top_queries": gain.get("top_queries").cloned().unwrap_or_else(|| json!([]))
    })
}

fn history_run_count(gain: &Value) -> u64 {
    gain.get("sampled_runs")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn summarize_index_cache(index_cache: Option<&Value>) -> Value {
    let Some(index_cache) = index_cache else {
        return json!({
            "scanned_files": 0,
            "indexed_files": 0,
            "reused_files": 0,
            "changed_files": 0,
            "new_files": 0,
            "reuse_ratio_pct": 0.0
        });
    };

    let scanned_files = index_cache
        .get("scanned_files")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let reused_files = index_cache
        .get("reused_files")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let reuse_ratio_pct = if scanned_files == 0 {
        0.0
    } else {
        (reused_files as f64 / scanned_files as f64) * 100.0
    };

    json!({
        "scanned_files": scanned_files,
        "indexed_files": index_cache.get("indexed_files").and_then(Value::as_u64).unwrap_or(0),
        "reused_files": reused_files,
        "changed_files": index_cache.get("changed_files").and_then(Value::as_u64).unwrap_or(0),
        "new_files": index_cache.get("new_files").and_then(Value::as_u64).unwrap_or(0),
        "reuse_ratio_pct": reuse_ratio_pct
    })
}

fn summarize_read_cache(read_cache: Option<&Value>) -> Value {
    let Some(read_cache) = read_cache else {
        return json!({
            "tracked_files": 0,
            "cache_hits": 0,
            "cache_misses": 0,
            "total_reads": 0,
            "hit_rate_pct": 0.0
        });
    };

    let tracked_files = read_cache
        .get("files")
        .and_then(Value::as_object)
        .map(|items| items.len() as u64)
        .unwrap_or(0);
    let cache_hits = read_cache
        .get("cache_hits")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let cache_misses = read_cache
        .get("cache_misses")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_reads = cache_hits + cache_misses;
    let hit_rate_pct = if total_reads == 0 {
        0.0
    } else {
        (cache_hits as f64 / total_reads as f64) * 100.0
    };

    json!({
        "tracked_files": tracked_files,
        "cache_hits": cache_hits,
        "cache_misses": cache_misses,
        "total_reads": total_reads,
        "hit_rate_pct": hit_rate_pct
    })
}

fn summarize_latest_activity(latest_stats: Option<&Value>, recent_audit: &[String]) -> Value {
    let latest_query = latest_stats
        .and_then(|stats| stats.get("query"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let latest_pack_path = latest_stats
        .and_then(|stats| stats.get("pack_path"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let latest_command = recent_audit.iter().rev().find_map(|line| {
        if !line.starts_with("run_command ") {
            return None;
        }
        extract_audit_field(line, "command")
    });

    json!({
        "latest_query": latest_query,
        "latest_pack_path": latest_pack_path,
        "latest_command": latest_command
    })
}

fn summarize_top_wins(gain: &Value, index_summary: &Value, read_summary: &Value) -> Value {
    let best_query = gain
        .get("top_queries")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .cloned()
        .unwrap_or(Value::Null);

    json!({
        "best_query": best_query,
        "index_reuse_ratio_pct": index_summary.get("reuse_ratio_pct").and_then(Value::as_f64).unwrap_or(0.0),
        "read_hit_rate_pct": read_summary.get("hit_rate_pct").and_then(Value::as_f64).unwrap_or(0.0)
    })
}

fn build_warnings(
    gain: &Value,
    index_cache: Option<&Value>,
    read_cache: Option<&Value>,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let sampled_runs = gain
        .get("sampled_runs")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if sampled_runs == 0 {
        warnings.push("no pack stats recorded yet".to_string());
    }
    if sampled_runs > 0 && sampled_runs < 3 {
        warnings.push("dashboard insights are based on fewer than 3 pack runs".to_string());
    }

    let average_reduction = gain
        .get("average_reduction_pct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if sampled_runs >= 3 && average_reduction < 20.0 {
        warnings.push("recent token savings are below 20%".to_string());
    }

    let latest_reduction = gain
        .get("latest_reduction_pct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if sampled_runs > 0 && latest_reduction < 0.0 {
        warnings.push("the latest pack was larger than the broad-context estimate".to_string());
    }

    let index_summary = summarize_index_cache(index_cache);
    let scanned = index_summary["scanned_files"].as_u64().unwrap_or(0);
    let reuse_ratio_pct = index_summary["reuse_ratio_pct"].as_f64().unwrap_or(0.0);
    if scanned >= 4 && reuse_ratio_pct < 25.0 {
        warnings.push("index cache reuse is low on the latest run".to_string());
    }

    let read_summary = summarize_read_cache(read_cache);
    if read_summary["tracked_files"].as_u64().unwrap_or(0) == 0 {
        warnings.push("no read cache entries recorded yet".to_string());
    }
    let total_reads = read_summary["total_reads"].as_u64().unwrap_or(0);
    let hit_rate_pct = read_summary["hit_rate_pct"].as_f64().unwrap_or(0.0);
    if total_reads >= 3 && hit_rate_pct < 40.0 {
        warnings.push("read cache hit rate is still low".to_string());
    }

    warnings
}

fn read_recent_audit(repo_root: &Path, limit: usize) -> Result<Vec<String>> {
    let path = repo_root.join(".ctx/audit.log");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut lines = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    Ok(lines)
}

fn extract_audit_field(line: &str, field: &str) -> Option<String> {
    let needle = format!("{field}=\"");
    let start = line.find(&needle)?;
    let rest = &line[start + needle.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].replace("\\\"", "\""))
}

fn format_tokens(value: u64) -> String {
    format!("{value} tokens")
}

fn format_pct(value: Option<f64>) -> String {
    format!("{:.2}%", value.unwrap_or(0.0))
}

fn markdown_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = Vec::new();
    out.push(format!("| {} |", headers.join(" | ")));
    out.push(format!(
        "| {} |",
        headers
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    for row in rows {
        out.push(format!("| {} |", row.join(" | ")));
    }
    out.join("\n")
}
