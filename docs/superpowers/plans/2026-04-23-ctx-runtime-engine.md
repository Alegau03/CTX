# CTX Context Runtime Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a local-first Context Runtime Engine for coding agents that cuts noisy context, preserves high-signal code evidence, and improves task outcomes while staying provider-agnostic.

**Architecture:** A Rust local runtime (`ctx`) with modular pipeline stages: signal collection, deterministic pruning, AST structural slicing, semantic ranking, graph enrichment, budget packing, agent invocation, and telemetry. It supports wrapper/pipe/hook/MCP modes and stores durable local memory in SQLite (FTS5 + JSON) to enable retrieval without reinjecting whole files.

**Tech Stack:** Rust (`clap`, `tokio`, `serde`, `tracing`, `rusqlite`), Tree-sitter, ONNX Runtime embeddings, local SQLite storage, optional MCP server, cross-platform packaging.

---

## 1) Scope Lock: Must-Have Requirements (From PDF)

- [ ] Keep project as local runtime utility (not a webapp, no mandatory hosting).
- [ ] Keep product local-first: no mandatory cloud backend, no mandatory account, no model hosting.
- [ ] Target token reduction objective: 60-90% on noisy workloads.
- [ ] Improve answer quality for debugging/refactor/repository-understanding tasks.
- [ ] Run 100% locally on common laptops.
- [ ] Avoid dedicated GPU dependency and avoid heavy server-side inference.
- [ ] Preserve model/provider agnosticism: Codex CLI, Claude Code, OpenCode, generic CLI.
- [ ] Deliver both core subsystems:
- [ ] Context Pruner & Optimizer.
- [ ] Local ML/Code Knowledge Graph.
- [ ] Support all integration modes:
- [ ] Wrapper mode (`ctx codex ...`, `ctx claude ...`, `ctx opencode ...`).
- [ ] Pipe/filter mode (`ctx prune logs`, `ctx prune diff`).
- [ ] MCP server mode (`ctx mcp serve` + tools/resources).
- [ ] Hook mode (pre-prompt preprocessing).
- [ ] Batch/index mode (`ctx index`, `ctx reindex`, `ctx graph rebuild`).
- [ ] Respect declared non-goals: not a new LLM, not an IDE, not SaaS observability, not cloud orchestrator.

## 2) Workstreams and Ownership

### Workstream A: Product + Positioning
- [ ] Define category language and README pitch: "Context Runtime Engine for Coding Agents".
- [ ] Preserve naming alternatives in docs for discoverability (`ctxd`, `prism`, `codectx`, `context-engine`, `agentctx`, `prunegraph`).
- [ ] Keep messaging focused on signal-over-noise and workflow compatibility.
- [ ] Ship transparent include/exclude explanations in CLI outputs.

### Workstream B: CLI Runtime Core
- [ ] Build command tree and config resolution.
- [ ] Build query intake and pipeline orchestration.
- [ ] Build token estimation and budget-aware pack builder.

### Workstream C: Pruning + Parsing
- [ ] Deterministic heuristics for logs/diff noise elimination.
- [ ] Parser packs for prioritized command outputs.
- [ ] Safe-pruning constraints for code integrity.

### Workstream D: Code Understanding
- [ ] Tree-sitter multi-language structural indexing.
- [ ] Symbol graph extraction and structural slicing.
- [ ] Semantic relevance ranking (hybrid scoring).

### Workstream E: Local Memory + Retrieval
- [ ] SQLite schema for graph, runs, failures, notes, snippets.
- [ ] Hybrid retrieval: graph traversal + FTS + semantic + recency.
- [ ] Incremental update policy tied to file changes, commits, failures, task completion.
- [ ] Record explicit ADR: SQLite chosen over Neo4j to preserve zero-cost/easy-install constraints.

### Workstream F: Integrations
- [ ] Agent adapters (Codex/Claude/OpenCode/generic).
- [ ] MCP server tools/resources.
- [ ] Hooks and batch workflows.

### Workstream G: Reliability + Security + Metrics
- [ ] Local privacy controls and opt-in telemetry.
- [ ] Benchmark harness and KPI dashboard.
- [ ] Packaging/install/distribution.

## 3) Proposed Repository Structure (Greenfield)

