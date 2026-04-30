#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOURCE_FIXTURE="$ROOT_DIR/demo/fixtures/opencode-auth-lab"
CTX_BIN="${1:-$ROOT_DIR/target/debug/ctx}"

if [[ -n "${CTX_DEMO_FIXTURE:-}" ]]; then
  FIXTURE="$CTX_DEMO_FIXTURE"
else
  TMP_FIXTURE_ROOT="$(mktemp -d)"
  trap 'rm -rf "$TMP_FIXTURE_ROOT"' EXIT
  FIXTURE="$TMP_FIXTURE_ROOT/opencode-auth-lab"
  cp -R "$SOURCE_FIXTURE" "$FIXTURE"
fi

rm -rf "$FIXTURE/.ctx" "$FIXTURE/.opencode" "$FIXTURE/opencode.json"

"$CTX_BIN" --repo-root "$FIXTURE" init >/dev/null
"$CTX_BIN" --repo-root "$FIXTURE" index >/dev/null
"$CTX_BIN" --repo-root "$FIXTURE" opencode install >/dev/null

DOCTOR_OUTPUT="$("$CTX_BIN" --repo-root "$FIXTURE" doctor)"
printf '%s\n' "$DOCTOR_OUTPUT" | grep 'config: ok' >/dev/null
printf '%s\n' "$DOCTOR_OUTPUT" | grep 'graph: ok' >/dev/null
printf '%s\n' "$DOCTOR_OUTPUT" | grep 'local_only: true' >/dev/null

BOOTSTRAP_OUTPUT="$("$CTX_BIN" --repo-root "$FIXTURE" memory bootstrap)"
printf '%s\n' "$BOOTSTRAP_OUTPUT" | grep 'imported_files=' >/dev/null

SEARCH_OUTPUT="$("$CTX_BIN" --repo-root "$FIXTURE" memory search 'auth root cause' --scope project --limit 10)"
printf '%s\n' "$SEARCH_OUTPUT" | grep 'root cause' >/dev/null

RETRIEVE_OUTPUT="$("$CTX_BIN" --repo-root "$FIXTURE" retrieve 'handle refresh route' --limit 8)"
printf '%s\n' "$RETRIEVE_OUTPUT" | grep 'src/http/refresh-route.ts::handleRefreshRoute' >/dev/null

GRAPH_OUTPUT="$("$CTX_BIN" --repo-root "$FIXTURE" graph query refresh)"
printf '%s\n' "$GRAPH_OUTPUT" | grep 'refresh' >/dev/null

PRUNE_OUTPUT="$(cat "$FIXTURE/logs/vitest-refresh-failure.log" | "$CTX_BIN" --repo-root "$FIXTURE" prune logs)"
printf '%s\n' "$PRUNE_OUTPUT" | grep 'AssertionError' >/dev/null

DIFF_OUTPUT="$(cat "$FIXTURE/diff/refresh-route.patch" | "$CTX_BIN" --repo-root "$FIXTURE" prune diff --query 'refresh token rotation')"
printf '%s\n' "$DIFF_OUTPUT" | grep 'rotated:' >/dev/null

PACK_OUTPUT="$("$CTX_BIN" --repo-root "$FIXTURE" pack 'fix refresh token rotation' --attach "$FIXTURE/logs/vitest-refresh-failure.log" --json)"
printf '%s\n' "$PACK_OUTPUT" | grep 'packed_tokens' >/dev/null
printf '%s\n' "$PACK_OUTPUT" | grep 'compact_context' >/dev/null

echo "CTX demo smoke passed: $FIXTURE"
