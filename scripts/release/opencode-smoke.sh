#!/usr/bin/env bash
set -euo pipefail

CTX_BIN="${1:-ctx}"
SMOKE_DIR="$(mktemp -d)"
trap 'rm -rf "$SMOKE_DIR"' EXIT

if [[ "$CTX_BIN" != */* ]]; then
  CTX_BIN="$(command -v "$CTX_BIN")"
fi

"$CTX_BIN" --repo-root "$SMOKE_DIR" init >/dev/null

mkdir -p "$SMOKE_DIR/src"
printf 'fn main() { println!("ctx"); }\n' > "$SMOKE_DIR/src/main.rs"
"$CTX_BIN" --repo-root "$SMOKE_DIR" index >/dev/null
"$CTX_BIN" --repo-root "$SMOKE_DIR" opencode install >/dev/null

test -f "$SMOKE_DIR/opencode.json"
test -f "$SMOKE_DIR/.opencode/commands/ctx-pack.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-doctor.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-memory-bootstrap.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-memory-search.md"
test -f "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md"

grep '"$schema": "https://opencode.ai/config.json"' "$SMOKE_DIR/opencode.json" >/dev/null
grep '"mcp"' "$SMOKE_DIR/opencode.json" >/dev/null
grep '"ctx"' "$SMOKE_DIR/opencode.json" >/dev/null
grep '"instructions"' "$SMOKE_DIR/opencode.json" >/dev/null
grep '.opencode/instructions/ctx-host-first.md' "$SMOKE_DIR/opencode.json" >/dev/null
grep 'stdio' "$SMOKE_DIR/opencode.json" >/dev/null

grep 'description:' "$SMOKE_DIR/.opencode/commands/ctx-pack.md" >/dev/null
grep 'ctx pack' "$SMOKE_DIR/.opencode/commands/ctx-pack.md" >/dev/null
grep 'Automatic CTX Usage' "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null
grep 'Do not revive wrapper-style workflows' "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null

command_count="$(find "$SMOKE_DIR/.opencode/commands" -type f | wc -l | tr -d ' ')"
if [[ "$command_count" -lt 20 ]]; then
  echo "expected at least 20 OpenCode command files, found $command_count" >&2
  exit 1
fi

echo "CTX OpenCode smoke passed: $CTX_BIN"
