# CTX Architecture

CTX is an OpenCode-first local context runtime. It does not replace the host agent. OpenCode owns the model, provider, plugins, and normal session behavior; CTX supplies local graph memory, retrieval, pruning, packing, benchmarks, diagnostics, and MCP tools.

## Runtime Pipeline

1. `ctx-intake`: normalize user/task intent.
2. `ctx-prune`: compact noisy logs and diffs.
3. `ctx-ast`: extract symbols and structural slices.
4. `ctx-graph`: persist files, snippets, symbols, memory directives, failures, decisions, and run metadata in SQLite/FTS.
5. `ctx-semantic`: rank relevant chunks with local fallback behavior.
6. `ctx-pack`: assemble compact task context under a budget.
7. `ctx-mcp`: expose local tools over stdio and localhost HTTP JSON-RPC.
8. `ctx-cli`: provide bootstrap/runtime commands and generate OpenCode assets.

## Persistence

| Data | Location |
|---|---|
| Config | `.ctx/config.toml` |
| Graph | `.ctx/graph.db` |
| Packs | `.ctx/packs/` |
| Stats | `.ctx/stats/latest.json` |
| Audit | `.ctx/audit.log` |
| OpenCode config | `opencode.json` |
| OpenCode commands | `.opencode/commands/*.md` |
| OpenCode instructions | `.opencode/instructions/ctx-host-first.md` |

## OpenCode Integration

`ctx opencode install` does three things:

- registers the current `ctx` binary path plus `--repo-root <repo> mcp stdio` as a local OpenCode MCP server
- generates `/ctx-*` command files under `.opencode/commands/`
- adds host-first instructions that tell OpenCode to prefer CTX graph/memory/retrieval before broad file dumping

Generated OpenCode commands do not pin a model or agent.

## Graph Memory

Graph memory stores project habits as structured directives. Existing markdown files such as `AGENTS.md`, `CLAUDE.md`, `CODEX.md`, and `.github/copilot-instructions.md` can seed the graph, but daily retrieval should query directives by topic instead of reinjecting full markdown files.

## Security Boundary

- CTX is local-first.
- MCP stdio is the preferred host transport.
- HTTP JSON-RPC binds to `127.0.0.1` for local debugging.
- Sensitive attachments are blocked before packing.
- Privacy decisions and pack summaries are recorded locally.
