# CTX Practical Guide

This guide shows how CTX should be used in real projects, what each flow does, and what output to expect.

## 1. First Setup In An Existing Repository

Use this when you want CTX to create its local runtime state and index the project.

```bash
ctx init
ctx index
```

Expected output:

```text
initialized: /path/to/project/.ctx/config.toml
indexed_files: 42
```

What happens:

- CTX creates `.ctx/config.toml`, `.ctx/graph.db`, `.ctx/packs/`, `.ctx/stats/` and `.ctx/audit.log`.
- CTX indexes source files into the local graph.
- Ignored directories such as `.git`, `.ctx`, `node_modules`, `target`, `build`, `dist`, `.next`, `.cache` and `coverage` are skipped.
- Sensitive-looking paths such as `.env`, `.pem`, `.key`, `credentials` and `secret` are excluded by default.

## 2. Build Context Without Running An Agent

Use this when you want to inspect the compact context before giving it to an AI CLI.

```bash
ctx ask "where is retry logic implemented?"
```

Expected output shape:

```text
query:
where is retry logic implemented?

symbols:
src/retry.rs::retry_with_backoff

memory:
[project:manual:testing.always_run] Run targeted tests before completion.
```

What happens:

- CTX retrieves relevant graph snippets, symbols, memory directives and recent local signals.
- CTX packs them under the configured token budget.
- No external agent is invoked.

## 3. Debug A Failing Test With Logs

Use this when a command produces noisy logs and you want only root-cause context.

```bash
pytest -q 2>&1 | ctx prune logs
```

Expected output shape:

```text
ERROR tests/test_auth.py::test_refresh_token_rotation
E AssertionError: expected token rotation
tests/test_auth.py:42
```

Then pack the failing log into context:

```bash
pytest -q 2>&1 > /tmp/pytest-failure.log
ctx pack "fix refresh token rotation" --attach /tmp/pytest-failure.log --json
```

Expected output fields:

```json
{
  "packed_tokens": 1200,
  "reduction_pct": 70.0,
  "included": ["query included: ...", "root_cause included: ..."],
  "excluded": [],
  "pack_path": ".ctx/packs/pack-....json",
  "compact_context": "query:\nfix refresh token rotation\n..."
}
```

What happens:

- `ctx prune logs` removes pass/progress noise.
- `ctx pack` combines the query, pruned root cause, graph retrieval, recent diff, dependencies and memory directives.
- CTX writes the pack artifact to `.ctx/packs/`.

## 4. Use CTX Inside Claude, Codex Or OpenCode

Use wrappers when you want CTX to prepare context and then invoke the real agent CLI.

```bash
ctx wrap claude --prompt "explain why this auth test is flaky"
ctx wrap codex --prompt "review the last diff and find risky changes"
ctx wrap opencode --prompt "implement caching for embeddings"
```

Direct adapter commands:

```bash
ctx claude "explain why this auth test is flaky"
ctx codex "review the last diff and find risky changes"
ctx opencode run "implement caching for embeddings"
```

Expected behavior:

- If the target CLI exists in `PATH`, CTX invokes it with compact context.
- If the target CLI is missing, CTX returns a prompt-safe fallback instead of losing the context.
- CTX records local stats in `.ctx/stats/latest.json`.
- CTX records invocation audit metadata in `.ctx/audit.log`.

Expected fallback output shape:

```json
{
  "agent": "claude",
  "status": "fallback",
  "fallback_used": true,
  "prompt_preview": "[CTX COMPACT CONTEXT]..."
}
```

## 5. Use CTX As A Pre-Prompt Hook

Use this when a CLI or editor can call a command before sending a user prompt.

```bash
ctx hook "fix failing pytest in auth" > /tmp/ctx-preprompt.txt
```

Expected output shape:

```text
Task:
fix failing pytest in auth

Compact Context:
query:
fix failing pytest in auth

Instruction:
Use the compact context above as the project-specific context.
```

What happens:

- CTX produces a ready-to-inject pre-prompt.
- The user or tool can prepend it to an agent prompt.

## 6. Replace AGENTS.md / CLAUDE.md / CODEX.md With Graph Memory

Use graph memory when project rules should be queryable, compact and testable instead of repeatedly pasted as markdown.

Create a directive manually:

```bash
ctx memory set testing.always_run "Run targeted tests before claiming completion." --scope project --source manual
```

Read it:

```bash
ctx memory get testing.always_run
```

Expected output:

```text
key: testing.always_run
scope: project
source: manual
body: Run targeted tests before claiming completion.
```

Import existing markdown rules:

```bash
ctx memory import --from AGENTS.md --scope project --source markdown --prefix agents
```

Expected output shape:

```json
{
  "markdown_path": "AGENTS.md",
  "scope": "project",
  "source": "markdown",
  "imported": 12,
  "keys": ["agents.1", "agents.2"]
}
```

Export graph memory back to markdown for audit/review:

```bash
ctx memory export --to AGENTS.generated.md --scope project --limit 200
```

Expected behavior:

