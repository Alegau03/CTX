# CTX Practical Guide

This is the operational manual for using CTX inside OpenCode.

If you only want the product overview, start with [README.md](README.md). This guide is intentionally command-heavy: it shows what to run, where to run it, and what should happen.

For a recording-ready walkthrough, see [docs/demo-script.md](docs/demo-script.md).

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

1. Confirm the project builds/tests normally.
2. Install the `ctx` binary.
3. Run `ctx init`.
4. Run `ctx index`.
5. Run `ctx opencode install`.
6. Open `opencode` in the repo.
7. Run `/ctx` inside OpenCode.
8. Bootstrap graph memory with `/ctx-memory-bootstrap`.
9. Use `/ctx-memory-search`, `/ctx-retrieve`, and `/ctx-pack` during real work.
10. Run benchmarks once the graph-memory workflow is populated.

## Install CTX

From the repository root:

```bash
cargo install --locked --path crates/ctx-cli
```

If your shell cannot find `ctx` after install:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Verify:

```bash
ctx help
ctx doctor
```

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
```

Expected behavior:

- `ctx init` creates the local runtime.
- `ctx index` writes source, snippets, symbols, and graph links to `.ctx/graph.db`.
- `ctx opencode install` registers CTX as a local MCP server and generates OpenCode command files.

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
- It recommends the best next CTX command for the current repo state.

Useful first commands:

```text
/ctx-doctor
/ctx-index
/ctx-memory-bootstrap
/ctx-memory-search tests
/ctx-retrieve auth refresh token
/ctx-pack fix failing auth test
```

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
imported_files=4 imported_directives=23
- /repo/AGENTS.md => 18 directives
- /repo/CLAUDE.md => 2 directives
- /repo/CODEX.md => 2 directives
- /repo/.github/copilot-instructions.md => 1 directives
```

### Import One File Manually

Inside OpenCode:

```text
/ctx-memory-import AGENTS.md project markdown agents
```

Equivalent CLI command:

```bash
ctx memory import --from AGENTS.md --scope project --source markdown --prefix agents
```

Expected JSON fields:

```json
{
  "markdown_path": "AGENTS.md",
  "scope": "project",
  "source": "markdown",
  "imported": 12,
  "keys": ["agents.1", "agents.2"]
}
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
- the included fixture currently shows `75.81%` fewer rule tokens for graph memory than the full markdown source

### Add Or Update A Directive

Inside OpenCode:

```text
/ctx-memory-set testing.always_run Run targeted tests before completion.
```

Equivalent CLI command:

```bash
ctx memory set testing.always_run "Run targeted tests before completion." --scope project --source manual
```

Expected output:

```text
testing.always_run [project:manual]
Run targeted tests before completion.
```

### Inspect Or Remove Directives

```text
/ctx-memory-list
/ctx-memory-get testing.always_run
/ctx-memory-delete testing.always_run
```

CLI equivalents:

```bash
ctx memory list --scope project --limit 20
ctx memory get testing.always_run
ctx memory delete testing.always_run
```

### Export For Compatibility

Inside OpenCode:

```text
/ctx-memory-export AGENTS.generated.md project 200
```

CLI equivalent:

```bash
ctx memory export --to AGENTS.generated.md --scope project --limit 200
```

Use this only when a markdown artifact is needed for auditing or compatibility.

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

Expected output shape:

```text
src/http/refresh-route.ts score=...
src/auth/session.ts score=...
tests/auth/refresh-route.test.ts score=...
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

### Build A Compact Pack

Inside OpenCode:

```text
/ctx-pack fix refresh token rotation
```

CLI equivalent:

```bash
ctx pack "fix refresh token rotation" --json
```

Expected JSON fields:

```json
{
  "packed_tokens": 1200,
  "reduction_pct": 70.0,
  "pack_path": ".ctx/packs/pack-....json",
  "included": [],
  "excluded": []
}
```

### Ask Without Invoking A Host Agent

```bash
ctx ask "where is retry logic implemented?"
```

This prints compact context directly. It is useful for debugging CTX itself, but daily usage should happen through OpenCode.

## Logs And Diffs

### Prune Logs

Inside OpenCode:

```text
/ctx-prune-logs npm test -- --grep "refresh"
```

CLI pipe equivalent:

```bash
npm test -- --grep "refresh" 2>&1 | ctx prune logs --max-lines 50
```

Expected behavior:

- repeated success/noise lines are removed
- failing assertions and stack frames are preserved
- parser-specific diagnostics are kept when recognized
- if you only provide a topic instead of a runnable shell command, CTX should ask for the exact command instead of guessing

### Prune Diffs

Inside OpenCode:

```text
/ctx-prune-diff refresh token
```

CLI pipe equivalent:

```bash
git diff | ctx prune diff --query "refresh token"
```

Expected behavior:

- relevant hunks are kept
- unrelated hunks are collapsed
- output remains explainable and task-focused

## Benchmarks

### Single A/B Benchmark

Inside OpenCode:

