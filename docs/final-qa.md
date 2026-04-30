# OpenCode-Native Final QA

## Goal

Confirm that a new user can install CTX, bootstrap a repository, open OpenCode, and use graph memory without wrapper-style detours.

## Automated Gate

```bash
scripts/release/final-qa.sh
```

This builds and verifies the release archive, then reruns install, OpenCode, demo, MCP, and benchmark validations.

## Manual Fixture QA

```bash
ctx --repo-root demo/fixtures/opencode-auth-lab init
ctx --repo-root demo/fixtures/opencode-auth-lab index
ctx --repo-root demo/fixtures/opencode-auth-lab opencode install
cd demo/fixtures/opencode-auth-lab
npm install
opencode mcp list --print-logs --log-level DEBUG --pure
opencode
```

Expected before opening the TUI:

- `ctx connected`
- `toolCount=13`

Inside OpenCode, run:

```text
/ctx
/ctx-doctor
/ctx-memory-bootstrap
/ctx-memory-search auth
/ctx-retrieve refresh route
/ctx-pack fix refresh token bug
/ctx-prune-logs npm run test:auth
/ctx-benchmark-memory-suite benchmarks/memory-suite.toml benchmarks/report.md benchmarks/report.json
```

## Expected Results

- `/ctx` shows the categorized command center.
- `/ctx-doctor` reports `ready: true` after `init` + `index`.
- if `/ctx-doctor` also shows `next: ...`, treat it as the recommended workflow step, not proof that CTX is broken.
- OpenCode sees CTX commands from `.opencode/commands/`.
- `opencode.json` registers CTX as a local MCP server.
- graph memory imports `AGENTS.md`, `CLAUDE.md`, `CODEX.md`, and Copilot instructions when present for `27` directives in the fixture.
- memory search returns relevant directives only.
- `/ctx-prune-logs npm run test:auth` keeps the refresh-token assertion failure readable after `npm install`.
- pack output includes graph and memory context instead of a giant markdown dump.
- benchmark reports regenerate successfully with `56.72%` token reduction and a graph quality win on the fixture.

## Real Repo QA

On a separate real project:

```bash
ctx init
ctx index
ctx opencode install
opencode
```

Inside OpenCode:

```text
/ctx
/ctx-doctor
/ctx-memory-bootstrap
/ctx-memory-search tests
/ctx-pack <real task>
```

A release is ready when the fixture and one real project both pass this flow.