- The graph stores compact directives with metadata.
- Retrieval and packing can include only the memory relevant to the current task.
- Markdown can still be imported/exported for compatibility.

## 7. Benchmark Graph Memory Against Markdown

Use this when you want to prove whether graph memory saves tokens and preserves useful instructions.

```bash
ctx benchmark memory-ab "run tests before completion" --markdown AGENTS.md --limit 20
```

Expected output fields:

```json
{
  "markdown_tokens": 2400,
  "graph_memory_tokens": 420,
  "token_reduction_pct": 82.5,
  "markdown_query_term_coverage": 0.75,
  "graph_query_term_coverage": 1.0,
  "graph_directives_count": 8
}
```

For quality scoring, add a checklist and two answer files:

```bash
ctx benchmark memory-ab "run tests before completion" \
  --markdown AGENTS.md \
  --limit 20 \
  --checklist quality-checklist.md \
  --markdown-answer baseline-answer.txt \
  --graph-answer ctx-answer.txt
```

Expected extra fields:

```json
{
  "markdown_success_rate": 0.7,
  "graph_success_rate": 0.9,
  "quality_winner": "graph",
  "quality_delta_pct": 20.0
}
```

## 8. Connect CTX To Claude Code Through MCP

Use this when an MCP-capable client should request project context dynamically.

```bash
ctx mcp config claude
```

Expected output shape:

```json
{
  "mcpServers": {
    "ctx": {
      "command": "ctx",
      "args": ["--repo-root", "/path/to/project", "mcp", "stdio"]
    }
  }
}
```

Manual stdio smoke test:

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n' | ctx mcp stdio
```

Expected output:

```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"ctx-mcp","version":"0.1.0"},"capabilities":{"tools":{},"resources":{}}}}
```

What happens:

- The MCP client launches CTX locally.
- CTX serves tools such as `get_relevant_context`, `project_map`, `search_symbols`, `recent_decisions`, `related_failures` and `get_compact_diff`.
- Project data stays inside the local CTX process unless the MCP client or agent independently sends it elsewhere.

## 9. Security Smoke Test

Use this to verify that sensitive attachments are blocked.

```bash
ctx init
printf 'API_KEY=secret\n' > .env
ctx pack "fix auth" --attach .env
cat .ctx/audit.log
```

Expected command error:

```text
attachment .env matches sensitive file patterns and was blocked
```

Expected audit event:

```json
{"kind":"privacy_decision","decision":"excluded","path":".env","reason":"sensitive_pattern","local_only":true,"remote_upload_enabled":false,"message":"blocked sensitive attachment before packing"}
```

What happens:

- CTX refuses to read the sensitive attachment into the pack.
- CTX records a local privacy decision in `.ctx/audit.log`.

## 10. Inspect Last Run Stats

Use this after `ctx pack`, `ctx claude`, `ctx codex`, `ctx opencode` or `ctx wrap`.

```bash
ctx stats
```

Expected output shape:

```json
{
  "original_tokens": 3200,
  "packed_tokens": 980,
  "reduction_pct": 69.37,
  "latency_ms": 120,
  "agent": "codex",
  "status": "succeeded",
  "fallback_used": false
}
```

What happens:

- CTX reads `.ctx/stats/latest.json`.
- Stats are local-only and are not uploaded by CTX.

## 11. Verify Installation With Doctor

Use this immediately after installing CTX or cloning a project.

```bash
ctx doctor
```

Expected output before initialization:

```text
CTX Doctor
config: missing
next: ctx init
```

Initialize and verify again:

```bash
ctx init
ctx doctor
```

Expected output after initialization:

```text
config: ok
graph: ok
audit_log: ok
local_only: true
remote_upload_enabled: false
next: ctx index
```

What happens:

- CTX checks local runtime files and privacy defaults.
- CTX tells the user the next useful command.
- This is the command to use in install smoke tests and support/debug reports.

## 12. Package And Smoke-Test A Release

Build the release artifact for the current platform:

```bash
scripts/release/build.sh
```

Expected output:

```text
Release artifact ready: dist/ctx-0.1.0-<target>.tar.gz
Checksum file: dist/SHA256SUMS
```

Smoke-test a binary directly:

```bash
scripts/release/install-smoke.sh ./target/release/ctx
```

Expected output:

```text
CTX install smoke passed: ./target/release/ctx
```

What happens:

- The smoke script creates a temporary project.
- It verifies `ctx help`, `ctx doctor`, `ctx init`, `ctx index`, `ctx pack`, `ctx stats` and `ctx mcp stdio`.
- The release script packages the binary and writes a checksum file.

## 13. Recommended Daily Workflow

```bash
ctx doctor
ctx init
ctx index
ctx memory import --from AGENTS.md --scope project --source markdown --prefix agents
pytest -q 2>&1 | ctx prune logs
ctx wrap codex --prompt "fix the failing auth test and run targeted tests"
ctx stats
```

Expected result:

- CTX uses compact project context instead of full markdown and noisy logs.
- The selected agent receives a smaller, more relevant prompt.
- The user can inspect pack artifacts, local stats and audit events afterward.
