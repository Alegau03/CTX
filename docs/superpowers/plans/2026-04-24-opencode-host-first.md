# 2026-04-24 OpenCode Host-First Pivot Plan

## Status

This is a historical pivot plan.

Current source of truth:

- `README.md`
- `docs/opencode-integration.md`
- `docs/superpowers/plans/2026-04-25-final-release-roadmap.md`

Current reality:

- the public wrapper-style CLI entrypoints have already been removed;
- OpenCode-first bootstrap is implemented through `ctx opencode install`;
- this document is kept to explain the pivot, not to describe the current public CLI surface.

## Objective

Reorient CTX so that everything already built can be used from inside OpenCode. The user should open `opencode`, stay in the TUI, and access CTX features there. The wrapper-first public CLI described during the transition has since been removed.

## External Constraints

This plan is based on current OpenCode docs and repository behavior:

- OpenCode supports project config via `opencode.json`.
- OpenCode loads project-local directories like `.opencode/commands/`.
- OpenCode supports local MCP servers through config.
- OpenCode exposes MCP tools directly to the model.
- OpenCode commands have short descriptions shown in the TUI.
- OpenCode can expose MCP prompts as commands.

## Repo Reality Check

### What we already have

- `ctx mcp stdio` with a meaningful CTX tool surface
- graph memory CRUD and markdown import/export
- benchmark memory-ab
- diagnostics and local stats
- OpenCode adapter compatibility path

### What conflicts with the target

- the fully automatic host-native OpenCode experience is still incomplete
- future non-OpenCode hosts still need native bootstrap plans

## Workstreams

### Workstream A: Product and docs pivot

- Rewrite docs to make OpenCode-first the primary story.
- Mark wrappers as transitional.
- Add architecture and guideline docs for host-native integration.

Deliverables:

- `docs/guidelines.md`
- `docs/opencode-integration.md`
- updated `README.md`
- updated `docs/architecture.md`
- updated `guide.md`
- updated `docs/install.md`

### Workstream B: OpenCode bootstrap

- Add a CTX command that writes or patches project-local OpenCode integration files.
- Candidate names:
- `ctx mcp config opencode`
- `ctx opencode install`
- `ctx integrate opencode`

Preferred outcome:

- one command creates all OpenCode integration assets for the repo
- after that, daily usage happens inside OpenCode

### Workstream C: OpenCode-native command surface

- Map CTX features to OpenCode commands:
- `/ctx-doctor`
- `/ctx-pack`
- `/ctx-graph-query`
- `/ctx-memory-set`
- `/ctx-memory-list`
- `/ctx-memory-import`
- `/ctx-memory-export`
- `/ctx-benchmark-memory-ab`

- Keep descriptions short because they appear in the TUI.
- Keep command templates thin and push real work into CTX MCP tools.

### Workstream D: Automatic tool usage

- Add instructions/rules so OpenCode uses CTX automatically:
- prefer graph memory over large markdown habit files
- use compact diff and retrieval tools before broad scans
- avoid reinjecting full files when CTX tools already expose structure

### Workstream E: Deprecation path

- Keep wrapper-style public CLI removed.
- Remove stale wrapper-era wording from historical docs and examples.
- Keep only internal backend code that still matters for future host-native integrations.

## Detailed Acceptance Criteria

### Acceptance 1

Opening `opencode` in a CTX-enabled repo should expose CTX tools through MCP without any second terminal.

### Acceptance 2

CTX explicit commands should be discoverable from inside OpenCode with short descriptions.

### Acceptance 3

Normal OpenCode prompts should benefit from CTX graph memory and retrieval without requiring wrapper prompts.

### Acceptance 4

The OpenCode-selected model and agent should remain in control; CTX should not silently replace host model selection.

### Acceptance 5

The repo documentation should teach OpenCode-native usage first and wrappers second.

## Test Plan

### Stage 1: Spec tests

- Add ignored acceptance tests in `crates/ctx-cli/tests/` describing the final OpenCode-native behavior.
- Add active docs tests ensuring the repo now names OpenCode-first as top priority.

### Stage 2: Bootstrap tests

- Verify generated `opencode.json` contains CTX MCP server config.
- Verify generated `.opencode/commands/` files exist and cover the expected command surface.

### Stage 3: Runtime tests

- Verify `ctx mcp stdio` works under the generated OpenCode config.
- Verify OpenCode-native command templates route to CTX tools, not wrapper prompts.

### Stage 4: Cleanup tests

- Ensure wrapper-style public commands stay removed.
- Ensure OpenCode-native docs remain primary.

## Recommended Execution Order

1. Finish the docs pivot and spec tests.
2. Implement OpenCode preset/bootstrap generation.
3. Add command generation under `.opencode/commands/`.
4. Add OpenCode-specific MCP config generation.
5. Add instruction/rules generation for automatic CTX usage.
6. Add full OpenCode host-native acceptance tests.
7. Demote wrappers in docs.
8. Only then resume lower-priority roadmap tasks.