- [ ] Create `crates/ctx-cli/src/main.rs` (CLI entrypoint).
- [ ] Create `crates/ctx-core/src/lib.rs` (domain orchestration).
- [ ] Create `crates/ctx-config/src/lib.rs` (config parsing/validation).
- [ ] Create `crates/ctx-intake/src/lib.rs` (signal collection + intent typing).
- [ ] Create `crates/ctx-prune/src/lib.rs` (heuristics + parser framework).
- [ ] Create `crates/ctx-ast/src/lib.rs` (tree-sitter symbol extraction/slicing).
- [ ] Create `crates/ctx-semantic/src/lib.rs` (embeddings + ranking).
- [ ] Create `crates/ctx-pack/src/lib.rs` (budget packer + explain mode).
- [ ] Create `crates/ctx-graph/src/lib.rs` (SQLite graph storage + retrieval).
- [ ] Create `crates/ctx-mcp/src/lib.rs` (MCP server implementation).
- [ ] Create `crates/ctx-adapters/src/lib.rs` (agent-specific invocations).
- [ ] Create `crates/ctx-hooks/src/lib.rs` (hook integration surfaces).
- [ ] Create `crates/ctx-telemetry/src/lib.rs` (stats + audit logs).
- [ ] Create `crates/ctx-token/src/lib.rs` (token estimation backends).
- [ ] Create `tests/e2e/` (scenario-level integration tests).
- [ ] Create `benchmarks/` (fixed benchmark tasks + repos metadata).
- [ ] Create `docs/` (architecture, security, CLI, benchmark report templates).
- [ ] Create `.ctx/` runtime artifacts directory contract (packs/cache/graph/stats/audit).

## 4) Task-by-Task Execution Plan

### Task 1: Bootstrap Workspace and Build Skeleton

**Files:**
- Create: `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`
- Create: `crates/*/Cargo.toml`, `crates/*/src/lib.rs`, `crates/ctx-cli/src/main.rs`
- Create: `README.md`, `LICENSE`, `.gitignore`
- Test: `tests/smoke/test_cli_boot.rs`

- [ ] Step 1: Initialize Rust workspace with crate boundaries listed above.
- [ ] Step 2: Implement minimal `ctx --help` and `ctx --version`.
- [ ] Step 3: Add CI checks for build, clippy, fmt, unit tests.
- [ ] Step 4: Add smoke test asserting CLI boots on macOS/Linux.
- [ ] Step 5: Commit baseline scaffold.

**Acceptance Criteria:** workspace compiles; `ctx --help` works; CI green.

### Task 2: CLI Surface (All Commands in PDF)

**Files:**
- Modify: `crates/ctx-cli/src/main.rs`
- Create: `crates/ctx-cli/src/commands/*.rs`
- Test: `tests/e2e/test_cli_commands.rs`

- [ ] Step 1: Implement top-level commands:
- [ ] `ctx init`
- [ ] `ctx index`
- [ ] `ctx reindex`
- [ ] `ctx graph build`
- [ ] `ctx graph query "..."`
- [ ] `ctx prune logs`
- [ ] `ctx prune diff`
- [ ] `ctx pack "..."`
- [ ] `ctx explain "..."`
- [ ] `ctx codex "..."`
- [ ] `ctx claude "..."`
- [ ] `ctx opencode run "..."`
- [ ] `ctx mcp serve`
- [ ] `ctx stats`
- [ ] Step 2: Add global flags: `--budget`, `--json`, `--attach`, `--repo-root`, `--config`.
- [ ] Step 3: Add structured JSON output contract for automation and adapter calls.
- [ ] Step 4: Write E2E tests for parsing, help text, and invalid input handling.
- [ ] Step 5: Commit CLI contract.

**Acceptance Criteria:** every command in PDF exists and is documented.

### Task 3: Config System (`.ctx/config.toml`)

**Files:**
- Create: `crates/ctx-config/src/config.rs`
- Create: `templates/config.default.toml`
- Test: `crates/ctx-config/tests/config_parse.rs`

- [ ] Step 1: Implement config sections exactly as specified:
- [ ] `[general] repo_root, default_budget, agent`
- [ ] `[pruning] collapse_success_logs, keep_imports, keep_public_signatures, max_log_lines`
- [ ] `[semantic] enabled, backend, model, max_chunks`
- [ ] `[graph] enabled, store, index_tests, index_docs`
- [ ] `[mcp] enabled, port`
- [ ] Step 2: Add `ctx init` that writes `.ctx/config.toml` and directory tree.
- [ ] Step 3: Add config precedence: CLI flags > env > local config > defaults.
- [ ] Step 4: Add config validation errors with actionable messages.
- [ ] Step 5: Commit config layer.

