# CTX - Context Runtime Engine for Coding Agents

Local-first context runtime che riduce rumore nel prompt e preserva segnale utile per coding agents.

## Current Status

Implementazione attiva basata su:
- `CTX_description.pdf`
- `docs/superpowers/plans/2026-04-23-ctx-runtime-engine.md`

Fondamenta già implementate:
- workspace Rust multi-crate
- CLI estesa (`init`, `index`, `reindex`, `graph build/query/rebuild`, `prune`, `pack`, `explain`, `retrieve`, wrappers, `memory`, `benchmark`, `stats`, `mcp serve`)
- config `.ctx/config.toml`
- pruning logs/diff con parser packs specializzati e provenance delle decisioni
- context packing avanzato con budget, priorità rigida, provenance e pack artifact
- graph SQLite con simboli, edge, snippet FTS, failure/decision memory
- retrieval ibrido (graph + FTS + semantic ranking ONNX/local-hash explainable)
- memory directives complete nel graph (`ctx memory set/get/list/delete/import/export`)
- benchmark A/B completo memory vs markdown (`ctx benchmark memory-ab`) con token, query coverage e quality/success scoring via checklist
- adapter runtime reali per `codex` e `opencode` (invocazione CLI + fallback)
- MCP server locale operativo su `/rpc`
- guardrail sicurezza (blocchi su file sensibili) + audit log

## Workspace Layout

