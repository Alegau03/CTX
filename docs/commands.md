# CTX Commands

This document is the single reference for the CTX command surface.

Use it when you want:

- the exact CLI syntax
- the matching OpenCode slash command when one exists
- a plain-English explanation of what each command does
- one concrete example per command

For the end-to-end workflow, see [guide.md](../guide.md).

## Global Options

These options can be used before most CLI commands:

- `--repo-root <path>`: run CTX against a specific repository root
- `--budget <n>`: override the context token budget
- `--json`: print machine-readable JSON when supported
- `--attach <file>`: attach a diagnostic file, mainly for `pack`

Example:

```bash
ctx --repo-root /path/to/repo --budget 4000 pack "fix auth regression" --json
```

## OpenCode Setup

### `ctx init`

- OpenCode: `/ctx-init`
- What it does: creates `.ctx/`, the local config, and the graph database scaffold.

```bash
ctx init
```

### `ctx index [paths...]`

- OpenCode: `/ctx-index`
- What it does: indexes files, symbols, snippets, and graph links for retrieval.

```bash
ctx index
ctx index src tests
```

### `ctx reindex [paths...]`

- OpenCode: `/ctx-reindex`
- What it does: refreshes indexing for selected paths without rebuilding everything from scratch.

```bash
ctx reindex src tests
```

### `ctx doctor`

- OpenCode: `/ctx-doctor`
- What it does: checks repo readiness, privacy defaults, graph presence, local stats, and audit paths.

```bash
ctx doctor
```

### `ctx opencode install`

- OpenCode: `/ctx-opencode-install`
- What it does: writes `opencode.json`, `.opencode/commands/*.md`, and `.opencode/instructions/ctx-host-first.md`.

```bash
ctx opencode install
```

### `ctx menu`

- OpenCode: `/ctx`
- What it does: prints the CTX command center and suggests the best next command for the current repo state.

```bash
ctx menu
```

### `ctx help`

- OpenCode: `/ctx-help`
- What it does: shows the public CTX command guide from the CLI.

```bash
ctx help
```

## Retrieval And Context

### `ctx retrieve <query> [--limit <n>]`

- OpenCode: `/ctx-retrieve <query>`
- What it does: runs hybrid retrieval across graph data, snippets, symbols, and semantic ranking.

```bash
ctx retrieve "refresh token auth failure" --limit 8
```

### `ctx pack <query> [--json] [--attach <file>] [--budget <n>]`

- OpenCode: `/ctx-pack <task>`
- What it does: builds a compact task-specific context pack and stores an artifact under `.ctx/packs/`.

```bash
ctx pack "fix refresh token rotation" --json
ctx pack "fix failing auth test" --attach /tmp/fail.log --json
```

### `ctx ask <query>`

- OpenCode: `/ctx-ask <task>`
- What it does: prints a compact context block directly for a human or host to reuse.

```bash
ctx ask "where is retry logic implemented?"
```

### `ctx hook <query>`

- OpenCode: `/ctx-hook <task>`
- What it does: produces a deterministic pre-prompt payload for hook or preprocessing workflows.

```bash
ctx hook "fix flaky auth test"
```

### `ctx explain <query>`

- OpenCode: `/ctx-explain <task>`
- What it does: explains likely intent and which context CTX considers relevant.

```bash
ctx explain "fix failing pytest in auth"
```

### `ctx stats`

- OpenCode: `/ctx-stats`
- What it does: prints the latest local telemetry snapshot, including token reduction and runtime metadata.

```bash
ctx stats
```

## Memory Commands

Graph memory is CTX's structured replacement for repeatedly loading whole markdown instruction files.

### `ctx memory bootstrap [paths...] [--scope <scope>] [--source <source>]`

- OpenCode: `/ctx-memory-bootstrap`
- What it does: imports conventional rule files such as `AGENTS.md`, `CLAUDE.md`, `CODEX.md`, and `.github/copilot-instructions.md`.

```bash
ctx memory bootstrap
ctx memory bootstrap AGENTS.md CLAUDE.md CODEX.md .github/copilot-instructions.md
```

### `ctx memory import --from <file> [--scope <scope>] [--source <source>] [--prefix <prefix>]`

- OpenCode: `/ctx-memory-import <file>`
- What it does: imports one markdown file into graph memory directives.

```bash
ctx memory import --from AGENTS.md --scope project --source markdown --prefix agents
```

### `ctx memory search <query> [--scope <scope>] [--limit <n>]`

