#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURE="${CTX_DEMO_FIXTURE:-$ROOT_DIR/demo/fixtures/opencode-auth-lab}"
CTX_BIN="${1:-$ROOT_DIR/target/debug/ctx}"

rm -rf "$FIXTURE/.ctx"
"$CTX_BIN" --repo-root "$FIXTURE" init >/dev/null
"$CTX_BIN" --repo-root "$FIXTURE" index >/dev/null

BOOTSTRAP_RESPONSE="$(printf '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"memory_bootstrap_markdown","arguments":{}}}\n' | "$CTX_BIN" --repo-root "$FIXTURE" mcp stdio)"
printf '%s\n' "$BOOTSTRAP_RESPONSE" | grep 'imported_files' >/dev/null

SEARCH_RESPONSE="$(printf '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"memory_search","arguments":{"query":"auth root cause","scope":"project","limit":10}}}\n' | "$CTX_BIN" --repo-root "$FIXTURE" mcp stdio)"
printf '%s\n' "$SEARCH_RESPONSE" | grep 'root cause' >/dev/null

PACK_RESPONSE="$(printf '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_relevant_context","arguments":{"query":"fix refresh token rotation","budget":160}}}\n' | "$CTX_BIN" --repo-root "$FIXTURE" mcp stdio)"
printf '%s\n' "$PACK_RESPONSE" | grep 'compact_context' >/dev/null

echo "CTX demo MCP smoke passed: $FIXTURE"
