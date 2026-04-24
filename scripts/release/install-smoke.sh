#!/usr/bin/env bash
set -euo pipefail

CTX_BIN="${1:-ctx}"
SMOKE_DIR="$(mktemp -d)"
trap 'rm -rf "$SMOKE_DIR"' EXIT

if [[ "$CTX_BIN" != */* ]]; then
  CTX_BIN="$(command -v "$CTX_BIN")"
fi

"$CTX_BIN" help >/dev/null
"$CTX_BIN" --repo-root "$SMOKE_DIR" doctor | grep 'config: missing' >/dev/null
"$CTX_BIN" --repo-root "$SMOKE_DIR" init >/dev/null
"$CTX_BIN" --repo-root "$SMOKE_DIR" doctor | grep 'local_only: true' >/dev/null

mkdir -p "$SMOKE_DIR/src"
printf 'fn main() {}\n' > "$SMOKE_DIR/src/main.rs"
"$CTX_BIN" --repo-root "$SMOKE_DIR" index | grep 'indexed_files:' >/dev/null
"$CTX_BIN" --repo-root "$SMOKE_DIR" pack 'explain main function' >/dev/null
"$CTX_BIN" --repo-root "$SMOKE_DIR" stats | grep 'packed_tokens' >/dev/null

printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n' \
  | "$CTX_BIN" --repo-root "$SMOKE_DIR" mcp stdio \
  | grep 'ctx-mcp' >/dev/null

echo "CTX install smoke passed: $CTX_BIN"
