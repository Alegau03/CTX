---
description: Benchmark | Run a reusable CTX memory benchmark suite
---

Run the CTX memory benchmark suite in the current repository.

Arguments:
- `$1`: required spec path
- `$2`: optional markdown report path, default `benchmark-report.md`
- `$3`: optional JSON report path

Run:
- `'/Users/alessandrogautieri/Documents/coding/ctx/target/debug/ctx' --repo-root 'demo/fixtures/opencode-auth-lab' benchmark memory-suite --spec <spec> --report-out <report>`
- include `--json-out <json>` when structured output is also needed

Rules:
- run only the exact CTX benchmark command
- do not infer KPIs from source files manually

Then summarize the suite KPIs and point to the generated report files.
