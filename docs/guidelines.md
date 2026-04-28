# CTX Product Guidelines

## North Star

CTX must feel like a local context runtime inside OpenCode, not a second agent launcher.

The target user experience is:

- open `opencode`
- stay inside `opencode`
- use CTX through `/ctx-*` commands and local MCP tools
- keep the current OpenCode model, provider, plugins, and agent behavior
- avoid a second terminal for daily CTX usage

## Product Rules

- OpenCode-first is the highest-priority integration target.
- OpenCode-native commands are the product surface.
- Daily usage must happen inside OpenCode after bootstrap.
- The CLI should focus on runtime setup, indexing, MCP, benchmarks, and OpenCode asset generation.
- Treat wrapper-first UX as legacy.
- Do not reintroduce `ctx wrap`, `ctx opencode run`, or host-launcher style commands.
- CTX must not override host model/provider selection by default.
- Graph memory should replace repeated full markdown rereads when possible.
- Compatibility markdown files such as `AGENTS.md`, `CLAUDE.md`, `CODEX.md`, and Copilot instructions should remain valid graph-memory seed inputs.

## OpenCode Rules

- Prefer local MCP stdio for tool access.
- Prefer `.opencode/commands/*.md` for explicit user-facing commands.
- Keep generated command descriptions short and discoverable.
- Keep project integration assets repo-local and reviewable.
- Use `/ctx` as the command center and quickstart surface.

## Definition Of Success

CTX is ready for OpenCode when:

- a user can clone a repo, run `ctx opencode install`, open `opencode`, and use CTX without leaving the TUI;
- graph memory and retrieval reduce token usage without losing relevant project rules;
- generated OpenCode assets are clear enough to inspect and commit;
- docs, tests, and release scripts all describe the same OpenCode-first product.