- `crates/ctx-cli`: binary `ctx`
- `crates/ctx-core`: orchestrazione pipeline
- `crates/ctx-config`: config/bootstrap
- `crates/ctx-prune`: pruning deterministico logs/diff con parser packs per tool comuni
- `crates/ctx-pack`: context rewriter + budget packer avanzato
- `crates/ctx-graph`: storage/query SQLite + FTS
- `crates/ctx-intake`: intent detection
- `crates/ctx-ast`: parsing/slicing strutturale (tree-sitter + fallback)
- `crates/ctx-semantic`: semantic ranking ibrido con backend ONNX feature-gated, fallback local-hash e cache metadata
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
| `ctx prune logs` | Pulisce rumore log mantenendo root-cause e segnali critici da parser packs (`pytest`, traceback Python, `tsc`, `eslint`, `ruff`, `mypy`, `cargo`, `go test`, `npm`, `git status`) | `pytest -q 2>&1 \| ctx prune logs` | output compatto con failure/error/location utili, senza pass/progress noise |
| `ctx prune diff --query "refresh token"` | Tiene file header e hunk diff pertinenti alla query; accetta anche query posizionale (`ctx prune diff "refresh token"`) | `git diff \| ctx prune diff --query "refresh token"` | diff compatto query-relevant, senza hunk/file non pertinenti |
| `ctx help` | Stampa guida completa in inglese: cosa fa ogni comando + esempio di utilizzo | `ctx help` | output `CTX Command Guide` con esempi per tutti i comandi |
| `ctx pack "..." --json --attach file` | Crea context pack avanzato a budget con root cause, simboli, test, recent diff, dipendenze immediate, task/failure/memory e docs secondari | `ctx pack "fix auth" --json --attach /tmp/fail.txt` | JSON con `packed_tokens`, `reduction_pct`, `included`, `excluded`, `pack_path`, `compact_context` |
| `ctx explain "..."` | Mostra intent e contesto probabile | `ctx explain "fix failing pytest"` | include `intent: debug` |
| `ctx retrieve "..." --limit N` | Retrieval ibrido ordinato per score | `ctx retrieve "refresh token auth" --limit 3` | lista hit con score e contenuto rilevante |
| `ctx codex "..."` | Costruisce context pack e invoca Codex CLI con prompt + contesto compattato | `ctx codex "review risky diff"` | se `codex` è in PATH: esecuzione reale; altrimenti fallback con prompt pronto |
| `ctx claude "..."` | Wrapper adapter claude con contesto compattato | `ctx claude "explain flaky test"` | output con prefisso `adapter=claude` |
| `ctx opencode run "..."` | Costruisce context pack e invoca OpenCode CLI (`opencode run`) con prompt + contesto compattato | `ctx opencode run "explain this diff"` | se `opencode` è in PATH: esecuzione reale; altrimenti fallback con prompt pronto |
| `ctx memory set <key> <body> --scope project --source manual` | Inserisce/aggiorna una direttiva comportamentale nel grafo locale | `ctx memory set testing.always_run "Run targeted tests before completion." --scope project --source manual` | direttiva salvata nel graph memory |
| `ctx memory get <key>` | Legge una direttiva memory specifica | `ctx memory get testing.always_run` | stampa key/scope/source/body o `not found` |
| `ctx memory import --from AGENTS.md --scope project --source markdown --prefix agents` | Importa direttive da markdown (`AGENTS.md`/`CLAUDE.md`/`CODEX.md`) nel graph memory | `ctx memory import --from AGENTS.md --scope project --source markdown --prefix agents` | direttive estratte e persistite con report import |
| `ctx memory export --to AGENTS.generated.md --scope project --limit 200` | Esporta il graph memory in markdown compatibile/auditabile | `ctx memory export --to AGENTS.generated.md --scope project --limit 200` | file markdown generato con tutte le direttive |
| `ctx memory list --scope project --limit 10` | Elenca le direttive memory recenti | `ctx memory list --scope project --limit 10` | lista direttive con metadata |
| `ctx memory delete <key>` | Rimuove una direttiva memory | `ctx memory delete testing.always_run` | conferma eliminazione |
| `ctx benchmark memory-ab "<query>" --markdown AGENTS.md --limit 20 [--checklist file --markdown-answer file --graph-answer file]` | Benchmark A/B completo: token, query coverage e (opzionale) quality/success scoring su checklist | `ctx benchmark memory-ab "run tests and fix root cause" --markdown AGENTS.md --limit 20 --checklist quality-checklist.md --markdown-answer md.txt --graph-answer graph.txt` | output con `markdown_tokens`, `graph_memory_tokens`, `token_reduction_pct`, `quality_winner` |
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
| Pruning | `cargo test -p ctx-prune` | parser packs, dedup, budget priority, traceback preservation, git diff hunk selection |
| Intake | `cargo test -p ctx-intake` | intent detection e normalizzazione query |
| Token | `cargo test -p ctx-token` | stima token deterministica |
| AST | `cargo test -p ctx-ast` | estrazione simboli tree-sitter + slicing |
| Semantic | `cargo test -p ctx-semantic` | formula + ranking ibrido + dedup/adaptive threshold + fallback ONNX esplicito + embedding cache metadata |
| Semantic ONNX feature | `cargo test -p ctx-semantic --features onnx` | compilazione/test del backend ONNX feature-gated e policy fallback/errori |
| Graph | `cargo test -p ctx-graph` | simboli/edge/snippet FTS/failure/decision + memory directives CRUD/search |
| Pack | `cargo test -p ctx-pack` | priority order, compact rewrites, traceability, budget exclusions |
| Core | `cargo test -p ctx-core` | index+pack artifact+retrieval+guardrail + memory import/export + benchmark quality/success scoring |
| CLI | `cargo test -p ctx-cli` | e2e comandi + mcp serve + memory/benchmark/import/export commands |
| MCP | `cargo test -p ctx-mcp` | roundtrip RPC server + memory MCP tools |
| Telemetry | `cargo test -p ctx-telemetry` | stats e benchmark summary/report |

### 5) Semantic Backend Configuration

Default config keeps CTX usable out of the box:
```toml
[semantic]
enabled = true
backend = "onnx"
model = "local-mini-embed"
vocab = ""
max_chunks = 64
allow_fallback = true
```

What it does:
- `backend = "onnx"` requests a local ONNX embedding model.
- `model` can point to a local `.onnx` embedding model, including quantized models supported by the ONNX backend.
- `vocab` can point to a local `vocab.txt` for WordPiece-style token ids.
- `allow_fallback = true` keeps retrieval usable when local model files are missing by falling back to deterministic `local_hash` embeddings and marking reasons with `fallback_from=onnx`.
- Set `allow_fallback = false` to make missing ONNX files fail fast with an actionable error.

How to test strict ONNX build support:
```bash
cargo test -p ctx-semantic --features onnx
```

Expected behavior:
- semantic tests pass with and without the `onnx` feature.
- retrieval reasons expose the active backend, e.g. `backend=local_hash fallback_from=onnx` when fallback is used.

## Roadmap & Release Plan

Source of truth:
- `docs/superpowers/plans/2026-04-23-ctx-runtime-engine.md`
- `CTX_description.pdf`

