# CTX Demo Walkthrough

This walkthrough validates CTX on the in-repo fixture project:

- `demo/fixtures/opencode-auth-lab/`

## Goal

Demonstrate that CTX works end-to-end inside the OpenCode-first workflow:

- bootstrap a repo
- install OpenCode integration
- import AGENTS-style rules into graph memory
- query only the relevant directives
- prune noisy logs
- build compact context
- benchmark graph memory against markdown memory

## Setup

```bash
ctx --repo-root demo/fixtures/opencode-auth-lab init
ctx --repo-root demo/fixtures/opencode-auth-lab index
ctx --repo-root demo/fixtures/opencode-auth-lab opencode install
```

## OpenCode flow

Open the fixture repo in OpenCode and run:

```text
/ctx
/ctx-doctor
/ctx-memory-bootstrap
/ctx-memory-search auth root cause
/ctx-retrieve refresh token auth failure
/ctx-pack fix refresh token rotation
```

## Expected outcomes

- graph memory is populated from markdown seed files
- `/ctx` shows the categorized CTX command center and recommends a sensible next command
- topic search returns auth and testing rules without rereading the full markdown corpus
- retrieval surfaces the refresh-route and session code
- prune logs isolates the root assertion failure
- pack produces compact context with graph, memory, and recent signals

## Automated validation

```bash
scripts/demo/opencode-auth-lab-smoke.sh ./target/debug/ctx
scripts/demo/opencode-auth-lab-mcp-smoke.sh ./target/debug/ctx
scripts/demo/opencode-auth-lab-benchmark.sh ./target/debug/ctx
```
