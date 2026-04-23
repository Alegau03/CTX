# CTX - Context Runtime Engine for Coding Agents

Local-first context runtime che riduce rumore nel prompt e preserva segnale utile per coding agents.

## Current Status

Implementazione attiva basata su:
- `CTX_description.pdf`
- `docs/superpowers/plans/2026-04-23-ctx-runtime-engine.md`

Fondamenta già implementate:
- workspace Rust multi-crate
- CLI estesa (`init`, `index`, `reindex`, `graph build/query/rebuild`, `prune`, `pack`, `explain`, `retrieve`, wrappers, `stats`, `mcp serve`)
- config `.ctx/config.toml`
- pruning logs/diff
- context packing con budget e priorità
- graph SQLite con simboli, edge, snippet FTS, failure/decision memory
- retrieval ibrido (graph + FTS + semantic ranking)
- adapter runtime reali per `codex` e `opencode` (invocazione CLI + fallback)
- MCP server locale operativo su `/rpc`
- guardrail sicurezza (blocchi su file sensibili) + audit log

## Workspace Layout

- `crates/ctx-cli`: binary `ctx`
- `crates/ctx-core`: orchestrazione pipeline
- `crates/ctx-config`: config/bootstrap
- `crates/ctx-prune`: pruning deterministico logs/diff
- `crates/ctx-pack`: context packer
- `crates/ctx-graph`: storage/query SQLite + FTS
- `crates/ctx-intake`: intent detection
- `crates/ctx-ast`: parsing/slicing strutturale (tree-sitter + fallback)
- `crates/ctx-semantic`: ranking ibrido
- `crates/ctx-telemetry`: stats/benchmark summary
- `crates/ctx-adapters`: preparazione + invocazione adapter CLI
- `crates/ctx-hooks`: hook utility
- `crates/ctx-mcp`: MCP server locale
- `crates/ctx-token`: token estimator

## MCP Server: A Cosa Serve E Cosa Fa

Serve a esporre il motore CTX come servizio locale interrogabile da agent/tooling esterni (es. Claude Code via MCP), evitando di incollare manualmente file e log nel prompt.

Cosa fa in pratica:
- gira in locale su `127.0.0.1` (nessun cloud obbligatorio)
- espone endpoint RPC `POST /rpc`
- supporta metodi MCP-like:
  - `initialize`
  - `ping`
  - `tools/list`
  - `tools/call`
  - `resources/list`
  - `resources/read`
- tool disponibili:
  - `get_relevant_context`
  - `project_map`
  - `search_symbols`
  - `related_failures`
  - `recent_decisions`
  - `get_compact_diff`

Comando avvio:
```bash
ctx mcp serve --port 8765
```

## Quick Start

1. Verifica completa:
```bash
cargo test --workspace
```

2. Init progetto:
```bash
ctx init
```

3. Indicizzazione:
```bash
ctx index
```

4. Pack compatto:
```bash
ctx pack "fix failing pytest in auth" --json --attach /tmp/fail.txt
```

5. Retrieval ibrido:
```bash
ctx retrieve "refresh token auth failure" --limit 5
```

## Test

### 1) Full Validation

Command:
```bash
cargo test --workspace
```

Cosa fa:
- Esegue unit/integration/e2e su tutte le crate.

Comportamento atteso:
- Tutti i test in stato `ok`.

### 2) CLI Commands And Functional Tests