### Task Status Matrix (1-20)

| Task | Area | Status | Note |
|---|---|---|---|
| 1 | Workspace bootstrap | Done | Struttura multi-crate pronta |
| 2 | CLI surface | Done | Comandi principali disponibili |
| 3 | Config system | Done | `.ctx/config.toml` e bootstrap runtime |
| 4 | Query intake | Done | Intent/query baseline operativi |
| 5 | Heuristic pruner + parser packs | Done | parser packs modulari pronti per `pytest`, traceback Python, `git diff`, `git status`, `tsc`, `eslint`, `ruff`, `mypy`, `cargo`, `go test`, `npm`; budget prioritario e provenance inclusa |
| 6 | AST analyzer | Done | tree-sitter + slicing (Rust/Python baseline) |
| 7 | Semantic engine | Done | ranking ibrido explainable, backend ONNX locale feature-gated, fallback local-hash esplicito, cache metadata/invalidation e integrazione retrieval |
| 8 | Knowledge graph | Done | schema/edge/snippet/failure/decision |
| 9 | Retrieval layer | Done | graph + FTS + semantic ranking |
| 10 | Context rewriter + budget packer | Done | priority order completo: query, root cause, symbols, tests, recent diff, dependencies, task/failure/directive memory, secondary docs; pack artifact JSON e included/excluded explainability |
| 11 | Invocation + telemetry | Partial | codex/opencode real invocation done, claude + runs metadata pending |
| 12 | Integration modes | Done (baseline) | wrapper/pipe/index/rebuild operativi |
| 13 | MCP server mode | Done (baseline) | server locale e tool core operativi |
| 14 | Security/privacy controls | Done (baseline) | guardrail + audit log locale |
| 15 | Installation/packaging | In Progress | release artifacts/Homebrew/pipeline da chiudere |
| 16 | Benchmarking framework | In Progress | harness/report pubblicabile da chiudere |
| 17 | MVP phase gates | Planned | formalizzazione criteri release |
| 18 | Demo/community assets | Planned | demo GIF/script + messaging finale |
| 19 | Future extensions backlog | Planned | backlog post-MVP da consolidare |
| 20 | Graph Memory Replacement Validation | Done | graph memory completo (CRUD/import/export) + benchmark A/B completo (token, coverage, checklist-based quality/success scoring) |

### Ordered Execution Queue (Now -> Final GitHub Release)

1. Chiudere `Task 11`: metadati invocazioni in tabella `runs` (agent, durata, esito, token before/after, fallback yes/no).
2. Implementare adapter CLI `Claude` reale (`ctx claude ...`) con `prepare + execute + fallback`.
3. Aggiungere alias workflow del PDF: `ctx ask ...` e `ctx wrap <agent> --prompt ...` con test E2E.
4. Completare `hook mode` operativo (`ctx hook ...`) per pre-prompt processing reale.
5. Estendere AST/symbol extraction a `TS/JS` e consolidare coverage linguaggi MVP.
6. Migliorare dependency/call graph oltre le euristiche attuali (cross-file più preciso).
7. Rifinire MCP “plug-and-play” (transport standard aggiuntivi + preset integrazione agent).
8. Completare hardening sicurezza/privacy (telemetry opt-in rigoroso, ignore rules sensibili, audit include/exclude verificabile).
9. Chiudere benchmark harness end-to-end (`repos.yaml`, `tasks/*.yaml`, runner, KPI, report markdown).
10. Eseguire benchmark reali e versionare i report nel repository.
11. Completare packaging release (binari macOS/Linux, script release, smoke test installazione).
12. Completare distribuzione Homebrew/tap e docs install (`cargo`, binary release, brew) con first-run checks.
13. Rifinire README finale: stato reale, test matrix completa, limiti noti, esempi finali.
14. Pubblicare benchmark suite completa del task “Graph Memory Replacement” su repository di esempio multipli con report versionati.
15. Eseguire QA finale sugli scenari PDF (debug, refactor, explain, MCP retrieval, wrappers codex/opencode/claude).
16. Pubblicazione GitHub: tag `v0.1.0`, changelog, release notes, upload artefatti, template issue e backlog post-MVP.

## Notes

- Runtime artifacts restano in locale sotto `.ctx/`.
- Nessun upload remoto obbligatorio.
- MCP è local-first su loopback `127.0.0.1`.
