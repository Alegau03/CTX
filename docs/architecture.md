# CTX Architecture (Initial Implementation)

## Pipeline

1. Intake (`ctx-intake`)
2. Deterministic pruning (`ctx-prune`)
3. Context packing (`ctx-pack`)
4. Local graph enrichment (`ctx-graph`)
5. Agent wrapper integration (`ctx-cli` + `ctx-adapters`)
6. Local telemetry (`ctx-telemetry`)

## Persistence

- Config: `.ctx/config.toml`
- Graph: `.ctx/graph.db` (SQLite + FTS tables scaffolded)
- Stats: `.ctx/stats/latest.json`
- Audit: `.ctx/audit.log`

## Security Model

- Local-first execution
- No mandatory network calls
- MCP scaffold is localhost-oriented
- Sensitive path filtering and richer policy enforcement scheduled in next iteration
