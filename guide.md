# CTX Practical Guide

This is the operational manual for using CTX inside OpenCode.

If you only want the product overview, start with [README.md](README.md). This guide is intentionally command-heavy: it shows what to run, where to run it, and what should happen.

For a recording-ready walkthrough, see [docs/demo-script.md](docs/demo-script.md). For exact syntax of every command, jump to [docs/commands.md](docs/commands.md).

## Contents

- [Recommended Order](#recommended-order)
- [Install CTX](#install-ctx)
- [Enable CTX In A Repo](#enable-ctx-in-a-repo)
- [OpenCode-First Workflow](#opencode-first-workflow)
- [Graph Memory Workflow](#graph-memory-workflow)
- [Context And Retrieval](#context-and-retrieval)
- [Logs And Diffs](#logs-and-diffs)
- [Benchmarks](#benchmarks)
- [MCP](#mcp)
- [Demo Fixture](#demo-fixture)
- [Command Reference](#command-reference)

## Recommended Order

Use CTX in this order in a real repository:

1. Confirm the project already builds or tests normally.
2. Install the `ctx` binary.
3. Run `ctx init`.
4. Run `ctx index`.
5. Run `ctx opencode install`.
6. Open `opencode` in the repo.
7. Run `/ctx`.
8. Bootstrap graph memory with `/ctx-memory-bootstrap`.
9. Use `/ctx-plan`, `/ctx-retrieve`, `/ctx-read`, `/ctx-pack`, and `/ctx-run` during normal work.
10. Use `/ctx-gain` and `/ctx-dashboard` to verify the token-savings story.

## Install CTX

Recommended public install paths:

```bash
cargo install ctx-cli
curl -fsSL https://raw.githubusercontent.com/Alegau03/CTX/main/scripts/install.sh | sh
npm i -g @alegau/ctx-bin
brew tap Alegau03/ctx && brew install ctx
```

If you want the full installation matrix, update paths, release archive flow, or verification steps, use [docs/install.md](docs/install.md).

If your shell cannot find `ctx` after install:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Verify:

```bash
ctx help
ctx doctor
ctx update --check
```

Native update flow:

```bash
ctx update
```

If CTX cannot confidently detect the install channel, it prints all supported update paths instead of guessing.

Expected before initialization:

```text
CTX Doctor
config: missing
next: ctx init
```

## Enable CTX In A Repo

Run this from the project root:

```bash
ctx init
ctx index
ctx opencode install
```

Optional lean setup:

```bash
ctx opencode install --profile core
```

Expected files:

```text
.ctx/config.toml
.ctx/graph.db
.ctx/packs/
.ctx/stats/
.ctx/audit.log
opencode.json
.opencode/commands/ctx.md
.opencode/instructions/ctx-host-first.md
.opencode/tui.json                         # full profile only
.opencode/plugins/ctx-dashboard.tsx        # full profile only
.opencode/package.json                     # full profile only
```

Expected behavior:

- `ctx init` creates the local runtime.
- `ctx index` writes source, snippets, symbols, and graph links to `.ctx/graph.db`.
- `ctx opencode install` registers CTX as a local MCP server and generates OpenCode command files.
- `ctx opencode install` with the default `full` profile also provisions the live right-sidebar `CTX Dashboard`.
- `ctx opencode install --profile core` keeps only the smallest daily slash-command surface.
- rerunning `ctx opencode install --profile full` restores the complete CTX surface and sidebar.

## OpenCode-First Workflow

Open OpenCode from the same repository:

```bash
opencode
```

Start with:

```text
/ctx
```

Expected behavior:

- OpenCode shows the CTX Command Center.
- The menu is organized by setup, context, memory, debug, benchmark, and MCP.
- It reports repository status from `ctx menu`, not from a manual repo scan.
- It recommends the best next CTX command for the current repo state.

Useful first commands:

```text
/ctx-doctor
/ctx-memory-bootstrap
/ctx-plan fix auth refresh regression
/ctx-retrieve refresh token auth failure
/ctx-read src/auth.ts outline
/ctx-pack fix auth refresh regression
/ctx-compare fix auth refresh regression
/ctx-run npm run test:auth
/ctx-gain
/ctx-dashboard
```

The generated OpenCode commands prefer a stable result-first format such as:

- `## 🧭 CTX Plan`
- `## 📖 CTX Read`
- `## 🧪 CTX Run`
- `## 📊 CTX Dashboard`
- `## 💸 CTX Gain`

With the `full` profile, OpenCode should also show a `CTX Dashboard` panel in the right sidebar. It auto-refreshes local runtime metrics such as:

- total estimated tokens saved
- average saved per run
- reduction percentages
- read-cache hit rate
- index-cache reuse rate
- top current win
- latest pack artifact

The sidebar is intentionally compact: it shows the live signal, while detailed activity and audit trails stay in the normal CTX command outputs.

Expected `doctor` shape after `ctx init` + `ctx index`:

```text
indexed_files: <n>
ready: true
next: ctx memory bootstrap
```

`ready: true` means CTX is operational. `next:` is then a recommended workflow step, not a blocker.

## Graph Memory Workflow

Graph memory is CTX's replacement for repeatedly rereading large project-instruction markdown files.

### Bootstrap From Markdown

Inside OpenCode:

```text
/ctx-memory-bootstrap
```

Equivalent CLI command:

```bash
ctx memory bootstrap
```

Default scanned files:

- `AGENTS.md`
- `CLAUDE.md`
- `CODEX.md`
- `.github/copilot-instructions.md`

Expected output shape:

```text
imported_files=4 imported_directives=27
- /repo/AGENTS.md => 13 directives
- /repo/CLAUDE.md => 6 directives
- /repo/CODEX.md => 6 directives
- /repo/.github/copilot-instructions.md => 2 directives
```

### Search Relevant Directives

Inside OpenCode:

```text
/ctx-memory-search auth tests root cause
```

Equivalent CLI command:

```bash
ctx memory search "auth tests root cause" --scope project --limit 10
```

Expected output:

```text
[project:markdown:agents.3] Run targeted auth tests before claiming completion.
[project:manual:auth.root_cause] Fix the real refresh-token root cause instead of bypassing failures.
```

Why this saves tokens:

- markdown flow sends the whole instruction file repeatedly
- graph flow retrieves only the directives related to the current task
- the included fixture currently shows `56.72%` fewer rule tokens for graph memory than the full markdown source while keeping `markdown=1.00` and `graph=1.00` query coverage

### Add, Inspect, Or Export Directives

```text
/ctx-memory-set testing.always_run Run targeted tests before completion.
/ctx-memory-list
/ctx-memory-get testing.always_run
/ctx-memory-export AGENTS.generated.md project 200
```

Equivalent CLI commands:

```bash
ctx memory set testing.always_run "Run targeted tests before completion." --scope project --source manual
ctx memory list --scope project --limit 20
ctx memory get testing.always_run
ctx memory export --to AGENTS.generated.md --scope project --limit 200
```

## Context And Retrieval

### Retrieve Relevant Files And Snippets

Inside OpenCode:

```text
/ctx-retrieve refresh token auth failure
```

CLI equivalent:

```bash
ctx retrieve "refresh token auth failure" --limit 8
```

### Query The Graph

Inside OpenCode:

```text
/ctx-graph-query auth
```

CLI equivalent:

```bash
ctx graph query auth
```

### Read With Session Cache / Re-Read Compression

Inside OpenCode:

```text
/ctx-read src/features/auth/session.ts outline
/ctx-read src/features/auth/session.ts digest
/ctx-read src/features/auth/session.ts digest
```

Expected behavior:

- first `digest` reads and fingerprints the file
- the second `digest` can return a cache hit when the file is unchanged
- `outline` keeps the read structural instead of dumping the full body

### Build A Compact Pack

Inside OpenCode:

```text
/ctx-pack fix refresh token rotation
/ctx-compare fix refresh token rotation
/ctx-plan fix refresh token rotation
```

CLI equivalents:

```bash
ctx pack "fix refresh token rotation" --json
ctx explain "fix refresh token rotation"
```

Expected pack JSON fields:

```json
{
  "packed_tokens": 1200,
  "reduction_pct": 70.0,
  "pack_path": ".ctx/packs/pack-....json",
  "included": [],
  "excluded": []
}
```

## Logs And Diffs

### Run And Compress Logs

Inside OpenCode:

```text
/ctx-run npm run test:auth
/ctx-prune-logs npm run test:auth
```

CLI pipe equivalent:

```bash
npm run test:auth 2>&1 | ctx prune logs --max-lines 50
```

Expected behavior:

- repeated noise is removed
- failing assertions and stack frames are preserved
- parser-specific diagnostics are kept when recognized
- the raw log path stays available for deeper inspection

### Prune Diffs

Inside OpenCode:

```text
/ctx-prune-diff refresh token
```

CLI pipe equivalent:

```bash
git diff | ctx prune diff --query "refresh token"
```

## Benchmarks

### Local Runtime Checks

Inside OpenCode:

```text
/ctx-gain
/ctx-dashboard
/ctx-stats
```

Use these to verify that the graph-memory and compact-pack workflow is actually saving tokens in the current repo.

### Single A/B Benchmark

Inside OpenCode:

```text
/ctx-benchmark-memory-ab run auth tests and fix root cause AGENTS.md 20
```

CLI equivalent:

```bash
ctx benchmark memory-ab "run auth tests and fix root cause" --markdown AGENTS.md --limit 20
```

### Suite Benchmark

Inside OpenCode:

```text
/ctx-benchmark-memory-suite benchmarks/memory-suite.toml benchmarks/report.md benchmarks/report.json
```

CLI equivalent:

```bash
ctx benchmark memory-suite \
  --spec benchmarks/memory-suite.toml \
  --report-out benchmarks/report.md \
  --json-out benchmarks/report.json
```

## MCP

OpenCode uses CTX through local stdio MCP after `ctx opencode install`.

Inspect the generated config:

```bash
ctx mcp config opencode
```

Validate the OpenCode MCP handshake directly:

```bash
opencode mcp list --print-logs --log-level DEBUG --pure
```

Expected output:

```text
ctx connected
toolCount=13
```

Low-level stdio mode:

```bash
ctx --repo-root /path/to/project mcp stdio
```

HTTP JSON-RPC mode for local debugging:

```bash
ctx mcp serve --port 8765
```

## Demo Fixture

The main committed demo project is:

```text
demo/fixtures/opencode-auth-lab
```

Useful scripts:

```bash
scripts/demo/opencode-auth-lab-smoke.sh ./target/debug/ctx
scripts/demo/opencode-auth-lab-mcp-smoke.sh ./target/debug/ctx
scripts/demo/opencode-auth-lab-benchmark.sh ./target/debug/ctx
```

The fixture is also linked from [docs/demo-walkthrough.md](docs/demo-walkthrough.md) and the recording flow in [docs/demo-script.md](docs/demo-script.md).

## Command Reference

The complete command reference lives in [docs/commands.md](docs/commands.md).

Use it when you want:

- exact CLI syntax
- the matching OpenCode slash command
- one example per command
- the full OpenCode-only command list for `/ctx-plan`, `/ctx-compare`, `/ctx-read`, `/ctx-run`, `/ctx-gain`, `/ctx-dashboard`, Toolbooks, and `/ctx-learn`