**Acceptance Criteria:** `.ctx/config.toml` generated and validated deterministically.

### Task 4: Query Intake + Signal Collection (Pipeline Stage 0)

**Files:**
- Create: `crates/ctx-intake/src/intake.rs`
- Modify: `crates/ctx-core/src/pipeline.rs`
- Test: `crates/ctx-intake/tests/intake_normalization.rs`

- [ ] Step 1: Define normalized intake payload fields:
- [ ] `task`, `intent`, `repo_root`, `signals.recent_files`, `signals.command_output`, `signals.git_status`.
- [ ] Step 2: Build detectors for intent classes (`debug`, `refactor`, `review`, `explain`).
- [ ] Step 3: Collect local signals: recent file touches, last failing command, git diff/status, failing test references.
- [ ] Step 4: Persist intake events for telemetry and explainability.
- [ ] Step 5: Commit intake module.

**Acceptance Criteria:** each invocation produces typed intake JSON and signal snapshot.

### Task 5: Heuristic Pruner + Parser Packs (Stage 1)

**Files:**
- Create: `crates/ctx-prune/src/heuristic.rs`
- Create: `crates/ctx-prune/src/parsers/*.rs`
- Test: `crates/ctx-prune/tests/{logs,diff,traceback}.rs`

- [ ] Step 1: Implement deterministic pruning rules:
- [ ] drop duplicate lines
- [ ] collapse repetitive success output
- [ ] preserve stderr critical blocks
- [ ] extract stacktrace root cause
- [ ] shorten diff to query-relevant hunks
- [ ] Step 2: Implement parser packs for prioritized sources:
- [ ] `pytest`, Python traceback, `npm install`, `tsc`, `cargo build`, `go test`, `git diff`, `git status`, `ruff`, `mypy`, `eslint`.
- [ ] Step 3: Enforce safe-pruning constraints (never strip critical imports/public signatures/class headers/relevant decorators/public API docstrings).
- [ ] Step 4: Add explain output: included, excluded, and reasons.
- [ ] Step 5: Commit pruning engine.

**Acceptance Criteria:** noisy inputs reduced while preserving failure root-cause signal.

### Task 6: Syntax & Structure Analyzer (Stage 2)

**Files:**
- Create: `crates/ctx-ast/src/tree_sitter_index.rs`
- Create: `crates/ctx-ast/src/symbols.rs`
- Test: `crates/ctx-ast/tests/symbol_extraction.rs`

- [ ] Step 1: Add tree-sitter parsers for initial language set (Python + TS/JS in MVP, Rust/Go in phase extension).
- [ ] Step 2: Extract entities: modules, classes, functions, methods, imports, tests.
- [ ] Step 3: Maintain file-to-symbol mapping and stable symbol IDs.
- [ ] Step 4: Implement structural slicing by symbol boundaries.
- [ ] Step 5: Commit AST layer.

**Acceptance Criteria:** `ctx index` builds symbol map and supports structural candidate chunks.

### Task 7: Semantic Relevance Engine (Stage 3)

**Files:**
- Create: `crates/ctx-semantic/src/onnx_backend.rs`
- Create: `crates/ctx-semantic/src/ranking.rs`
- Test: `crates/ctx-semantic/tests/ranking_formula.rs`

- [ ] Step 1: Integrate ONNX Runtime embedding backend with local quantized model support.
- [ ] Step 2: Build chunk embedding cache metadata and invalidation policy.
- [ ] Step 3: Implement hybrid score formula:
- [ ] `0.40 semantic_similarity + 0.20 keyword_overlap + 0.15 recency + 0.15 graph_distance_bonus + 0.10 failure_bonus`
- [ ] Step 4: Implement adaptive thresholds and deduplication.
- [ ] Step 5: Add `ctx explain` scoring breakdown.
- [ ] Step 6: Commit semantic engine.

**Acceptance Criteria:** query-aware ranking is deterministic and explainable.

### Task 8: Knowledge Graph Engine (Stage 4 + Durable Memory)

