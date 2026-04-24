# CTX - Context Runtime Engine for Coding Agents

Local-first context runtime che riduce rumore nel prompt e preserva segnale utile per coding agents.

## Current Status

Implementazione attiva basata su:
- `CTX_description.pdf`
- `docs/superpowers/plans/2026-04-23-ctx-runtime-engine.md`

Fondamenta già implementate:
- workspace Rust multi-crate
- CLI estesa (`init`, `index`, `reindex`, `graph build/query/rebuild`, `prune`, `pack`, `ask`, `hook`, `wrap`, `explain`, `retrieve`, wrappers, `memory`, `benchmark`, `stats`, `doctor`, `mcp serve`, `mcp stdio`, `mcp config`)
- config `.ctx/config.toml`
- pruning logs/diff con parser packs specializzati e provenance delle decisioni
- context packing avanzato con budget, priorità rigida, provenance e pack artifact
- graph SQLite con simboli, edge, snippet FTS, failure/decision memory
- retrieval ibrido (graph + FTS + semantic ranking ONNX/local-hash explainable)
- memory directives complete nel graph (`ctx memory set/get/list/delete/import/export`)
- benchmark A/B completo memory vs markdown (`ctx benchmark memory-ab`) con token, query coverage e quality/success scoring via checklist
- adapter runtime reali per `codex`, `claude` e `opencode` con invocation telemetry locale e fallback prompt-safe
- MCP server locale operativo via HTTP JSON-RPC `/rpc` e stdio MCP per client che avviano processi locali
- security/privacy controls completi: local-only di default, nessun upload remoto, telemetria anonima opt-in off, blocchi file sensibili, ignore rules configurabili e audit include/exclude locale
- installazione/release DX: `ctx doctor`, script packaging, smoke test installazione, docs install e Formula Homebrew template

Guide utili:
- `guide.md`: casi reali di utilizzo, comandi e output attesi
- `docs/security.md`: threat model, privacy defaults e verifiche sicurezza

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
- `guide.md`: guida pratica end-to-end
- `docs/security.md`: security/privacy model

## MCP Server: A Cosa Serve E Cosa Fa

Serve a esporre il motore CTX come servizio locale interrogabile da agent/tooling esterni (es. Claude Code via MCP), evitando di incollare manualmente file e log nel prompt.

Cosa fa in pratica:
- gira in locale su `127.0.0.1` (nessun cloud obbligatorio)
- espone endpoint RPC `POST /rpc`
- espone anche transport stdio (`ctx mcp stdio`) per client MCP che lanciano un comando locale
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

Comando stdio per integrazione MCP locale:
```bash
ctx --repo-root /path/to/project mcp stdio
```

Preset Claude Code:
```bash
ctx mcp config claude
```

Il preset stampa uno snippet JSON con `mcpServers.ctx.command = "ctx"` e `args = ["--repo-root", "...", "mcp", "stdio"]`, così il server CTX parte dentro il progetto corretto.

## Uso Nei Progetti Reali

Flusso consigliato in una repo:

1. Inizializza CTX:
```bash
ctx init
```

2. Indicizza il codice:
```bash
ctx index
```

3. Usa CTX come context builder senza invocare agent:
```bash
ctx ask "where is retry logic implemented?"
```

4. Usa CTX come wrapper per il tuo agent:
```bash
ctx wrap claude --prompt "explain why this auth test is flaky"
ctx wrap codex --prompt "review the last diff and find risky changes"
ctx wrap opencode --prompt "implement caching for embeddings"
```

5. Usa CTX in hook/pre-prompt scripts:
```bash
ctx hook "fix failing pytest in auth" > /tmp/ctx-preprompt.txt
```

6. Collega Claude Code via MCP stdio:
```bash
ctx mcp config claude
```