```text
/ctx-benchmark-memory-ab run auth tests and fix root cause AGENTS.md 20
```

CLI equivalent:

```bash
ctx benchmark memory-ab "run auth tests and fix root cause" --markdown AGENTS.md --limit 20
```

Expected metrics:

- markdown tokens
- graph memory tokens
- token reduction percentage
- query-term coverage
- optional checklist-based success rate

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

Expected output:

```text
wrote benchmarks/report.md
wrote benchmarks/report.json
```

## MCP

OpenCode uses CTX through local stdio MCP after `ctx opencode install`.

Inspect the generated config:

```bash
ctx mcp config opencode
```

Expected shape:

```json
{
  "mcp": {
    "ctx": {
      "type": "local",
      "enabled": true,
      "command": ["ctx", "--repo-root", "/repo", "mcp", "stdio"]
    }
  }
}
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

The in-repo validation fixture is:

```text
demo/fixtures/opencode-auth-lab
```

Run the smoke flow:

```bash
cargo build --bin ctx
scripts/demo/opencode-auth-lab-smoke.sh ./target/debug/ctx
scripts/demo/opencode-auth-lab-mcp-smoke.sh ./target/debug/ctx
scripts/demo/opencode-auth-lab-benchmark.sh ./target/debug/ctx
```

Manual OpenCode flow:

```bash
ctx --repo-root demo/fixtures/opencode-auth-lab init
ctx --repo-root demo/fixtures/opencode-auth-lab index
ctx --repo-root demo/fixtures/opencode-auth-lab opencode install
cd demo/fixtures/opencode-auth-lab
opencode
```

Then inside OpenCode:

```text
/ctx
/ctx-memory-bootstrap
/ctx-memory-search auth root cause
/ctx-retrieve refresh token auth failure
/ctx-pack fix refresh token rotation
/ctx-benchmark-memory-suite benchmarks/memory-suite.toml benchmarks/report.md benchmarks/report.json
```

## Command Reference

| OpenCode command | CLI equivalent | What it does |
|---|---|---|
| `/ctx` | `ctx doctor` plus menu guidance | Shows the CTX command center |
| `/ctx-help` | `ctx help` | Shows every CTX command and examples |
| `/ctx-init` | `ctx init` | Initializes `.ctx/` runtime |
| `/ctx-index` | `ctx index` | Indexes files, symbols, snippets, and graph links |
| `/ctx-reindex` | `ctx reindex` | Re-indexes selected paths |
| `/ctx-doctor` | `ctx doctor` | Checks readiness and privacy defaults |
| `/ctx-pack <task>` | `ctx pack <task>` | Builds compact task context |
| `/ctx-ask <task>` | `ctx ask <task>` | Prints compact context directly |
| `/ctx-hook <task>` | `ctx hook <task>` | Produces pre-prompt hook payload |
| `/ctx-explain <task>` | `ctx explain <task>` | Explains likely intent and relevant context |
| `/ctx-retrieve <query>` | `ctx retrieve <query>` | Hybrid retrieval over graph/snippets/semantic ranking |
| `/ctx-graph-query <query>` | `ctx graph query <query>` | Searches graph paths and indexed context |
| `/ctx-prune-logs <shell command>` | `<shell command> 2>&1 | ctx prune logs --max-lines 50` | Compacts noisy logs |
| `/ctx-prune-diff <topic>` | `ctx prune diff --query <topic>` | Compacts diffs around relevant hunks |
| `/ctx-memory-bootstrap` | `ctx memory bootstrap` | Imports conventional markdown rules into graph memory |
| `/ctx-memory-import <file>` | `ctx memory import --from <file>` | Imports one markdown file |
| `/ctx-memory-search <query>` | `ctx memory search <query>` | Finds relevant memory directives |
| `/ctx-memory-list` | `ctx memory list` | Lists memory directives |
| `/ctx-memory-get <key>` | `ctx memory get <key>` | Reads one directive |
| `/ctx-memory-set <key> <body>` | `ctx memory set <key> <body>` | Creates or updates a directive |
| `/ctx-memory-delete <key>` | `ctx memory delete <key>` | Deletes a directive |
| `/ctx-memory-export <file>` | `ctx memory export --to <file>` | Exports graph memory to markdown |
| `/ctx-benchmark-memory-ab ...` | `ctx benchmark memory-ab ...` | Compares markdown vs graph memory |
| `/ctx-benchmark-memory-suite ...` | `ctx benchmark memory-suite ...` | Runs a benchmark suite and writes reports |
| `/ctx-stats` | `ctx stats` | Shows latest local token/runtime stats |
| `/ctx-mcp-stdio` | `ctx mcp stdio` | Shows stdio MCP launch command |
| `/ctx-mcp-serve` | `ctx mcp serve` | Starts/debugs localhost JSON-RPC server |
| `/ctx-mcp-config-opencode` | `ctx mcp config opencode` | Prints OpenCode MCP config |
| `/ctx-opencode-install` | `ctx opencode install` | Refreshes OpenCode integration files |
