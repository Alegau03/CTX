# CTX

Local-first context runtime for coding agents.

CTX reduces prompt noise, preserves high-signal project knowledge, and replaces large instruction markdown files with queryable graph memory that can be retrieved only when relevant.

## What CTX Is

CTX is not a replacement for your host agent CLI.

The primary product direction is OpenCode-first:

- open `opencode`
- keep the host-selected model and agent
- use CTX from inside OpenCode through `/ctx-*`
- let CTX provide graph memory, retrieval, pruning, compact packing, diagnostics, and benchmark tooling

CTX can also bootstrap native integrations for Codex and Claude Code, but OpenCode is the main product path.

## Why It Exists

Modern coding agents often waste context budget on:

- huge logs
- broad diffs
- repeated project rules
- large `AGENTS.md` or `CLAUDE.md` files that must be reread over and over

CTX turns those signals into local structured runtime data:

- graph-backed code understanding
- graph memory directives
- compact task packs
- explainable pruning
- local MCP tools

The goal is simple: pass less noise, keep more signal, and make the useful project context cheaper to retrieve.

## Core Idea: Graph Memory

CTX treats project habits and instructions as structured memory instead of one large markdown blob.

That means a host can retrieve only the directives relevant to the current task.
For example, if the task is about tests, CTX can surface only the testing-related directives instead of forcing the model to reread an entire `AGENTS.md` file.

Today CTX supports:

- importing existing markdown rule files such as `AGENTS.md`, `CLAUDE.md`, `CODEX.md`, and `.github/copilot-instructions.md`
- storing them as graph memory directives
- querying them by topic
- editing them through CTX commands
- exporting them back to markdown when compatibility is needed

## What Works Today

Implemented and usable today:

- Rust multi-crate workspace with a working `ctx` binary
- local runtime bootstrap through `.ctx/config.toml`
- deterministic log and diff pruning with parser packs and explainable provenance
- advanced context packing with strict priority ordering and pack artifacts
- SQLite graph with symbols, edges, snippet FTS, recent failures, recent decisions, and graph memory directives
- cross-file dependency and call-graph enrichment from indexed symbol bodies
- AST and symbol extraction for Rust, Python, TypeScript, and JavaScript
- hybrid retrieval with graph, FTS, and semantic ranking with explicit local fallback
- structured recent diff summaries with changed symbol extraction
- graph memory CRUD, topic search, markdown bootstrap/import/export, and A/B benchmark support
- local MCP runtime over HTTP JSON-RPC and stdio
- security and privacy controls with local-only defaults, sensitive file blocking, and audit logging
- repo-local OpenCode bootstrap through `ctx opencode install`
- generated OpenCode slash-command surface under `.opencode/commands/`
- repo-local Codex bootstrap through `ctx codex install`
- repo-local Claude Code bootstrap through `ctx claude install`

## OpenCode-First Usage

The supported daily path is:

```bash
ctx init
ctx index
ctx opencode install
opencode
```

After that, stay inside OpenCode and use `/ctx-*` commands.

The full usage order, command reference, examples, and expected outputs live in [guide.md](guide.md).

## Host Integrations

### OpenCode

`ctx opencode install`:

- creates or merges `opencode.json`
- registers CTX as a local MCP server through `ctx --repo-root <repo> mcp stdio`
- generates `.opencode/commands/*.md`
- generates `.opencode/instructions/ctx-host-first.md`
- preserves the host-selected OpenCode model because generated commands do not pin `agent` or `model`

### Codex

`ctx codex install` writes:

- `.codex/config.toml`
- `.agents/skills/ctx-*/SKILL.md`

### Claude Code

`ctx claude install` writes:

- `.mcp.json`
- `.claude/skills/ctx-*/SKILL.md`

## MCP Runtime

CTX exposes local MCP-compatible tools so host CLIs can request only the context they need.

Current transports:

- HTTP JSON-RPC on `127.0.0.1`
- stdio MCP for host-launched local processes

Representative tool surface:

- `get_relevant_context`
- `project_map`
- `search_symbols`
- `related_failures`
- `recent_decisions`
- `get_compact_diff`
- `memory_list`
- `memory_search`
- `memory_set`
- `memory_import_markdown`
- `memory_bootstrap_markdown`

## Security Model

CTX is local-first by default.

Important defaults:

- `local_only = true`
- `remote_upload_enabled = false`
- sensitive-looking files are blocked from pack attachments by default
- privacy decisions are recorded locally in `.ctx/audit.log`

See [docs/security.md](docs/security.md) for the full threat model and privacy behavior.

## Documentation

Start here:

- [guide.md](guide.md): usage order, commands, examples, expected outputs, OpenCode-first workflow
- [docs/install.md](docs/install.md): installation and release smoke flow
- [docs/opencode-integration.md](docs/opencode-integration.md): OpenCode-native architecture target
- [docs/security.md](docs/security.md): privacy defaults, threat model, and audit behavior
- [docs/guidelines.md](docs/guidelines.md): product and architectural guardrails
- [docs/codex-integration.md](docs/codex-integration.md): Codex-native bootstrap path
- [docs/claude-integration.md](docs/claude-integration.md): Claude Code native bootstrap path
- [docs/superpowers/plans/2026-04-25-final-release-roadmap.md](docs/superpowers/plans/2026-04-25-final-release-roadmap.md): current roadmap to release

## Status

Current implementation status:

- Phase 1: OpenCode-first product surface complete
- Phase 2: wrapper cleanup complete
- Phase 3: native host bootstraps complete
- Phase 4: analysis and retrieval quality complete
- Next focus: real-world demo validation, benchmark publication, and release polish

## Workspace Layout

- `crates/ctx-cli`: `ctx` binary and host/bootstrap command surface
- `crates/ctx-core`: orchestration for indexing, packing, memory, and benchmarking
- `crates/ctx-config`: config parsing and runtime bootstrap
- `crates/ctx-prune`: deterministic log and diff pruning
- `crates/ctx-pack`: context rewriting and budget-aware packing
- `crates/ctx-graph`: SQLite storage and query layer
- `crates/ctx-intake`: query normalization and intent detection
- `crates/ctx-ast`: structural parsing and slicing
- `crates/ctx-semantic`: semantic ranking and embedding backend handling
- `crates/ctx-telemetry`: stats and benchmark summary output
- `crates/ctx-hooks`: hook and pre-prompt helpers
- `crates/ctx-mcp`: MCP server runtime
- `crates/ctx-token`: token estimation helpers

## Source Material

This repository is being implemented from:

- `CTX_description.pdf`
- `docs/superpowers/plans/2026-04-23-ctx-runtime-engine.md`

The current release direction is tracked in:

- [docs/superpowers/plans/2026-04-25-final-release-roadmap.md](docs/superpowers/plans/2026-04-25-final-release-roadmap.md)
