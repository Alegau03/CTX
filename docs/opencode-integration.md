# CTX Inside OpenCode

## Goal

Make CTX live inside OpenCode so the user can keep using OpenCode normally while CTX supplies graph memory, retrieval, pruning, compact context, benchmark utilities, diagnostics, and local MCP tools.

## Current Implementation

`ctx opencode install` currently writes or updates:

- `opencode.json`
- `.opencode/commands/*.md`
- `.opencode/instructions/ctx-host-first.md`
- `.opencode/tui.json` when the `full` profile is installed
- `.opencode/plugins/ctx-dashboard.tsx` when the `full` profile is installed
- `.opencode/package.json` when the `full` profile is installed

The generated config registers CTX as a local MCP server launched with:

```bash
/absolute/path/to/ctx --repo-root <repo> mcp stdio
```

The generated commands expose the current CTX feature surface as `/ctx-*` commands inside OpenCode.
The installer supports two profiles:

- `full` (default): complete CTX command surface plus a live `CTX Dashboard` panel in the OpenCode right sidebar
- `core`: lean daily workflow with only the smallest OpenCode slash-command set

The generated `/ctx` command center and `ctx-host-first.md` instructions reflect the installed profile.
The `full` profile also provisions a TUI sidebar plugin that refreshes CTX savings, cache, top-win, and artifact metrics directly in the OpenCode right rail.
The bootstrap and graph-memory flow still support compatibility seed files such as `AGENTS.md`, `CLAUDE.md`, `CODEX.md`, and `.github/copilot-instructions.md`.
The generated command files prefer deterministic CTX-owned execution: they call the absolute `ctx` binary with `--repo-root`, lean on `--json` where it reduces host chatter, and ask OpenCode only for a narrow result-first explanation of the command output.

Users should open `opencode` after bootstrap and keep normal work inside the OpenCode TUI.

## User Flow

```bash
ctx init
ctx index
ctx opencode install
opencode
```

Lean alternative:

```bash
ctx opencode install --profile core
opencode
```

If you install the default `full` profile instead, OpenCode should also show the live `CTX Dashboard` in the right sidebar after the repo reloads.

Inside OpenCode:

```text
/ctx
/ctx-memory-bootstrap
/ctx-memory-search auth
/ctx-pack fix refresh token bug
```

## Command Surface

The OpenCode integration covers:

- setup: `/ctx`, `/ctx-help`, `/ctx-doctor`, `/ctx-init`, `/ctx-index`, `/ctx-reindex`
- context: `/ctx-pack`, `/ctx-ask`, `/ctx-hook`, `/ctx-explain`, `/ctx-retrieve`, `/ctx-read`, `/ctx-graph-query`
- pruning: `/ctx-run <shell command>`, `/ctx-prune-logs <shell command>`, `/ctx-prune-diff`
- memory: `/ctx-memory-bootstrap`, `/ctx-memory-import`, `/ctx-memory-search`, `/ctx-memory-list`, `/ctx-memory-get`, `/ctx-memory-set`, `/ctx-memory-delete`, `/ctx-memory-export`
- benchmarks: `/ctx-dashboard`, `/ctx-gain`, `/ctx-benchmark-memory-ab`, `/ctx-benchmark-memory-suite`, `/ctx-stats`

`/ctx-dashboard` is meant to be the quick local control panel: it highlights savings, cache ratios, top wins, and audit-backed runtime telemetry in one result-first snapshot.
- the `full` profile right-sidebar panel shows the same runtime story live without spending another chat turn, but keeps the layout intentionally tighter than the in-thread command output
- MCP/bootstrap: `/ctx-mcp-stdio`, `/ctx-mcp-serve`, `/ctx-mcp-config-opencode`, `/ctx-opencode-install`

The core profile keeps only:

- `/ctx`
- `/ctx-doctor`
- `/ctx-plan <task>`
- `/ctx-retrieve <query>`
- `/ctx-pack <task>`
- `/ctx-run <shell command>`
- `/ctx-prune-logs <shell command>`
- `/ctx-stats`
- `/ctx-gain`

The core profile intentionally does not install the right-sidebar dashboard plugin.

## Design Constraints

- Do not pin model or agent in generated commands.
- Do not ask users to use wrapper commands for daily work.
- Keep all CTX data local unless a future explicit opt-in remote feature is added.
- Prefer graph memory over repeated full markdown instruction injection.
- Treat `ctx doctor` output as the source of truth for readiness. `ready: true` means CTX is operational; `next:` is then a recommended workflow step, not a blocker.

## Validation Notes

- MCP health is validated automatically with:

```bash
cd <repo>
opencode mcp list --print-logs --log-level DEBUG --pure
```

- A healthy run should show `ctx connected` and `toolCount=13`.
- Headless `opencode run` is useful for validating `/ctx` menu rendering and MCP bootstrap, but the interactive TUI remains the source of truth for the full slash-command UX.
- Bootstrap, `ctx opencode install`, MCP handshake, and basic retrieval/pack flow were also validated on the public `charmbracelet/glow` repository.

## Remaining Work

- Add public screenshots and video assets after manual OpenCode validation.
- Run the benchmark flow on at least one external real-world repository.
- Continue improving automatic host use of MCP tools where OpenCode exposes deeper hooks.