Copia lo snippet prodotto nella configurazione MCP del progetto o dell'utente. In questo modo Claude Code può chiamare strumenti come `get_relevant_context`, `search_symbols`, `recent_decisions` e `get_compact_diff` senza incollare manualmente file nel prompt.

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
| `ctx doctor` | Verifica first-run/install readiness: config, graph, audit, stats dirs e privacy defaults | `ctx doctor` prima/dopo `ctx init` | prima indica `config: missing` e `next: ctx init`; dopo indica `config: ok`, `graph: ok`, `local_only: true` |
| `ctx pack "..." --json --attach file` | Crea context pack avanzato a budget con root cause, simboli, test, recent diff, dipendenze immediate, task/failure/memory e docs secondari | `ctx pack "fix auth" --json --attach /tmp/fail.txt` | JSON con `packed_tokens`, `reduction_pct`, `included`, `excluded`, `pack_path`, `compact_context` |
| `ctx ask "..."` | Costruisce un context pack senza invocare agent, utile per copiare/leggere contesto o collegarlo a workflow custom | `ctx ask "where is retry logic implemented?"` | output compatto con `query:` e sezioni rilevanti |
| `ctx hook "..."` | Produce un pre-prompt pronto per hook/script degli agent CLI | `ctx hook "fix flaky auth test"` | output con `Task:`, `Compact Context:` e istruzione finale |
| `ctx wrap <agent> --prompt "..."` | Wrapper generico per `codex`, `claude`, `opencode`, `generic`; usa lo stesso runtime adapter e telemetry | `ctx wrap claude --prompt "explain flaky test"` | invoca agent reale se presente o fallback prompt-safe |
| `ctx explain "..."` | Mostra intent e contesto probabile | `ctx explain "fix failing pytest"` | include `intent: debug` |
| `ctx retrieve "..." --limit N` | Retrieval ibrido ordinato per score | `ctx retrieve "refresh token auth" --limit 3` | lista hit con score e contenuto rilevante |
| `ctx codex "..."` | Costruisce context pack, invoca Codex CLI via `codex exec` e registra invocation stats locali | `ctx codex "review risky diff"` | se `codex` è in PATH: esecuzione reale; altrimenti fallback con prompt pronto e `fallback_used=true` |
| `ctx claude "..."` | Costruisce context pack, invoca Claude Code via `claude -p` e registra invocation stats locali | `ctx claude "explain flaky test"` | se `claude` è in PATH: esecuzione reale; altrimenti fallback con prompt pronto e `fallback_used=true` |
| `ctx opencode run "..."` | Costruisce context pack, invoca OpenCode CLI via `opencode run` e registra invocation stats locali | `ctx opencode run "explain this diff"` | se `opencode` è in PATH: esecuzione reale; altrimenti fallback con prompt pronto e `fallback_used=true` |
| `ctx memory set <key> <body> --scope project --source manual` | Inserisce/aggiorna una direttiva comportamentale nel grafo locale | `ctx memory set testing.always_run "Run targeted tests before completion." --scope project --source manual` | direttiva salvata nel graph memory |
| `ctx memory get <key>` | Legge una direttiva memory specifica | `ctx memory get testing.always_run` | stampa key/scope/source/body o `not found` |
| `ctx memory import --from AGENTS.md --scope project --source markdown --prefix agents` | Importa direttive da markdown (`AGENTS.md`/`CLAUDE.md`/`CODEX.md`) nel graph memory | `ctx memory import --from AGENTS.md --scope project --source markdown --prefix agents` | direttive estratte e persistite con report import |
| `ctx memory export --to AGENTS.generated.md --scope project --limit 200` | Esporta il graph memory in markdown compatibile/auditabile | `ctx memory export --to AGENTS.generated.md --scope project --limit 200` | file markdown generato con tutte le direttive |
| `ctx memory list --scope project --limit 10` | Elenca le direttive memory recenti | `ctx memory list --scope project --limit 10` | lista direttive con metadata |
| `ctx memory delete <key>` | Rimuove una direttiva memory | `ctx memory delete testing.always_run` | conferma eliminazione |
| `ctx benchmark memory-ab "<query>" --markdown AGENTS.md --limit 20 [--checklist file --markdown-answer file --graph-answer file]` | Benchmark A/B completo: token, query coverage e (opzionale) quality/success scoring su checklist | `ctx benchmark memory-ab "run tests and fix root cause" --markdown AGENTS.md --limit 20 --checklist quality-checklist.md --markdown-answer md.txt --graph-answer graph.txt` | output con `markdown_tokens`, `graph_memory_tokens`, `token_reduction_pct`, `quality_winner` |
| `ctx stats` | Legge token reduction, latency, adapter status e fallback metadata dell'ultimo run locale | dopo `ctx claude ...`, esegui `ctx stats` | JSON con `original_tokens`, `packed_tokens`, `reduction_pct`, `latency_ms`, `agent`, `status`, `fallback_used` |
| `ctx pack "..." --attach .env` | Security guardrail: blocca allegati sensibili e scrive audit privacy locale | `ctx pack "fix auth" --attach .env && cat .ctx/audit.log` | errore esplicito di blocco + evento `privacy_decision` con `decision=excluded` e `reason=sensitive_pattern` |
| `ctx mcp serve --port 8765 --once` | Avvia server MCP e gestisce una sola request | avvia e invia una RPC | risposta valida e chiusura pulita |
| `ctx mcp stdio` | Avvia MCP su stdin/stdout per client che eseguono server locali | `printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n' \| ctx mcp stdio` | risposta JSON-RPC con `serverInfo.name = "ctx-mcp"` |
| `ctx mcp config claude` | Stampa configurazione MCP stdio per Claude Code legata al repo corrente | `ctx mcp config claude` | JSON con `mcpServers.ctx.command` e `args` |

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
| Config privacy | `cargo test -p ctx-config security_` | default local-only, no remote upload, telemetria anonima opt-in off |
| Pruning | `cargo test -p ctx-prune` | parser packs, dedup, budget priority, traceback preservation, git diff hunk selection |
| Intake | `cargo test -p ctx-intake` | intent detection e normalizzazione query |
| Token | `cargo test -p ctx-token` | stima token deterministica |
| AST | `cargo test -p ctx-ast` | estrazione simboli tree-sitter + slicing |
| Semantic | `cargo test -p ctx-semantic` | formula + ranking ibrido + dedup/adaptive threshold + fallback ONNX esplicito + embedding cache metadata |
| Semantic ONNX feature | `cargo test -p ctx-semantic --features onnx` | compilazione/test del backend ONNX feature-gated e policy fallback/errori |
| Graph | `cargo test -p ctx-graph` | simboli/edge/snippet FTS/failure/decision + memory directives CRUD/search |
| Pack | `cargo test -p ctx-pack` | priority order, compact rewrites, traceability, budget exclusions |
| Core | `cargo test -p ctx-core` | index+pack artifact+retrieval+guardrail + memory import/export + benchmark quality/success scoring |
| Core privacy | `cargo test -p ctx-core sensitive` | blocco allegati sensibili, skip indicizzazione sensibile e audit privacy |
| CLI | `cargo test -p ctx-cli` | e2e comandi + mcp serve + memory/benchmark/import/export commands |
| CLI doctor | `cargo test -p ctx-cli doctor` | first-run/install readiness prima e dopo `ctx init` |
| Release assets | `cargo test -p ctx-cli release_assets` | script release, smoke install, Formula Homebrew e docs install presenti/coerenti |
| MCP | `cargo test -p ctx-mcp` | roundtrip RPC server, stdio dispatcher + memory MCP tools |
| Telemetry | `cargo test -p ctx-telemetry` | stats, invocation metadata compatibility, audit log e benchmark summary/report |
| Telemetry privacy | `cargo test -p ctx-telemetry privacy` | eventi `privacy_decision` JSON append-only in `.ctx/audit.log` |