| Command | Cosa fa / Deve fare | Come testarla | Risultato atteso |
|---|---|---|---|
| `ctx init` | Inizializza runtime locale (`.ctx/config.toml`, dirs runtime, `audit.log`, graph db) | `ctx init` | Output `initialized: .../.ctx/config.toml` |
| `ctx index` | Indicizza file codice e simboli nel graph | crea `src/auth.rs`, poi `ctx index` | `indexed_files: N` con `N >= 1` |
| `ctx reindex src tests` | Reindicizza path specifici | `ctx reindex src tests` | `indexed_files: N` |
| `ctx graph build` | Rebuild graph globale | `ctx graph build` | `graph_build_indexed_files: N` |
| `ctx graph rebuild` | Alias di rebuild graph | `ctx graph rebuild` | `graph_build_indexed_files: N` |
| `ctx graph query auth` | Query file path nel graph | `ctx graph query auth` | path rilevanti (es. `src/auth.rs`) |
| `ctx prune logs` | Pulisce rumore log mantenendo segnali critici | `printf 'PASS ok\nERROR broken\n' \| ctx prune logs` | include `ERROR broken`, rimuove rumore |
| `ctx prune diff --query "refresh token"` | Tiene hunk diff pertinenti alla query | `cat diff.patch \| ctx prune diff --query "refresh token"` | diff compatto query-relevant |
| `ctx help` | Stampa guida completa in inglese: cosa fa ogni comando + esempio di utilizzo | `ctx help` | output `CTX Command Guide` con esempi per tutti i comandi |
| `ctx pack "..." --json --attach file` | Crea context pack a budget con priorità | `ctx pack "fix auth" --json --attach /tmp/fail.txt` | JSON con `packed_tokens`, `reduction_pct`, `compact_context` |
| `ctx explain "..."` | Mostra intent e contesto probabile | `ctx explain "fix failing pytest"` | include `intent: debug` |
| `ctx retrieve "..." --limit N` | Retrieval ibrido ordinato per score | `ctx retrieve "refresh token auth" --limit 3` | lista hit con score e contenuto rilevante |
| `ctx codex "..."` | Costruisce context pack e invoca Codex CLI con prompt + contesto compattato | `ctx codex "review risky diff"` | se `codex` è in PATH: esecuzione reale; altrimenti fallback con prompt pronto |
| `ctx claude "..."` | Wrapper adapter claude con contesto compattato | `ctx claude "explain flaky test"` | output con prefisso `adapter=claude` |
| `ctx opencode run "..."` | Costruisce context pack e invoca OpenCode CLI (`opencode run`) con prompt + contesto compattato | `ctx opencode run "explain this diff"` | se `opencode` è in PATH: esecuzione reale; altrimenti fallback con prompt pronto |
| `ctx stats` | Legge metriche locali ultimo run | dopo `ctx pack ...`, esegui `ctx stats` | JSON con `original_tokens`, `packed_tokens`, `reduction_pct` |
| `ctx pack "..." --attach .env` | Security guardrail: blocca allegati sensibili | `ctx pack "fix auth" --attach .env` | errore esplicito di blocco |
| `ctx mcp serve --port 8765 --once` | Avvia server MCP e gestisce una sola request | avvia e invia una RPC | risposta valida e chiusura pulita |

### 3) MCP RPC Smoke

Initialize:
```bash
curl -s http://127.0.0.1:8765/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
```

Tools list:
```bash
curl -s http://127.0.0.1:8765/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
```

Tool call (`get_relevant_context`):
```bash
curl -s http://127.0.0.1:8765/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_relevant_context","arguments":{"query":"fix auth failure","budget":120}}}'
```

Atteso:
- `initialize` restituisce `serverInfo.name = "ctx-mcp"`
- `tools/list` contiene i 6 tool previsti
- `tools/call` restituisce pack con `packed_tokens > 0`

### 4) Module-Level Tests

| Feature | Command | Cosa verifica |
|---|---|---|
| Config | `cargo test -p ctx-config` | parsing, defaults, validazione, bootstrap `.ctx` |
| Pruning | `cargo test -p ctx-prune` | dedup, noise filtering, root-cause preservation |
| Intake | `cargo test -p ctx-intake` | intent detection e normalizzazione query |
| Token | `cargo test -p ctx-token` | stima token deterministica |
| AST | `cargo test -p ctx-ast` | estrazione simboli tree-sitter + slicing |
| Semantic | `cargo test -p ctx-semantic` | formula + ranking ibrido + dedup/adaptive threshold |
| Graph | `cargo test -p ctx-graph` | simboli/edge/snippet FTS/failure/decision |
| Core | `cargo test -p ctx-core` | index+pack+retrieval+guardrail |
| CLI | `cargo test -p ctx-cli` | e2e comandi + mcp serve |
| MCP | `cargo test -p ctx-mcp` | roundtrip RPC server |
| Telemetry | `cargo test -p ctx-telemetry` | stats e benchmark summary/report |

## Roadmap

Stato task dal piano (`docs/superpowers/plans/2026-04-23-ctx-runtime-engine.md`):

