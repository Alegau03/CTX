# CTX Commands

This is the single reference for the CTX command surface.

Use it when you want:

- the exact CLI syntax
- the matching OpenCode slash command when one exists
- a plain-English explanation of what each command does
- one concrete example per command

For the end-to-end workflow, start with [guide.md](../guide.md).

## OpenCode-Only Commands

Some workflows exist only as OpenCode slash commands. They intentionally do not add public CLI subcommands, because CTX's product surface is OpenCode-first and the host should stay in control.

OpenCode-only commands in this document:

- `/ctx-compare <task>`
- `/ctx-dashboard`
- `/ctx-gain`
- `/ctx-plan <task>`
- `/ctx-read <file> [mode]`
- `/ctx-run <shell command>`
- `/ctx-toolbook-import <name> <file>`
- `/ctx-toolbook-search <name> "<query>"`
- `/ctx-toolbook-list <name>`
- `/ctx-toolbook-pack <name> "<task>"`
- `/ctx-learn <key> "<body>"`

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
- Delta-aware behavior: unchanged files are reused from the local cache, and the latest summary is written to `.ctx/cache/index-report.json`.

```bash
ctx index
ctx index src tests
```

### `ctx reindex [paths...]`

- OpenCode: `/ctx-reindex`
- What it does: refreshes indexing for selected paths without rebuilding everything from scratch.
- Delta-aware behavior: when selected files are unchanged, CTX can skip reprocessing them and record the reuse in `.ctx/cache/index-report.json`.

```bash
ctx reindex src tests
```

### `ctx doctor`

- OpenCode: `/ctx-doctor`
- What it does: checks repo readiness, privacy defaults, graph presence, local stats, and audit paths.

```bash
ctx doctor
```

### `ctx update [--check] [--yes] [--channel <name>]`

- OpenCode: none; this is a public CLI install-management command
- What it does: checks the latest CTX version, detects how CTX was installed when possible, and prints the safest update path for that install channel.
- `--check`: reports current version, latest version, detected channel, and whether an update is available without modifying anything
- `--channel`: forces one of `installer`, `cargo`, `npm`, or `brew`
- `--yes`: executes the installer update path only when the install channel is confidently detected as `installer`; other channels stay guided and print the exact manual command

```bash
ctx update --check
ctx update --channel cargo
ctx update --channel brew
```

### `ctx opencode install`

- OpenCode: `/ctx-opencode-install`
- What it does: writes `opencode.json`, `.opencode/commands/*.md`, and `.opencode/instructions/ctx-host-first.md`.
- Full profile extras: also provisions `.opencode/tui.json`, `.opencode/plugins/ctx-dashboard.tsx`, and `.opencode/package.json` so OpenCode can render the live `CTX Dashboard` in the right sidebar.
- Profiles:
  - `full` (default): full OpenCode CTX surface plus the live sidebar dashboard
  - `core`: lean daily workflow with only `/ctx`, `/ctx-doctor`, `/ctx-plan`, `/ctx-retrieve`, `/ctx-pack`, `/ctx-run`, `/ctx-prune-logs`, `/ctx-stats`, and `/ctx-gain`

```bash
ctx opencode install
ctx opencode install --profile core
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
- Cache explainability: when available, pack metadata includes the latest local index-cache summary so reuse of unchanged indexed files is visible.

```bash
ctx pack "fix refresh token rotation" --json
ctx pack "fix failing auth test" --attach /tmp/fail.log --json
```

### `/ctx-compare <task>` OpenCode-only

- CLI equivalent: none; it uses `ctx pack <task> --json` internally from OpenCode.
- What it does: shows a compact before-vs-CTX table using `original_estimated_tokens`, `packed_tokens`, `reduction_pct`, and `pack_path`.

```text
/ctx-compare fix auth refresh regression
```

### `/ctx-plan <task>` OpenCode-only

- CLI equivalent: none; it combines existing CTX primitives from inside OpenCode.
- What it does: builds a graph-backed implementation plan using retrieval, graph query, memory search, and a compact context pack.
- Output includes: task summary, intent, relevant context, token efficiency, implementation steps, suggested tests, and the first action.
- Output format: stable markdown sections beginning with `## 🧭 CTX Plan`.

```text
/ctx-plan add a registration with email button in the login menu
```

### `/ctx-gain` OpenCode-only

- CLI equivalent: none; it uses `ctx --json stats --history 20` internally from OpenCode.
- What it does: shows recent token savings, biggest wins, and top repeated queries from local stats history.
- Output format: stable markdown sections beginning with `## 💸 CTX Gain`.

