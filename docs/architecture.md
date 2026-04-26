# CTX Architecture

## Product Direction

CTX is not supposed to become a parallel agent launcher long-term.

The target product shape is:

- host CLI stays primary
- CTX runs locally behind the host CLI
- host model/provider/agent selection stays intact
- CTX contributes graph memory, retrieval, pruning, compact context, benchmark, and diagnostics through host-native commands and tools

OpenCode is the first-class host target.
Codex and Claude Code now have native bootstrap paths built on the same local runtime.

Historical wrapper-oriented plans still exist under `docs/superpowers/plans/` for implementation traceability, but they are not the current product source of truth.

## Pipeline

1. Intake (`ctx-intake`)
2. Deterministic pruning (`ctx-prune`)
3. Context packing (`ctx-pack`)
4. Local graph enrichment (`ctx-graph`)
5. Host integration (`ctx-cli` + `ctx-mcp` + host-native command surfaces)
6. Local telemetry (`ctx-telemetry`)

## Persistence

- Config: `.ctx/config.toml`
- Graph: `.ctx/graph.db` (SQLite + FTS tables scaffolded)
- Stats: `.ctx/stats/latest.json`
- Audit: `.ctx/audit.log`

## Security Model

- Local-first execution
- No mandatory network calls
- MCP stdio is the preferred host integration transport
- MCP HTTP remains localhost-oriented
- Sensitive path filtering is already enforced
- OpenCode integration should inherit the same local-first trust boundary