**Files:**
- Create: `crates/ctx-graph/src/schema.sql`
- Create: `crates/ctx-graph/src/store.rs`
- Create: `crates/ctx-graph/src/retrieval.rs`
- Test: `crates/ctx-graph/tests/{schema,queries,incremental_updates}.rs`

- [ ] Step 1: Implement SQLite schema with tables:
- [ ] `files`, `symbols`, `edges`, `tasks`, `runs`, `failures`, `notes`, `snippets`, `embeddings_metadata`.
- [ ] Step 2: Add FTS5 index for textual snippets and JSON fields for structured metadata.
- [ ] Step 3: Model required entities:
- [ ] repository, directory, file, symbol, dependency/import, test, command run, failure, task, decision, note, issue/PR local reference.
- [ ] Step 4: Model edges:
- [ ] `contains`, `imports`, `calls`, `defines`, `tests`, `failed_in`, `related_to_query`, `edited_with`, `mentioned_in_task`.
- [ ] Step 5: Implement update policies:
- [ ] file modified -> incremental reparse
- [ ] new commit -> dependency refresh
- [ ] failed command -> create failure node
- [ ] task completed -> short decision summary
- [ ] Step 6: Implement graph query API backing `ctx graph query`.
- [ ] Step 7: Add ADR note documenting SQLite-over-Neo4j tradeoff and rationale.
- [ ] Step 8: Commit graph engine.

**Acceptance Criteria:** graph stores structural+operational memory and supports fast local queries.

### Task 9: Retrieval Layer (Hybrid)

**Files:**
- Modify: `crates/ctx-graph/src/retrieval.rs`
- Create: `crates/ctx-core/src/retrieval_orchestrator.rs`
- Test: `tests/e2e/test_retrieval_precision.rs`

- [ ] Step 1: Implement retrieval strategy combining graph traversal + FTS + semantic + recency bias.
- [ ] Step 2: Add per-query retrieval budget and diversity constraints.
- [ ] Step 3: Expose retrieval snippets with stable source references.
- [ ] Step 4: Track precision@k for evaluation.
- [ ] Step 5: Commit retrieval layer.

**Acceptance Criteria:** relevant symbol/test/failure neighbors are recoverable without full-file injection.

### Task 10: Context Rewriter + Budget Packer (Stage 5)

**Files:**
- Create: `crates/ctx-pack/src/rewriter.rs`
- Create: `crates/ctx-pack/src/packer.rs`
- Test: `crates/ctx-pack/tests/packing_priorities.rs`

- [ ] Step 1: Implement compact rewrite forms:
- [ ] signature + short doc instead of full blocks
- [ ] compact diff summaries
- [ ] module-level hierarchical summaries
- [ ] Step 2: Implement strict priority order in packer:
- [ ] user query
- [ ] error root cause
- [ ] directly relevant symbols
- [ ] associated tests
- [ ] recent diff
- [ ] immediate dependencies
- [ ] memory/decisions
- [ ] secondary docs
- [ ] Step 3: Preserve minimum guarantees for every compressed block:
- [ ] file path, symbol signature, significant imports, relationships, source references/ranges
- [ ] Step 4: Emit JSON output contract with original tokens, packed tokens, reduction %, included/excluded, pack path.
- [ ] Step 5: Commit packer.

**Acceptance Criteria:** pack respects budgets and preserves traceability.

### Task 11: Invocation + Telemetry (Stage 6)

**Files:**
- Create: `crates/ctx-adapters/src/{codex,claude,opencode,generic}.rs`
- Create: `crates/ctx-telemetry/src/stats.rs`
- Create: `crates/ctx-telemetry/src/audit.rs`
- Test: `tests/e2e/test_adapter_invocation.rs`

- [ ] Step 1: Build adapter abstraction with per-agent command templates.
- [ ] Step 2: Implement wrapper commands: `ctx codex`, `ctx claude`, `ctx opencode run`.
- [ ] Step 3: Register invocation metadata in local runs table.
- [ ] Step 4: Implement `ctx stats` with token reduction and latency overhead summaries.
- [ ] Step 5: Add local audit log of pruning decisions.
- [ ] Step 6: Commit invocation/telemetry layer.

**Acceptance Criteria:** end-to-end invoke path works and records local metrics.

### Task 12: Integration Modes Beyond Wrapper

