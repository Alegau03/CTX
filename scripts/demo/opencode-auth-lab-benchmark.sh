#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURE="${CTX_DEMO_FIXTURE:-$ROOT_DIR/demo/fixtures/opencode-auth-lab}"
CTX_BIN="${1:-$ROOT_DIR/target/debug/ctx}"

rm -rf "$FIXTURE/.ctx"
"$CTX_BIN" --repo-root "$FIXTURE" init >/dev/null
"$CTX_BIN" --repo-root "$FIXTURE" memory import --from "$FIXTURE/AGENTS.md" --scope project --source markdown --prefix agents >/dev/null
"$CTX_BIN" --repo-root "$FIXTURE" benchmark memory-suite \
  --spec "$FIXTURE/benchmarks/memory-suite.toml" \
  --report-out "$FIXTURE/benchmarks/report.md" \
  --json-out "$FIXTURE/benchmarks/report.json" >/dev/null

test -f "$FIXTURE/benchmarks/report.md"
test -f "$FIXTURE/benchmarks/report.json"
grep 'CTX Demo Memory Benchmark' "$FIXTURE/benchmarks/report.md" >/dev/null
grep 'case_count' "$FIXTURE/benchmarks/report.json" >/dev/null

echo "CTX demo benchmark passed: $FIXTURE"
