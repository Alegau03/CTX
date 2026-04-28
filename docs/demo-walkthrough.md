# CTX Demo Walkthrough

This walkthrough validates CTX on the in-repo fixture project:

```text
demo/fixtures/opencode-auth-lab
```

## Goal

Show the full OpenCode-first CTX loop:

- bootstrap a repo
- install OpenCode integration
- import markdown rules into graph memory
- retrieve only relevant memory directives
- retrieve relevant code context
- prune noisy logs/diffs
- build compact task context
- benchmark graph memory against full markdown rules

## Setup

```bash
ctx --repo-root demo/fixtures/opencode-auth-lab init
ctx --repo-root demo/fixtures/opencode-auth-lab index
ctx --repo-root demo/fixtures/opencode-auth-lab opencode install
cd demo/fixtures/opencode-auth-lab
opencode
```

## OpenCode Flow

```text
/ctx
/ctx-doctor
/ctx-memory-bootstrap
/ctx-memory-search auth root cause
/ctx-retrieve refresh token auth failure
/ctx-pack fix refresh token rotation
/ctx-benchmark-memory-suite benchmarks/memory-suite.toml benchmarks/report.md benchmarks/report.json
```

## Expected Outcomes

- `/ctx` opens the command center.
- graph memory imports `AGENTS.md` and `.github/copilot-instructions.md`.
- memory search returns auth/testing directives without rereading the full markdown source.
- retrieval surfaces refresh-route/session/test files.
- pack produces compact context with graph, memory, and recent signals.
- benchmark reports show the markdown-vs-graph token delta.

## Automated Validation

```bash
scripts/demo/opencode-auth-lab-smoke.sh ./target/debug/ctx
scripts/demo/opencode-auth-lab-mcp-smoke.sh ./target/debug/ctx
scripts/demo/opencode-auth-lab-benchmark.sh ./target/debug/ctx
```