**Files:**
- Create: `crates/ctx-cli/src/commands/prune.rs`
- Create: `crates/ctx-hooks/src/hook_runner.rs`
- Modify: `crates/ctx-cli/src/commands/index.rs`
- Test: `tests/e2e/test_modes.rs`

- [ ] Step 1: Finalize pipe/filter workflows (`ctx prune logs`, `ctx prune diff`).
- [ ] Step 2: Add hook mode entrypoint suitable for pre-prompt scripts.
- [ ] Step 3: Add batch/index flows:
- [ ] `ctx index`
- [ ] `ctx reindex src tests`
- [ ] `ctx graph rebuild`
- [ ] Step 4: Add mode-specific docs and examples.
- [ ] Step 5: Commit integration modes.

**Acceptance Criteria:** all five integration modes from PDF are usable.

### Task 13: MCP Server Mode

**Files:**
- Create: `crates/ctx-mcp/src/server.rs`
- Create: `crates/ctx-mcp/src/tools/*.rs`
- Create: `crates/ctx-mcp/src/resources/*.rs`
- Test: `tests/e2e/test_mcp_tools.rs`

- [ ] Step 1: Implement local MCP server bootstrap (`ctx mcp serve`).
- [ ] Step 2: Expose required tools/resources:
- [ ] `get_relevant_context`
- [ ] `project_map`
- [ ] `search_symbols`
- [ ] `related_failures`
- [ ] `recent_decisions`
- [ ] `get_compact_diff`
- [ ] Step 3: Add auth/trust model for localhost-only by default.
- [ ] Step 4: Add MCP integration docs for Claude Code.
- [ ] Step 5: Commit MCP mode.

**Acceptance Criteria:** an MCP-capable agent can request context tools dynamically.

### Task 14: Security and Privacy Controls

**Files:**
- Create: `docs/security.md`
- Modify: `crates/ctx-config/src/config.rs`
- Modify: `crates/ctx-telemetry/src/audit.rs`
- Test: `tests/e2e/test_privacy_defaults.rs`

- [x] Step 1: Enforce defaults: no remote upload, telemetry opt-in off, local storage only.
- [x] Step 2: Implement ignore rules for sensitive file patterns.
- [x] Step 3: Provide auditability for include/exclude decisions.
- [x] Step 4: Add threat model doc for local trust assumptions.
- [ ] Step 5: Commit security/privacy controls.

**Acceptance Criteria:** privacy posture is explicit, verifiable, and safe-by-default.

### Task 15: Installation, Packaging, and DX

**Files:**
- Create: `scripts/release/*.sh`
- Create: `Formula/context-engine.rb` (or tap instructions)
- Create: `docs/install.md`
- Test: `tests/e2e/test_installation_smoke.rs`

- [x] Step 1: Produce release artifacts for macOS/Linux.
- [x] Step 2: Document install paths: Homebrew, Cargo, GitHub Releases binaries.
- [x] Step 3: Ensure first-run flow: `ctx init`, `ctx index`, `ctx stats`.
- [x] Step 4: Add installation smoke checks in CI release pipeline.
- [ ] Step 5: Commit packaging.

**Acceptance Criteria:** users can install and run first useful command sequence in <10 minutes.

### Task 16: Benchmarking and KPI Framework

**Files:**
- Create: `benchmarks/repos.yaml`
- Create: `benchmarks/tasks/*.yaml`
- Create: `crates/ctx-telemetry/src/benchmark.rs`
- Create: `docs/benchmark-results-template.md`
- Test: `tests/e2e/test_benchmark_runner.rs`

- [ ] Step 1: Build reproducible benchmark harness on Python, TS/JS, Rust/Go repos.
- [ ] Step 2: Define baseline vs `ctx` experimental protocol.
- [ ] Step 3: Collect required KPIs:
- [ ] token reduction %
- [ ] latency overhead local
- [ ] task success rate vs baseline
- [ ] user-judged answer quality
- [ ] fix success on benchmark tasks
- [ ] retrieval precision@k
- [ ] Step 4: Generate shareable markdown report outputs.
- [ ] Step 5: Commit benchmark system.

**Acceptance Criteria:** claims are evidence-backed and reproducible.

### Task 17: MVP Phasing (Virality to Advanced)

