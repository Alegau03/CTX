# CTX Final Release Roadmap

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship CTX as an OpenCode-first local context runtime that users can install in a repository and use naturally from inside the host CLI, with future non-OpenCode support added through native host integrations rather than wrapper-first UX.

**Architecture:** CTX remains a local Rust runtime with graph, retrieval, packing, memory, telemetry, and MCP capabilities. The primary product surface becomes host-native integration inside OpenCode through repo-local bootstrap, MCP stdio, generated slash commands, and host-first instructions; old wrapper-first public entrypoints stay removed while future host work is added natively.

**Tech Stack:** Rust workspace, Clap CLI, SQLite/FTS, tree-sitter, ONNX feature-gated local embeddings, JSON-RPC/MCP stdio + HTTP, OpenCode project config and command markdown files.

---

## File Map

### Core product/runtime

- `crates/ctx-cli/src/main.rs`
- `crates/ctx-core/**`
- `crates/ctx-adapters/**`
- `crates/ctx-mcp/**`
- `crates/ctx-pack/**`
- `crates/ctx-graph/**`
- `crates/ctx-ast/**`
- `crates/ctx-semantic/**`
- `crates/ctx-prune/**`

### Host integration and docs

- `README.md`
- `guide.md`
- `docs/install.md`
- `docs/guidelines.md`
- `docs/opencode-integration.md`
- `docs/security.md`
- `docs/superpowers/plans/2026-04-23-ctx-runtime-engine.md`
- `docs/superpowers/plans/2026-04-24-opencode-host-first.md`

### Test coverage

- `crates/ctx-cli/tests/cli_behavior.rs`
- `crates/ctx-cli/tests/opencode_host_integration_spec.rs`
- `crates/ctx-core/tests/**`
- `crates/ctx-mcp/tests/**`
- `scripts/release/**`

## Current Baseline

- [x] OpenCode repo-local bootstrap exists through `ctx opencode install`
- [x] `opencode.json` merge works and preserves non-CTX MCP servers
- [x] `.opencode/commands/*.md` generation exists for the current CTX surface
- [x] OpenCode keeps the host-selected model because generated commands do not force `agent` or `model`
- [x] Wrapper-first public CLI entrypoints have been removed
- [x] OpenCode repo-local host-first usage rules are generated and loaded
- [x] Wrapper de-emphasis/deprecation is complete in the primary docs and public CLI surface
- [x] Codex host-native bootstrap exists through `ctx codex install`
- [x] Claude Code host-native bootstrap exists through `ctx claude install`
- [ ] Benchmark publishing pipeline is not complete
- [ ] Release pipeline is not complete

## Phase 1: Finish OpenCode-First Product Surface

- [x] Expand generated OpenCode commands to cover every daily CTX action that should be reachable from the TUI
- [x] Add repo-local host-first instructions so OpenCode prefers CTX graph/memory/retrieval before broad file dumping
- [x] Add OpenCode-native smoke scenarios that validate slash commands, MCP bootstrap, and non-regression of host model ownership
- [x] Tighten docs so OpenCode-native usage is always taught before any fallback or legacy references
- [x] Document the exact “open `opencode`, then use `/ctx-*`” workflow as the primary onboarding path

**Exit criteria**

- OpenCode users can install CTX in a repo with one command
- OpenCode users can use CTX daily from inside the TUI
- Docs treat OpenCode-native usage as the only primary path

## Phase 2: Consolidate Wrapper Removal

- [x] Remove `ctx wrap`, `ctx codex`, `ctx claude`, and `ctx opencode run` from the public CLI
- [x] Move the user-facing documentation to the OpenCode-native path
- [x] Remove stale references and leftover wrapper-first assumptions from active docs/tests/plans
- [x] Keep only the backend pieces that still matter for future native host integrations

**Design rule**

- Do not reintroduce wrapper-first public UX for new hosts when native integration points are available

**Status**

- Phase 2 is complete after the documentation and historical-plan cleanup.
- `crates/ctx-adapters` remains only as internal compatibility/backend code, not as a public wrapper-first product surface.

## Phase 3: Add New Host-Native Integrations After OpenCode

- [x] Add a shared host integration abstraction so host-specific bootstraps are not implemented ad hoc
- [x] Implement Codex-native bootstrap/integration path after the OpenCode flow is stable
- [x] Refine Claude-native bootstrap/integration path to match the host-first model
- [x] Keep host-specific docs separate from the core runtime docs
- [x] Avoid adding new daily wrapper-first UX; new host work should prefer native integration points over fresh wrappers

**Exit criteria**

- OpenCode is first-class
- Codex and Claude each have a clear native path or a documented compatibility limitation

**Status**

- Phase 3 is complete.
- OpenCode remains the primary product path.
- Codex now uses project-local MCP plus `.agents/skills/ctx-*/SKILL.md`.
- Claude Code now uses project-local MCP plus `.claude/skills/ctx-*/SKILL.md`.

## Phase 4: Deepen Static Analysis and Retrieval Quality

- [x] Extend AST/symbol extraction beyond the original Rust/Python baseline to cover TypeScript and JavaScript
- [x] Improve dependency/call graph precision beyond current heuristics
- [x] Complete parser pack coverage for important tools still partial
- [x] Enrich packer inputs with more structured recent diff, task memory, and failure memory sections
- [x] Keep explainability and token budgeting visible while improving depth

**Exit criteria**

- CTX meaningfully outperforms a simple local grep/markdown strategy on realistic repositories

**Status**

- Phase 4 is complete.
- Cross-file dependency and call relationships are now enriched from indexed symbol bodies instead of only same-file neighbors.
- Parser packs now cover the critical missing diagnostic families added during this phase.
- The packer now carries richer structured diff and memory context without hiding budget/explainability behavior.

## Phase 5: Close Benchmark and Validation Story

- [x] Build a publishable memory benchmark suite harness with Markdown/JSON report generation
- [x] Add reusable benchmark scenario definitions and repeatable report generation for graph-memory validation
- [ ] Run A/B benchmarks against markdown memory and version the reports
- [ ] Add validation that graph memory actually saves tokens while preserving answer quality
- [ ] Use benchmark results to justify host-first positioning and graph-memory adoption

**Exit criteria**

- The repo contains evidence, not just claims, for token savings and quality retention

## Phase 6: Packaging, Release, and Community Readiness

- [ ] Finish binary packaging and release pipeline
- [ ] Finalize Homebrew/tap story once public release coordinates exist
- [ ] Produce final install docs, usage guide, and real-world walkthroughs
- [ ] Add demo/community assets and release messaging
- [ ] Execute final QA pass focused on OpenCode-native flows

**Exit criteria**

- A user can discover, install, test, and use CTX from GitHub without hidden setup steps

## Final Release Checklist

- [ ] OpenCode-native usage is the primary documented and tested path
- [ ] Future non-OpenCode host integrations are specified natively, not as wrapper revivals
- [ ] Codex/Claude roadmap is explicit
- [ ] Benchmark reports are published
- [ ] Release artifacts are reproducible
- [ ] README, guide, install docs, and roadmap all agree on current status

## Execution Note

Phase ordering remains the intended release order.

- Phase 2 is now complete.
- Phase 3 is now complete.
- Phase 4 is now complete.