- [x] Task 1: bootstrap workspace e skeleton
- [x] Task 2: CLI surface completa
- [x] Task 3: config system `.ctx/config.toml`
- [x] Task 4: query intake + signal collection (base)
- [x] Task 5: heuristic pruner + parser base logs/diff
- [x] Task 6: syntax & structure analyzer (tree-sitter + slicing)
- [x] Task 7: semantic relevance engine (ranking ibrido locale)
- [x] Task 8: knowledge graph engine (schema, simboli, edge, failure, decision, snippets FTS)
- [x] Task 9: retrieval layer ibrido (graph + FTS + semantic)
- [x] Task 10: context rewriter/packer (baseline priorità+budget)
- [x] Task 11: invocation + telemetry locale (codex/opencode real invocation + fallback, claude pending)
- [x] Task 12: integration modes principali (wrapper/pipe/index/rebuild)
- [x] Task 13: MCP server operativo
- [x] Task 14: sicurezza/privacy baseline (guardrail + audit)
- [ ] Task 15: packaging/install distribuzione completa (in progress)
- [ ] Task 16: benchmark harness full end-to-end (in progress)
- [ ] Task 17: fase MVP formalizzata con gate per release
- [ ] Task 18: demo/virality assets completi (GIF + script pubblico)
- [ ] Task 19: future extensions backlog operativo e prioritizzato

## Execution Order To Final GitHub Release

1. Chiudere `Task 11` completamente: salvare metadati invocazioni in tabella `runs` (agent, durata, esito, token before/after, fallback yes/no).
2. Implementare adapter CLI `Claude` reale (`ctx claude ...`) con stesso modello `prepare + execute + fallback` di Codex/OpenCode.
3. Aggiungere alias workflow del PDF: `ctx ask ...` e `ctx wrap <agent> --prompt ...` (con test E2E).
4. Completare `hook mode` operativo (`ctx hook ...`) per pre-prompt processing reale, non solo utility function.
5. Completare parser prioritari mancanti: `pytest`, Python traceback, `git diff`, `git status`, `tsc`, `eslint`, `ruff`, `mypy`, `cargo`, `go test`.
6. Portare i parser in formato estendibile (framework/plugin parser packs) con test di regressione per ogni parser.
7. Estendere AST/symbol extraction a `TS/JS` (MVP richiesto dal piano/PDF) e consolidare coverage linguaggi.
8. Migliorare extraction dipendenze/call graph oltre le euristiche attuali (incrementale, più preciso su cross-file).
9. Completare packer avanzato: `recent diff`, `immediate dependencies`, `task memory`, `failure memory` strutturata, `secondary docs`.
10. Aggiungere explainability del packer (`included/excluded + reason`) in output macchina (`--json`) e umano.
11. Implementare backend semantic `ONNX` reale con feature flag e fallback hash locale.
12. Rifinire MCP “plug-and-play”: compatibilità piena tool/resources, transport standard aggiuntivi oltre HTTP JSON-RPC custom, preset integrazione agent.
13. Completare hardening sicurezza/privacy: telemetry opt-in rigoroso, ignore rules sensibili configurabili, audit decisioni include/exclude verificabile.
14. Chiudere benchmark harness end-to-end (`repos.yaml`, `tasks/*.yaml`, runner, KPI completi, report markdown pubblicabile).
15. Eseguire benchmark reali e produrre report versionato da includere nel repository.
16. Completare packaging release: binari macOS/Linux, script release, smoke test installazione.
17. Completare distribuzione Homebrew/tap + documentazione install (`cargo`, binary release, brew) con test first-run (`ctx init`, `ctx index`, `ctx stats`).
18. Aggiornare README finale: stato reale, roadmap con check, sezione test completa, MCP purpose, limiti noti, esempi veri per ogni comando.
19. Eseguire QA finale su scenari del PDF (debug, refactor large repo, explain, MCP retrieval, codex/opencode/claude wrappers).
20. Preparare pubblicazione GitHub: tag `v0.1.0`, changelog, release notes, upload artefatti, issue templates/backlog post-MVP.

## Notes

- Runtime artifacts restano in locale sotto `.ctx/`.
- Nessun upload remoto obbligatorio.
- MCP è local-first su loopback `127.0.0.1`.