**Files:**
- Create: `docs/mvp-phases.md`
- Modify: `README.md`

- [ ] Step 1: Phase 1 (viral MVP): wrapper, prune logs/diff, index, symbol extraction, simple graph, budget packer, token-diff report.
- [ ] Step 2: Phase 2: semantic ONNX ranking + score explainability.
- [ ] Step 3: Phase 3: MCP + serious adapters + integration presets.
- [ ] Step 4: Phase 4: advanced memory (failures/tasks/decisions) + simple graph query language.
- [ ] Step 5: Commit phase gates and exit criteria.

**Acceptance Criteria:** roadmap is staged, shippable, and measurable per phase.

### Task 18: README, Demo, and Go-to-Community Assets

**Files:**
- Modify: `README.md`
- Create: `docs/demo-script.md`
- Create: `docs/virality-assets.md`

- [ ] Step 1: Add one-line pitch from PDF in README hero.
- [ ] Step 2: Add transparent before/after token demo section.
- [ ] Step 2b: Add GIF-based demo script showing giant logs -> `ctx prune logs` -> compact context -> successful agent outcome.
- [ ] Step 3: Add explainability output examples (included/excluded + why).
- [ ] Step 4: Add local-first/no-lock-in value proposition.
- [ ] Step 5: Add contributor-friendly extension points (parsers/adapters/language packs).
- [ ] Step 6: Commit communication assets.

**Acceptance Criteria:** repository message communicates value in <30 seconds.

### Task 19: Future Extensions Backlog (Explicitly Preserved)

**Files:**
- Create: `docs/future-extensions.md`

- [ ] Step 1: Track notebook-aware support.
- [ ] Step 2: Track PR review mode.
- [ ] Step 3: Track open benchmark suite for context quality.
- [ ] Step 4: Track stack-specific rule packs (PyTorch/FastAPI/React/Next.js/Rust crates).
- [ ] Step 5: Track active learning for relevance ranking.
- [ ] Step 6: Track export of shareable project capsules.
- [ ] Step 7: Track interactive command autocomplete popup/menu with fuzzy suggestions, short descriptions, examples, and parameter previews.
- [ ] Step 8: Commit explicit post-MVP backlog.

**Acceptance Criteria:** every future extension from PDF is preserved with ownership and status.

## 5) Quality Gates (Global)

- [ ] Unit tests for each crate pass.
- [ ] Integration tests for all modes pass.
- [ ] E2E scenarios pass:
- [ ] debug failing pytest
- [ ] refactor on large repo
- [ ] local explain mode without agent
- [ ] MCP tool call retrieval
- [ ] OpenCode and Codex wrapper paths
- [ ] Performance gates:
- [ ] pruning stage p95 latency within target
- [ ] packing overhead bounded
- [ ] no uncontrolled DB growth
- [ ] Security gates:
- [ ] telemetry disabled by default
- [ ] sensitive paths excluded
- [ ] no unintended network egress

## 6) Example Acceptance Scenarios (From PDF Use Cases)

- [ ] Scenario A: `pytest ... | ctx prune logs` retains only failure root cause + related tests + recent diffs.
- [ ] Scenario B: `ctx claude "Refactor data loader..."` includes relevant loader modules and excludes `notebooks/`, `artifacts/`, `build/`, `dist/`.
- [ ] Scenario C: `ctx explain "Where is retry logic implemented?"` returns likely symbols + related history.
- [ ] Scenario D: `ctx mcp serve` enables tool-based retrieval from agent side.
- [ ] Scenario E: `ctx codex "Review the last diff..."` provides compact risk-focused context package.
- [ ] Scenario F (Persona A ML/AI engineer): failing training/auth pipeline gets compact relevant code+test context without experiment-log bloat.
- [ ] Scenario G (Persona B OSS maintainer): large PR analysis keeps risky hunks/symbols and excludes unrelated directories.
- [ ] Scenario H (Persona C student/ricercatore): free-tier-friendly token budget stays under strict budget while preserving answer utility.

## 7) PDF Coverage Matrix (No Feature Left Behind)