### 5) Security And Privacy Smoke

Command:
```bash
ctx init
printf 'API_KEY=secret\n' > .env
ctx pack "fix auth" --attach .env
cat .ctx/audit.log
```

Cosa fa:
- verifica che CTX non legga allegati sensibili nel context pack;
- registra localmente la decisione privacy;
- conferma che la postura di default sia `local_only = true` e `remote_upload_enabled = false`.

Comportamento atteso:
```text
attachment .env matches sensitive file patterns and was blocked
```

Audit atteso:
```json
{"kind":"privacy_decision","decision":"excluded","path":".env","reason":"sensitive_pattern","local_only":true,"remote_upload_enabled":false,"message":"blocked sensitive attachment before packing"}
```

### 6) Semantic Backend Configuration

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

### 7) Installation And Release Smoke

First-run doctor:
```bash
ctx doctor
ctx init
ctx doctor
ctx index
```

Expected behavior:
- before init: `config: missing` and `next: ctx init`;
- after init: `config: ok`, `graph: ok`, `audit_log: ok`, `local_only: true`, `remote_upload_enabled: false`;
- after indexing: `indexed_files: N`.

Package current platform:
```bash
scripts/release/build.sh
```

What it does:
- runs `cargo fmt --all --check`;
- runs `cargo test --workspace`;
- builds `ctx` release binary;
- runs `scripts/release/install-smoke.sh`;
- creates `dist/ctx-<version>-<target>.tar.gz`;
- writes `dist/SHA256SUMS`.

