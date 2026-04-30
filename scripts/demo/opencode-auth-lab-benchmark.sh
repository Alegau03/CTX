#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOURCE_FIXTURE="$ROOT_DIR/demo/fixtures/opencode-auth-lab"
CTX_BIN="${1:-$ROOT_DIR/target/debug/ctx}"
if [[ "$CTX_BIN" != /* ]]; then
  CTX_BIN="$ROOT_DIR/$CTX_BIN"
fi

if [[ -n "${CTX_DEMO_FIXTURE:-}" ]]; then
  FIXTURE="$CTX_DEMO_FIXTURE"
else
  TMP_FIXTURE_ROOT="$(mktemp -d)"
  trap 'rm -rf "$TMP_FIXTURE_ROOT"' EXIT
  FIXTURE="$TMP_FIXTURE_ROOT/opencode-auth-lab"
  cp -R "$SOURCE_FIXTURE" "$FIXTURE"
fi

rm -rf "$FIXTURE/.ctx"
pushd "$FIXTURE" >/dev/null
"$CTX_BIN" --repo-root . init >/dev/null
"$CTX_BIN" --repo-root . memory import --from AGENTS.md --scope project --source markdown --prefix agents >/dev/null
"$CTX_BIN" --repo-root . benchmark memory-suite \
  --spec benchmarks/memory-suite.toml \
  --report-out benchmarks/report.md \
  --json-out benchmarks/report.json >/dev/null
popd >/dev/null

test -f "$FIXTURE/benchmarks/report.md"
test -f "$FIXTURE/benchmarks/report.json"
grep 'CTX Demo Memory Benchmark' "$FIXTURE/benchmarks/report.md" >/dev/null
grep 'case_count' "$FIXTURE/benchmarks/report.json" >/dev/null

echo "CTX demo benchmark passed: $FIXTURE"