- [ ] Section 1 (Executive summary): Implemented by Tasks 1-3, 10-11, 18.
- [ ] Section 2 (Problem framing): Implemented by Tasks 4-5, 10, 16.
- [ ] Section 3 (Goals/non-goals): Enforced by Scope Lock + Tasks 14-16.
- [ ] Section 4 (Positioning): Implemented by Workstream A + Task 18.
- [ ] Section 5 (Personas): Reflected in benchmark scenarios + docs messaging.
- [ ] Section 6 (Vision: Pruner + Graph): Implemented by Tasks 5-9.
- [ ] Section 7 (Core workflow): Implemented by Tasks 4-11.
- [ ] Section 8 (Integration modes): Implemented by Tasks 11-13.
- [ ] Section 9 (High-level architecture): Implemented by repo structure + orchestrator tasks.
- [ ] Section 10 (Components): Implemented by Tasks 4-13.
- [ ] Section 11 (Pipeline stages 0-6): Implemented by Tasks 4-11.
- [ ] Section 12 (Packing strategy): Implemented by Task 10.
- [ ] Section 13 (Concrete usage examples): Implemented by Task 6 + Scenario tests.
- [ ] Section 14 (CLI design): Implemented by Task 2.
- [ ] Section 15 (Graph spec): Implemented by Task 8.
- [ ] Section 16 (Semantic pruning): Implemented by Task 7 + Task 5 safe rules.
- [ ] Section 17 (Log parser priorities): Implemented by Task 5 parser packs.
- [ ] Section 18 (Tech stack): Implemented by Tasks 1, 6-8.
- [ ] Section 19 (Config): Implemented by Task 3.
- [ ] Section 20 (Installation/integrations): Implemented by Tasks 11-13, 15.
- [ ] Section 21 (Security/privacy): Implemented by Task 14.
- [ ] Section 22 (Metrics/benchmarking): Implemented by Task 16.
- [ ] Section 23 (MVP realism): Implemented by Task 17.
- [ ] Section 24 (Virality factors): Implemented by Task 18.
- [ ] Section 25 (Future extensions): Implemented by Task 19.
- [ ] Section 26 (Roadmap week-by-week): Mapped below.
- [ ] Section 27 (README one-liner): Implemented by Task 18.
- [ ] Section 28 (Conclusion differentiators): Implemented by combined architecture + docs strategy.

## 8) Delivery Schedule (Aligned to PDF + realistic buffer)

- [ ] Week 1: Tasks 1-3 + parser trio (`pytest`, traceback, `git diff`) and initial docs.
- [ ] Week 2: Tasks 4-6 + minimal graph schema.
- [ ] Week 3: Tasks 8-10 + pack JSON report.
- [ ] Week 4: Task 7 + explain mode + benchmark harness skeleton.
- [ ] Week 5: Tasks 11-13 + adapter stabilization.
- [ ] Week 6: Tasks 14-16 + measurable report publication.
- [ ] Week 7: Tasks 17-18 + public demo assets.
- [ ] Week 8+: Task 19 future extension execution by priority.

## 9) Risks and Mitigations

- [ ] Risk: semantic ranking complexity slows MVP.
Mitigation: release Phase 1 without ONNX hard dependency; keep semantic engine feature-flagged.

- [ ] Risk: graph drift and stale relationships.
Mitigation: enforce incremental update events + periodic `ctx graph rebuild` consistency checks.

- [ ] Risk: over-pruning breaks answer quality.
Mitigation: safe-pruning guardrails + explain mode + scenario regression tests.

- [ ] Risk: integration fragility across agents.
Mitigation: adapter contracts + snapshot tests + generic fallback adapter.

- [ ] Risk: claims not trusted publicly.
Mitigation: reproducible benchmarks, public task definitions, transparent metrics methodology.

## 10) Definition of Done (Project-Level)

- [ ] All five integration modes are operational.
- [ ] Both core subsystems (Pruner + Knowledge Graph) are production-usable locally.
- [ ] KPI report shows substantial token reduction without unacceptable latency overhead.
- [ ] At least one public benchmark/demo proves practical debugging/refactor benefit.
- [ ] Security defaults are local-safe and telemetry is opt-in.
- [ ] README and docs make onboarding possible in under 10 minutes.

## 11) Execution Order Recommendation

- [ ] Execute Tasks 1-6 first (stable MVP backbone).
- [ ] Execute Tasks 8-10 second (core differentiation).
- [ ] Execute Tasks 11-14 third (integrations and trust).
- [ ] Execute Tasks 15-19 fourth (distribution, credibility, growth).