Smoke an existing binary:
```bash
scripts/release/install-smoke.sh ./target/release/ctx
```

Expected behavior:
- validates `ctx help`, `ctx doctor`, `ctx init`, `ctx index`, `ctx pack`, `ctx stats` and `ctx mcp stdio`.

## Roadmap & Release Plan

Source of truth:
- `docs/superpowers/plans/2026-04-23-ctx-runtime-engine.md`
- `CTX_description.pdf`

### Task Status Matrix (1-21)

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
| 11 | Invocation + telemetry | Done | codex/claude/opencode real invocation, fallback behavior, local runs metadata, stats e audit complete |
| 12 | Integration modes | Done | wrapper, pipe/filter, batch/index, hook/pre-prompt, ask e generic wrap operativi |
| 13 | MCP server mode | Done | HTTP JSON-RPC, stdio MCP, preset Claude Code e tool core operativi |
| 14 | Security/privacy controls | Done | local-only defaults, no remote upload, anonymous telemetry opt-in off, local stats explicit, sensitive pattern blocking, configurable ignored dirs, privacy decision audit e threat model docs |
| 15 | Installation/packaging | Done | `ctx doctor`, release packaging script, install smoke script, docs install e Homebrew Formula template |
| 16 | Benchmarking framework | In Progress | harness/report pubblicabile da chiudere |
| 17 | MVP phase gates | Planned | formalizzazione criteri release |
| 18 | Demo/community assets | Planned | demo GIF/script + messaging finale |
| 19 | Future extensions backlog | Planned | backlog post-MVP da consolidare |
| 20 | Graph Memory Replacement Validation | Done | graph memory completo (CRUD/import/export) + benchmark A/B completo (token, coverage, checklist-based quality/success scoring) |
| 21 | Interactive command autocomplete menu | Planned | menu a comparsa con fuzzy autocomplete, descrizione breve del comando, esempi e preview parametri mentre l'utente scrive |

### Ordered Execution Queue (Now -> Final GitHub Release)

1. Estendere AST/symbol extraction a `TS/JS` e consolidare coverage linguaggi MVP.
2. Migliorare dependency/call graph oltre le euristiche attuali (cross-file più preciso).
3. Chiudere benchmark harness end-to-end (`repos.yaml`, `tasks/*.yaml`, runner, KPI, report markdown).
4. Eseguire benchmark reali e versionare i report nel repository.
5. Rifinire distribuzione Homebrew/tap dopo URL pubblico reale e SHA release definitivo.
6. Rifinire README finale: stato reale, test matrix completa, limiti noti, esempi finali.
7. Pubblicare benchmark suite completa del task “Graph Memory Replacement” su repository di esempio multipli con report versionati.
8. Eseguire QA finale sugli scenari PDF (debug, refactor, explain, MCP retrieval, wrappers codex/opencode/claude).
9. Pubblicazione GitHub: tag `v0.1.0`, changelog, release notes, upload artefatti, template issue e backlog post-MVP.
10. Post-release DX: progettare e implementare menu autocomplete interattivo (`ctx tui`/shell integration) con suggerimenti, descrizioni brevi e preview esempi.

## Notes

- Runtime artifacts restano in locale sotto `.ctx/`.
- Nessun upload remoto obbligatorio.
- MCP è local-first su loopback `127.0.0.1`.
- Privacy/security dettagliate in `docs/security.md`.
