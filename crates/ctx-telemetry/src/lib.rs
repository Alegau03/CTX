pub mod audit;
pub mod gain;
pub mod stats;

use serde::{Deserialize, Serialize};

pub use audit::{
    AuditEvent, PrivacyAuditEvent, append_audit_event, append_audit_line,
    append_privacy_audit_event,
};
pub use gain::{GainQuerySummary, GainReport, build_gain_report};
pub use stats::{StatsSnapshot, read_latest_stats, read_stats_history, write_latest_stats};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkCaseResult {
    pub case_name: String,
    pub original_tokens: usize,
    pub packed_tokens: usize,
    pub latency_ms: u64,
    pub baseline_success: bool,
    pub ctx_success: bool,
    pub retrieval_precision_at_k: f64,
    pub answer_quality_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub token_reduction_pct: f64,
    pub avg_latency_ms: f64,
    pub baseline_success_rate: f64,
    pub ctx_success_rate: f64,
    pub avg_answer_quality: f64,
    pub avg_retrieval_precision_at_k: f64,
    pub case_count: usize,
}

pub fn build_benchmark_summary(cases: &[BenchmarkCaseResult]) -> BenchmarkSummary {
    if cases.is_empty() {
        return BenchmarkSummary {
            token_reduction_pct: 0.0,
            avg_latency_ms: 0.0,
            baseline_success_rate: 0.0,
            ctx_success_rate: 0.0,
            avg_answer_quality: 0.0,
            avg_retrieval_precision_at_k: 0.0,
            case_count: 0,
        };
    }

    let original_total = cases.iter().map(|c| c.original_tokens as f64).sum::<f64>();
    let packed_total = cases.iter().map(|c| c.packed_tokens as f64).sum::<f64>();
    let baseline_success = cases.iter().filter(|c| c.baseline_success).count() as f64;
    let ctx_success = cases.iter().filter(|c| c.ctx_success).count() as f64;
    let latency_sum = cases.iter().map(|c| c.latency_ms as f64).sum::<f64>();
    let quality_sum = cases.iter().map(|c| c.answer_quality_score).sum::<f64>();
    let precision_sum = cases
        .iter()
        .map(|c| c.retrieval_precision_at_k)
        .sum::<f64>();
    let count = cases.len() as f64;

    let token_reduction_pct = if original_total <= 0.0 {
        0.0
    } else {
        (1.0 - packed_total / original_total) * 100.0
    };

    BenchmarkSummary {
        token_reduction_pct,
        avg_latency_ms: latency_sum / count,
        baseline_success_rate: (baseline_success / count) * 100.0,
        ctx_success_rate: (ctx_success / count) * 100.0,
        avg_answer_quality: quality_sum / count,
        avg_retrieval_precision_at_k: precision_sum / count,
        case_count: cases.len(),
    }
}

pub fn render_benchmark_markdown(summary: &BenchmarkSummary) -> String {
    format!(
        concat!(
            "# Benchmark Summary\n\n",
            "- Cases: {case_count}\n",
            "- Token reduction %: {token_reduction_pct:.2}\n",
            "- Latency overhead (avg ms): {avg_latency_ms:.2}\n",
            "- Task success rate vs baseline: {ctx_success_rate:.2}% vs {baseline_success_rate:.2}%\n",
            "- Answer quality (avg): {avg_answer_quality:.2}\n",
            "- Retrieval precision@k (avg): {avg_retrieval_precision_at_k:.3}\n"
        ),
        case_count = summary.case_count,
        token_reduction_pct = summary.token_reduction_pct,
        avg_latency_ms = summary.avg_latency_ms,
        ctx_success_rate = summary.ctx_success_rate,
        baseline_success_rate = summary.baseline_success_rate,
        avg_answer_quality = summary.avg_answer_quality,
        avg_retrieval_precision_at_k = summary.avg_retrieval_precision_at_k
    )
}