```text
/ctx-gain
```

### `/ctx-dashboard` OpenCode-only

- CLI equivalent: none; it uses the hidden helper `ctx --json host-dashboard` internally from OpenCode.
- What it does: shows a local dashboard snapshot for savings, cache ratios, top wins, recent audit activity, and related runtime telemetry.
- Sidebar note: the `full` OpenCode install profile also exposes the same savings and cache story in the live right-sidebar `CTX Dashboard`, in a more compact form focused on metrics, cache behavior, top win, and latest artifact.
- Output format: stable markdown sections beginning with `## 📊 CTX Dashboard`.

```text
/ctx-dashboard
```

### `/ctx-read <file> [mode]` OpenCode-only

- CLI equivalent: none; it uses the hidden helper `ctx --json host-read <file> --mode <mode>` internally from OpenCode.
- What it does: reads one repository file with `full`, `outline`, or `digest` mode and uses a local session read cache to compress unchanged rereads.
- Output format: stable markdown sections beginning with `## 📖 CTX Read`.
- Modes:
  - `full`: full file body for explicit deep inspection
  - `outline`: symbols, headings, signatures, and structure-first view
  - `digest`: compact fingerprint-oriented reread response for unchanged files

```text
/ctx-read src/auth.ts
/ctx-read src/auth.ts outline
/ctx-read docs/runbook.md digest
```

### `/ctx-run <shell command>` OpenCode-only

- CLI equivalent: none; it uses the hidden helper `ctx --json host-run "<shell command>"` internally from OpenCode.
- What it does: runs one repository-scoped shell command, captures combined output, prunes the noise, and keeps the root cause plus the raw-log path.
- Use it when: you want the normal OpenCode debugging flow in one step instead of piping logs manually.
- Output format: stable markdown sections beginning with `## 🧪 CTX Run`.

```text
/ctx-run npm run test:auth
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

### `ctx stats [--history <n>]`

- OpenCode: `/ctx-stats`
- What it does: prints the latest local telemetry snapshot, or an aggregate gain-style report when `--history` is greater than `1`.

```bash
ctx stats
ctx stats --history 20
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

## Toolbooks OpenCode-only

Toolbooks are scoped graph memory for large CLI manuals, runbooks, and cheat sheets. They are meant to replace putting huge command manuals into `AGENTS.md`.

### `/ctx-toolbook-import <name> <file>`

- CLI equivalent: none as a public command; internally uses `ctx memory import --scope toolbook:<name> --source toolbook`.
- What it does: imports a markdown manual into a scoped toolbook.

```text
/ctx-toolbook-import glab docs/glab.md
```

### `/ctx-toolbook-search <name> "<query>"`

- CLI equivalent: none as a public command; internally uses `ctx memory search <query> --scope toolbook:<name>`.
- What it does: retrieves only the relevant manual entries for a question.

```text
/ctx-toolbook-search glab "merge request create"
```

### `/ctx-toolbook-list <name>`

- CLI equivalent: none as a public command; internally uses `ctx memory list --scope toolbook:<name>`.
- What it does: lists stored directives for one toolbook.

```text
/ctx-toolbook-list glab
```

### `/ctx-toolbook-pack <name> "<task>"`

- CLI equivalent: none as a public command; internally combines toolbook memory search with `ctx pack`.
- What it does: retrieves relevant toolbook guidance and a task context pack in one OpenCode workflow.

```text
/ctx-toolbook-pack glab "create merge request for auth fix"
```

## Learning OpenCode-only

### `/ctx-learn <key> "<body>"`

- CLI equivalent: none as a public command; internally uses `ctx memory set <key> <body> --scope project --source learned`.
- What it does: stores a reusable lesson learned during real work so future `/ctx-memory-search` and `/ctx-pack` calls can find it.

```text
/ctx-learn auth.refresh_regression "When auth refresh fails, check token rotation and stale session flags first."
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
- Prefer `/ctx-run <shell command>` for the normal OpenCode workflow. Use `/ctx-prune-logs` when you already have raw output or want pruning only.

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
/ctx-plan fix auth refresh regression
/ctx-retrieve refresh token auth failure
/ctx-pack fix auth refresh regression
/ctx-compare fix auth refresh regression
/ctx-prune-logs npm run test:auth
/ctx-learn auth.refresh_regression "Check token rotation and stale session flags before changing tests."
/ctx-stats
```
