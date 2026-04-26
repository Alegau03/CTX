# CTX Inside OpenCode

## Goal

Make CTX live inside OpenCode so the user stays in the OpenCode TUI for normal work while CTX supplies graph memory, retrieval, pruning, compact context, benchmark utilities, and diagnostics.

This document is the product and architecture target. It is intentionally stricter than the current implementation.

## Current Truth

Today CTX already has the core runtime pieces:

- local graph and memory
- pack/prune/retrieval pipeline
- MCP server over stdio and HTTP JSON-RPC
- diagnostics and local stats
- repo-local OpenCode bootstrap and generated command files

What is still missing is the fully automatic host-native OpenCode experience. The primary docs now point to OpenCode-first usage, but automatic host-first behavior is still incomplete.

## OpenCode Capabilities We Can Build On

OpenCode already supports the primitives CTX needs:

- project config via `opencode.json`
- project command files in `.opencode/commands/`
- local MCP servers declared in OpenCode config
- MCP tools available directly to the LLM
- MCP prompts exposed as commands
- per-agent tool enable/disable controls

This means CTX does not need to replace OpenCode. CTX should become an OpenCode-local subsystem.

## Target UX

### Daily usage

The user:

- should open `opencode`
- opens `opencode`
- uses their normal OpenCode model and agent
- writes prompts normally
- benefits from CTX-backed retrieval and graph memory automatically

### Explicit CTX usage inside OpenCode

The user can also run CTX-specific commands from inside the OpenCode TUI, for example:

- `/ctx-doctor`
- `/ctx-memory-set`
- `/ctx-memory-list`
- `/ctx-graph-query`
- `/ctx-pack`
- `/ctx-benchmark-memory-ab`

These commands should exist as OpenCode-native commands, not as instructions to open another terminal.

## Architecture

### Layer 1: CTX runtime

Keep the existing Rust runtime and `.ctx/` persistence model.

No major architectural rewrite is needed here. The graph, memory, pruning, packing, stats, and audit layers stay the same.

### Layer 2: OpenCode MCP bridge

OpenCode should connect to CTX through `ctx mcp stdio` using project-local OpenCode config.

This makes CTX tools available directly to the OpenCode model without wrapper prompts.

### Layer 3: OpenCode command layer

Generate or maintain project-local command definitions under `.opencode/commands/` or `opencode.json`.

These commands do two things:

- provide explicit entrypoints for CTX features inside the OpenCode UI
- document short descriptions so the OpenCode command menu is understandable

### Layer 4: OpenCode rules/instructions

Add project-local instructions that teach OpenCode when to use CTX tools automatically:

- query graph memory instead of rereading large markdown files
- call compact diff/prune/retrieval tools before scanning large logs
- prefer CTX graph memory over repeated full-file prompt injection

Current implementation:

- `ctx opencode install` now writes `.opencode/instructions/ctx-host-first.md`
- `ctx opencode install` now merges `opencode.json.instructions` so OpenCode loads:
  - `docs/guidelines.md`
  - `docs/security.md`
  - `.opencode/instructions/ctx-host-first.md`
- `scripts/release/opencode-smoke.sh` now validates the generated OpenCode bootstrap assets during release smoke

### Layer 5: Compatibility layer

Do not reintroduce wrapper-first public UX for OpenCode.

Future non-OpenCode integrations should prefer native host bootstraps instead of reviving wrapper commands.

## Required Surface Area Inside OpenCode

The OpenCode-native integration must cover all current CTX capabilities that matter to daily usage:

- compact context packing
- retrieval
- graph queries
- memory bootstrap/search/CRUD
- markdown import/export for memory
- benchmark memory-ab
- doctor/status
- recent decisions and related failures
- security-safe attachment blocking behavior

## Implementation Plan

### Phase 1: Repo pivot

- Update README, install docs, architecture docs, and guide to say OpenCode-first clearly.
- Add project guidelines that define wrapper-first UX as legacy, not primary.
- Add acceptance/spec tests for the OpenCode-native target.

### Phase 2: OpenCode config preset generation

- Add a CTX command that generates or syncs OpenCode integration assets into the repo.
- Output:
- `opencode.json` or merged patch instructions
- `.opencode/commands/*.md`
- optional `.opencode/rules` or instruction file references

Acceptance:

- a repo can become CTX-enabled for OpenCode with a single bootstrap action;
- after bootstrap, the user works from inside OpenCode.

Current status:

- `ctx mcp config opencode` is implemented as the first bootstrap primitive.
- `ctx opencode install` now bootstraps `opencode.json` and `.opencode/commands/` in the repo.
- the remaining gap is stronger opportunistic CTX usage for normal prompts.

### Phase 3: MCP-first OpenCode integration

- Keep `ctx mcp config opencode` as the low-level config primitive.
- Keep `ctx opencode install` as the repo-local bootstrap command.
- Ensure generated config uses `ctx --repo-root <repo> mcp stdio`.
- Scope tool enablement carefully to avoid flooding context.

Acceptance:

- OpenCode can call CTX tools directly from the TUI without external wrappers.

### Phase 4: Command surface inside OpenCode

- Create CTX command definitions with short descriptions and argument hints.
- Cover at least:
- doctor
- pack
- graph query
- memory bootstrap/search/set/get/list/delete
- memory import/export
- benchmark memory-ab

Acceptance:

- OpenCode shows CTX commands in its command discovery UI with useful descriptions.

### Phase 5: Automatic usage rules

- Add instructions/rules so OpenCode uses CTX automatically:
- prefer graph memory over large markdown habit files
- use compact diff and retrieval tools before broad scans
- avoid reinjecting full files when CTX tools already expose structure

Acceptance:

- a normal OpenCode prompt in a CTX-enabled repo uses CTX tools opportunistically without wrapper prompts.

### Phase 6: Cleanup after wrapper removal

- Remove stale wrapper-first wording from docs and examples.
- Keep the public CLI focused on runtime/bootstrap capabilities plus OpenCode-native usage.
- Ensure future host integrations are specified as native host work, not wrapper revivals.

Acceptance:

- the primary docs no longer teach wrapper-first usage for OpenCode;
- the public CLI no longer exposes wrapper-first daily UX.

## Test Strategy

### Active tests now

- docs/spec tests that assert the repo is explicitly OpenCode-first
- config generation tests once the preset/bootstrap exists
- MCP preset tests for OpenCode once implemented

### Acceptance tests to add during implementation

- generated OpenCode config includes CTX MCP server
- generated OpenCode commands cover the CTX command surface
- OpenCode-native flow works without legacy wrapper-first public commands
- OpenCode keeps the host-selected model while CTX tools provide context
- graph memory, benchmark, and doctor all remain accessible from inside OpenCode

## Done Criteria

The OpenCode-native pivot is complete when:

- a user can clone a CTX-enabled repo, open `opencode`, and use CTX without leaving the TUI;
- the documented primary workflow no longer depends on wrappers;
- legacy wrapper-first public commands stay removed;
- the acceptance suite proves that the full CTX surface is reachable inside OpenCode.
