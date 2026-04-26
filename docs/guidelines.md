# CTX Product Guidelines

## North Star

CTX must behave like a local context runtime plugin that lives inside the host agent CLI, starting with OpenCode.

The user experience target is:

- open `opencode`
- stay inside `opencode`
- use CTX features from inside the OpenCode TUI
- keep the current OpenCode model, provider, and agent selection
- avoid a second terminal or wrapper-centric workflow for daily usage

## Product Rules

- OpenCode-first is the highest-priority integration target.
- Codex and Claude Code should follow as native host integrations, not as revived wrapper-style launchers.
- Daily usage must happen inside the host CLI, not through removed wrapper-style public commands.
- The public CLI should stay focused on runtime/bootstrap capabilities and host-native integration.
- Treat wrapper-first UX as legacy, not primary.
- The host CLI owns model/provider selection; CTX must not override it by default.
- CTX should provide retrieval, graph memory, pruning, benchmarking, and diagnostics as host-native commands or tools.
- Implicit usage is preferred over explicit host-side CTX commands whenever the host supports it.

## OpenCode-Specific Rules

- Prefer OpenCode MCP integration for tool access and background retrieval.
- Prefer OpenCode commands in `opencode.json` or `.opencode/commands/` for explicit CTX entrypoints inside the TUI.
- Use project-local integration assets so a repository can be cloned and opened directly in OpenCode.
- Generated command descriptions must be short and useful because OpenCode shows them in the TUI command list.
- Generated commands should preserve the current OpenCode agent unless a command explicitly opts into `plan` or subtask mode.

## Technical Guidelines

- Do not require a second terminal for normal CTX usage once OpenCode integration is installed.
- Do not require users to pipe shell output manually when the same behavior can be exposed through MCP tools or OpenCode commands.
- Keep the CTX runtime local-first: `ctx mcp stdio` is the preferred transport for OpenCode integration.
- Prefer committed project files over hidden one-off local setup whenever possible.
- Preserve compatibility with `.ctx/` storage and the existing graph/memory/runtime model.
- Do not reintroduce wrapper-first public UX for hosts when native integration points are available.

## UX Guidelines

- The user should be able to type normal prompts in OpenCode and benefit from CTX automatically.
- The user should also have explicit CTX commands inside OpenCode for graph, memory, benchmark, doctor, and context/debug actions.
- CTX commands inside OpenCode should feel like first-class commands, not hacks around shelling out.
- The integration should be understandable from the repo alone: README, install docs, and project files should make the workflow obvious.

## Definition Of Success

CTX is successful for OpenCode when:

- a user opens `opencode` in a CTX-enabled repo and can use CTX without leaving the TUI;
- the model selected in OpenCode remains the one doing the work;
- graph memory and retrieval reduce token usage without asking the user to manage a second interface;
- legacy wrapper-first workflows stay out of the primary product path.
