# CTX Inside OpenCode

## Goal

Make CTX live inside OpenCode so the user can keep using OpenCode normally while CTX supplies graph memory, retrieval, pruning, compact context, benchmark utilities, diagnostics, and local MCP tools.

## Current Implementation

`ctx opencode install` currently writes or updates:

- `opencode.json`
- `.opencode/commands/*.md`
- `.opencode/instructions/ctx-host-first.md`

The generated config registers CTX as a local MCP server launched with:

```bash
/absolute/path/to/ctx --repo-root <repo> mcp stdio
```

The generated commands expose the current CTX feature surface as `/ctx-*` commands inside OpenCode.
The bootstrap and graph-memory flow still support compatibility seed files such as `AGENTS.md`, `CLAUDE.md`, `CODEX.md`, and `.github/copilot-instructions.md`.

Users should open `opencode` after bootstrap and keep normal work inside the OpenCode TUI.

## User Flow

```bash
ctx init
ctx index
ctx opencode install
opencode
```

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
- context: `/ctx-pack`, `/ctx-ask`, `/ctx-hook`, `/ctx-explain`, `/ctx-retrieve`, `/ctx-graph-query`
- pruning: `/ctx-prune-logs <shell command>`, `/ctx-prune-diff`
- memory: `/ctx-memory-bootstrap`, `/ctx-memory-import`, `/ctx-memory-search`, `/ctx-memory-list`, `/ctx-memory-get`, `/ctx-memory-set`, `/ctx-memory-delete`, `/ctx-memory-export`
- benchmarks: `/ctx-benchmark-memory-ab`, `/ctx-benchmark-memory-suite`, `/ctx-stats`
- MCP/bootstrap: `/ctx-mcp-stdio`, `/ctx-mcp-serve`, `/ctx-mcp-config-opencode`, `/ctx-opencode-install`

## Design Constraints

- Do not pin model or agent in generated commands.
- Do not ask users to use wrapper commands for daily work.
- Keep all CTX data local unless a future explicit opt-in remote feature is added.
- Prefer graph memory over repeated full markdown instruction injection.

## Remaining Work

- Add public screenshots and video assets after manual OpenCode validation.
- Validate the same flow on an external real-world repository.
- Continue improving automatic host use of MCP tools where OpenCode exposes deeper hooks.