- OpenCode: `/ctx-memory-search <query>`
- What it does: searches stored directives by topic, keyword, or task intent.

```bash
ctx memory search "auth tests root cause" --scope project --limit 10
```

### `ctx memory list [--scope <scope>] [--limit <n>]`

- OpenCode: `/ctx-memory-list`
- What it does: lists recent memory directives, optionally filtered by scope.

```bash
ctx memory list --scope project --limit 10
```

### `ctx memory get <key>`

- OpenCode: `/ctx-memory-get <key>`
- What it does: reads one directive by key.

```bash
ctx memory get testing.always_run
```

### `ctx memory set <key> <body> [--scope <scope>] [--source <source>]`

- OpenCode: `/ctx-memory-set <key> <body>`
- What it does: creates or updates one graph-backed directive.

```bash
ctx memory set testing.always_run "Run targeted tests before completion." --scope project --source manual
```

### `ctx memory delete <key>`

- OpenCode: `/ctx-memory-delete <key>`
- What it does: deletes one directive from graph memory.

```bash
ctx memory delete testing.always_run
```

### `ctx memory export --to <file> [--scope <scope>] [--limit <n>]`

- OpenCode: `/ctx-memory-export <file>`
- What it does: exports graph memory back to markdown for auditing or compatibility.

```bash
ctx memory export --to AGENTS.generated.md --scope project --limit 200
```

## Graph Commands

### `ctx graph build`

- OpenCode: `/ctx-graph-build`
- What it does: builds graph data by indexing the repository.

```bash
ctx graph build
```

### `ctx graph rebuild`

- OpenCode: `/ctx-graph-rebuild`
- What it does: explicit alias of `ctx graph build`.

```bash
ctx graph rebuild
```

### `ctx graph query <query>`

- OpenCode: `/ctx-graph-query <query>`
- What it does: searches indexed graph paths and related context by keyword.

```bash
ctx graph query auth
```

## Pruning Commands

### `ctx prune logs [--max-lines <n>]`

- OpenCode: `/ctx-prune-logs <shell command>`
- What it does: removes repeated or low-signal log lines and keeps the failure root cause readable.

```bash
pytest -q 2>&1 | ctx prune logs --max-lines 50
npm run test:auth 2>&1 | ctx prune logs --max-lines 50
```

### `ctx prune diff [query] [--query <query>]`

- OpenCode: `/ctx-prune-diff <topic>`
- What it does: compacts diffs and keeps the hunks most relevant to the topic.

```bash
git diff | ctx prune diff --query "refresh token"
```

## Benchmarks

### `ctx benchmark memory-ab <query> --markdown <file> [--limit <n>]`

- OpenCode: `/ctx-benchmark-memory-ab ...`
- What it does: compares markdown instructions against graph memory on token usage, coverage, and optional quality signals.

```bash
ctx benchmark memory-ab "run tests and fix root cause" --markdown AGENTS.md --limit 20
```

### `ctx benchmark memory-suite --spec <file> --report-out <file> [--json-out <file>]`

- OpenCode: `/ctx-benchmark-memory-suite ...`
- What it does: runs a reusable benchmark suite from a spec file and writes markdown and JSON reports.

```bash
ctx benchmark memory-suite --spec benchmarks/memory-ab.example.toml --report-out benchmarks/report.md --json-out benchmarks/report.json
```

## MCP Commands

### `ctx mcp stdio`

- OpenCode: `/ctx-mcp-stdio`
- What it does: runs CTX as an MCP JSON-RPC server over stdin/stdout for local host integration.

```bash
ctx --repo-root /path/to/project mcp stdio
```

### `ctx mcp serve [--port <port>] [--once]`

- OpenCode: `/ctx-mcp-serve`
- What it does: starts the localhost HTTP JSON-RPC MCP server.

```bash
ctx mcp serve --port 8765
ctx mcp serve --port 8765 --once
```

### `ctx mcp config <client>`

- OpenCode: `/ctx-mcp-config-opencode`
- What it does: prints an MCP configuration snippet for OpenCode or a generic HTTP client.

```bash
ctx mcp config opencode
ctx mcp config http
```

## Recommended Daily Flow

For most repos, the shortest useful path is:

```bash
ctx init
ctx index
ctx opencode install
opencode
```

Then inside OpenCode:

```text
/ctx
/ctx-doctor
/ctx-memory-bootstrap
/ctx-memory-search auth
/ctx-retrieve refresh token auth failure
/ctx-pack fix auth refresh regression
/ctx-prune-logs npm run test:auth
/ctx-stats
```
