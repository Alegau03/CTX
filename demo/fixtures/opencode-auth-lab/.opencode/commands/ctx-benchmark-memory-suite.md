---
description: Benchmark | Run a reusable CTX memory benchmark suite
---

Run the CTX memory benchmark suite in the current repository.

Arguments:
- `$1`: required spec path
- `$2`: optional markdown report path, default `benchmark-report.md`
- `$3`: optional JSON report path

Run:
- `ctx benchmark memory-suite --spec <spec> --report-out <report>`
- include `--json-out <json>` when structured output is also needed

Then summarize the suite KPIs and point to the generated report files.
