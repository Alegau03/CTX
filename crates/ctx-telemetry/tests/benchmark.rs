use ctx_telemetry::{
    BenchmarkCaseResult, BenchmarkSummary, build_benchmark_summary, render_benchmark_markdown,
};

#[test]
fn benchmark_summary_computes_aggregate_metrics() {
    let cases = vec![
        BenchmarkCaseResult {
            case_name: "debug-auth".to_string(),
            original_tokens: 1000,
            packed_tokens: 200,
            latency_ms: 150,
            baseline_success: false,
            ctx_success: true,
            retrieval_precision_at_k: 0.8,
            answer_quality_score: 4.0,
        },
        BenchmarkCaseResult {
            case_name: "refactor-loader".to_string(),
            original_tokens: 800,
            packed_tokens: 400,
            latency_ms: 120,
            baseline_success: true,
            ctx_success: true,
            retrieval_precision_at_k: 0.6,
            answer_quality_score: 3.5,
        },
    ];

    let summary = build_benchmark_summary(&cases);

    assert!(summary.token_reduction_pct > 0.0);
    assert!(summary.ctx_success_rate >= summary.baseline_success_rate);
    assert!(summary.avg_latency_ms > 0.0);
}

#[test]
fn benchmark_markdown_contains_required_kpis() {
    let summary = BenchmarkSummary {
        token_reduction_pct: 65.0,
        avg_latency_ms: 140.0,
        baseline_success_rate: 50.0,
        ctx_success_rate: 100.0,
        avg_answer_quality: 4.2,
        avg_retrieval_precision_at_k: 0.72,
        case_count: 4,
    };

    let md = render_benchmark_markdown(&summary);
    assert!(md.contains("Token reduction %"));
    assert!(md.contains("Latency overhead"));
    assert!(md.contains("Task success rate vs baseline"));
    assert!(md.contains("Retrieval precision@k"));
}
