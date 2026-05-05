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
test -f "$SMOKE_DIR/.opencode/commands/ctx-plan.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-compare.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-dashboard.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-gain.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-read.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-run.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-doctor.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-memory-bootstrap.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-memory-search.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-toolbook-import.md"
test -f "$SMOKE_DIR/.opencode/commands/ctx-learn.md"
test -f "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md"
test -f "$SMOKE_DIR/.opencode/plugins/ctx-dashboard.tsx"
test -f "$SMOKE_DIR/.opencode/package.json"
test -f "$SMOKE_DIR/.opencode/tui.json"

grep '"$schema": "https://opencode.ai/config.json"' "$SMOKE_DIR/opencode.json" >/dev/null
grep '"mcp"' "$SMOKE_DIR/opencode.json" >/dev/null
grep '"ctx"' "$SMOKE_DIR/opencode.json" >/dev/null
grep '"instructions"' "$SMOKE_DIR/opencode.json" >/dev/null
grep '.opencode/instructions/ctx-host-first.md' "$SMOKE_DIR/opencode.json" >/dev/null
grep 'stdio' "$SMOKE_DIR/opencode.json" >/dev/null

grep 'description:' "$SMOKE_DIR/.opencode/commands/ctx-pack.md" >/dev/null
grep 'pack "$ARGUMENTS"' "$SMOKE_DIR/.opencode/commands/ctx-pack.md" >/dev/null
grep 'CTX Plan' "$SMOKE_DIR/.opencode/commands/ctx-plan.md" >/dev/null
grep '## 🧭 CTX Plan' "$SMOKE_DIR/.opencode/commands/ctx-plan.md" >/dev/null
grep 'retrieve "$ARGUMENTS" --limit 8 --json' "$SMOKE_DIR/.opencode/commands/ctx-plan.md" >/dev/null
grep 'Before vs CTX' "$SMOKE_DIR/.opencode/commands/ctx-compare.md" >/dev/null
grep 'CTX Dashboard snapshot.' "$SMOKE_DIR/.opencode/commands/ctx-dashboard.md" >/dev/null
grep 'host-dashboard' "$SMOKE_DIR/.opencode/commands/ctx-dashboard.md" >/dev/null
! grep -- '--json host-dashboard' "$SMOKE_DIR/.opencode/commands/ctx-dashboard.md" >/dev/null
grep 'CTX Dashboard' "$SMOKE_DIR/.opencode/plugins/ctx-dashboard.tsx" >/dev/null
grep -- '--json stats --history 20' "$SMOKE_DIR/.opencode/commands/ctx-gain.md" >/dev/null
grep '## 💸 CTX Gain' "$SMOKE_DIR/.opencode/commands/ctx-gain.md" >/dev/null
grep 'host-read' "$SMOKE_DIR/.opencode/commands/ctx-read.md" >/dev/null
grep '## 📖 CTX Read' "$SMOKE_DIR/.opencode/commands/ctx-read.md" >/dev/null
grep -- '--json host-run "$ARGUMENTS"' "$SMOKE_DIR/.opencode/commands/ctx-run.md" >/dev/null
grep '## 🧪 CTX Run' "$SMOKE_DIR/.opencode/commands/ctx-run.md" >/dev/null
grep 'toolbook:$1' "$SMOKE_DIR/.opencode/commands/ctx-toolbook-import.md" >/dev/null
grep -- '--source learned' "$SMOKE_DIR/.opencode/commands/ctx-learn.md" >/dev/null
grep 'deterministic CTX menu command' "$SMOKE_DIR/.opencode/commands/ctx.md" >/dev/null
grep 'ready: true' "$SMOKE_DIR/.opencode/commands/ctx-doctor.md" >/dev/null
grep 'do not inspect files manually' "$SMOKE_DIR/.opencode/commands/ctx-doctor.md" >/dev/null
grep 'Automatic CTX Usage' "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null
grep 'Install profile: `full`' "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null
grep '/ctx-plan' "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null
grep '/ctx-dashboard' "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null
grep '/ctx-gain' "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null
grep '/ctx-read' "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null
grep '/ctx-run' "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null
grep '/ctx-toolbook-import' "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null
grep 'Do not revive wrapper-style workflows' "$SMOKE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null
grep 'sidebar_content' "$SMOKE_DIR/.opencode/plugins/ctx-dashboard.tsx" >/dev/null
grep 'host-dashboard' "$SMOKE_DIR/.opencode/plugins/ctx-dashboard.tsx" >/dev/null
grep '@opencode-ai/plugin' "$SMOKE_DIR/.opencode/package.json" >/dev/null
grep '@opentui/solid' "$SMOKE_DIR/.opencode/package.json" >/dev/null
grep 'solid-js' "$SMOKE_DIR/.opencode/package.json" >/dev/null
grep '1.14.19' "$SMOKE_DIR/.opencode/package.json" >/dev/null
grep '0.1.101' "$SMOKE_DIR/.opencode/package.json" >/dev/null
grep '"$schema": "https://opencode.ai/tui.json"' "$SMOKE_DIR/.opencode/tui.json" >/dev/null
grep './plugins/ctx-dashboard.tsx' "$SMOKE_DIR/.opencode/tui.json" >/dev/null

command_count="$(find "$SMOKE_DIR/.opencode/commands" -type f | wc -l | tr -d ' ')"
if [[ "$command_count" -lt 20 ]]; then
  echo "expected at least 20 OpenCode command files, found $command_count" >&2
  exit 1
fi

CORE_DIR="$(mktemp -d)"
trap 'rm -rf "$SMOKE_DIR" "$CORE_DIR"' EXIT
"$CTX_BIN" --repo-root "$CORE_DIR" init >/dev/null
"$CTX_BIN" --repo-root "$CORE_DIR" index >/dev/null
"$CTX_BIN" --repo-root "$CORE_DIR" opencode install --profile core >/dev/null
test -f "$CORE_DIR/.opencode/commands/ctx-plan.md"
test -f "$CORE_DIR/.opencode/commands/ctx-gain.md"
test ! -f "$CORE_DIR/.opencode/commands/ctx-dashboard.md"
test ! -f "$CORE_DIR/.opencode/commands/ctx-read.md"
test ! -f "$CORE_DIR/.opencode/plugins/ctx-dashboard.tsx"
test ! -f "$CORE_DIR/.opencode/tui.json"
grep 'Install profile: `core`' "$CORE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null
grep 'ctx opencode install --profile full' "$CORE_DIR/.opencode/instructions/ctx-host-first.md" >/dev/null

echo "CTX OpenCode smoke passed: $CTX_BIN"
