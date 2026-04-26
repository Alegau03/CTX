# CTX Practical Guide

This guide is the operational manual for CTX.

It covers:

- the correct order to enable CTX in a repository
- the recommended OpenCode-first workflow
- the graph memory workflow starting from classic markdown files
- the command reference
- validation and expected behavior

## 1. Recommended Order In A Real Repository

Use CTX in this order:

1. Verify the project builds and tests normally.
2. Initialize CTX.
3. Index the repository.
4. Install the OpenCode integration.
5. Open the repository in OpenCode.
6. Use `/ctx-*` commands inside OpenCode.
7. Move project habits from markdown into graph memory.
8. Use CTX pack, retrieve, prune, and graph memory during daily work.
9. Use benchmarks only after the graph-memory workflow is in place.

The normal starting sequence is:

```bash
ctx init
ctx index
ctx opencode install
opencode
```

Expected result:

- `.ctx/` runtime exists
- repository graph is indexed
- `opencode.json` exists or is merged
- `.opencode/commands/` exists
- `.opencode/instructions/ctx-host-first.md` exists

## 2. OpenCode-First Workflow

CTX is meant to live inside OpenCode.

That means:

- keep your current OpenCode model and agent
- do not open a second terminal for normal CTX usage
- use `/ctx-*` commands when you need explicit CTX actions
- let CTX act as the local context runtime behind the session

Typical daily flow:

1. open `opencode`
2. run `/ctx-doctor` if the repo state is unclear
3. run `/ctx-retrieve`, `/ctx-graph-query`, or `/ctx-memory-search` to narrow context
4. run `/ctx-pack` when you want a compact task context
5. run `/ctx-prune-logs` or `/ctx-prune-diff` when logs or diffs are noisy
6. update graph memory when project rules change

## 3. First-Time Setup

Use this when CTX has never been initialized in the repository.

```bash
ctx init
ctx index
ctx opencode install
```

Expected output shape:

```text
initialized: /path/to/project/.ctx/config.toml
indexed_files: 42
installed OpenCode integration
```

What happens:

- CTX creates `.ctx/config.toml`, `.ctx/graph.db`, `.ctx/packs/`, `.ctx/stats/`, and `.ctx/audit.log`
- CTX indexes code files into the graph
- CTX extracts symbols and slices code for Rust, Python, TypeScript, and JavaScript
- OpenCode-local integration files are generated for repo-native usage

## 4. First Commands To Run Inside OpenCode

After `ctx opencode install`, open `opencode` and start with:

```text
/ctx-doctor
/ctx-memory-bootstrap
/ctx-memory-search tests
/ctx-retrieve auth refresh token
/ctx-pack fix failing auth test
```

What each one should do:

- `/ctx-doctor`: confirm CTX is ready in the repo
- `/ctx-memory-bootstrap`: import project rules from classic markdown files into graph memory
- `/ctx-memory-search tests`: retrieve only directives relevant to testing
- `/ctx-retrieve ...`: find relevant files, snippets, and symbols
- `/ctx-pack ...`: build compact task context from graph, memory, recent signals, and optional attachments

## 5. Build Context Without Running Another Agent

Use this when you want to inspect the compact context directly.

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

## 6. Debug Noisy Logs

Use this when logs are too noisy for direct use.

```bash
pytest -q 2>&1 | ctx prune logs
```

Expected output shape:

```text
ERROR tests/test_auth.py::test_refresh_token_rotation
E AssertionError: expected token rotation
tests/test_auth.py:42
```

Then build a pack:

```bash
pytest -q 2>&1 > /tmp/pytest-failure.log
ctx pack "fix refresh token rotation" --attach /tmp/pytest-failure.log --json
```

Expected JSON fields:

```json
{
  "packed_tokens": 1200,
  "reduction_pct": 70.0,
  "pack_path": ".ctx/packs/pack-....json",
  "compact_context": "query:\nfix refresh token rotation\n..."
}
```

## 7. Graph Memory Workflow

This is the most important workflow for replacing `AGENTS.md`-style files.

### 7.1 Bootstrap From Existing Markdown

If the repository already has classic instruction files, import them first.

Automatic bootstrap:

```bash
ctx memory bootstrap
```

This scans conventional files such as:

- `AGENTS.md`
- `CLAUDE.md`
- `CODEX.md`
- `.github/copilot-instructions.md`

Expected output shape:

```text
imported_files=2 imported_directives=3
- /repo/AGENTS.md => 2 directives
- /repo/.github/copilot-instructions.md => 1 directives
```

Manual single-file import is still available:

```bash
ctx memory import --from AGENTS.md --scope project --source markdown --prefix agents
```

Expected JSON shape:

```json
{
  "markdown_path": "AGENTS.md",
  "scope": "project",
  "source": "markdown",
  "imported": 12,
  "keys": ["agents.1", "agents.2"]
}
```

### 7.2 Query Only Relevant Rules

Use topic search instead of rereading everything.

```bash
ctx memory search "auth tests root cause" --scope project --limit 10
```

Expected output:

```text
auth.root_cause [project:manual] Fix auth root cause instead of bypassing refresh token failures.
testing.always_run [project:markdown] Run targeted tests before completion.
```

This is the key value proposition of graph memory:

- only the relevant directives are retrieved
- unrelated instructions stay out of context
- token usage drops compared to reusing a full markdown file every time

### 7.3 Add Or Change Directives

Create or update a directive:

```bash
ctx memory set testing.always_run "Run targeted tests before claiming completion." --scope project --source manual
```

Read one directive:

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

List directives:

```bash
ctx memory list --scope project --limit 20
```

Delete a directive:

```bash
ctx memory delete testing.always_run
```

### 7.4 Export Back To Markdown

Use this for compatibility, audit, or review.

```bash
ctx memory export --to AGENTS.generated.md --scope project --limit 200
```

Expected behavior:

- graph memory is written back as markdown
- export is useful for review and compatibility
- graph memory remains the primary structured source of truth

### 7.5 Reimport Behavior

If you reimport the same markdown source or bootstrap again, CTX replaces stale directives for that prefix instead of accumulating obsolete copies.

This matters because it keeps graph memory clean while you migrate away from classic markdown files.

## 8. Benchmark Graph Memory Against Markdown

Use this after the graph-memory workflow is in place.

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

For quality scoring:

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

Reusable suite:

```bash
ctx benchmark memory-suite \
  --spec benchmarks/memory-ab.example.toml \
  --report-out benchmarks/report.md \
  --json-out benchmarks/report.json
```

## 9. Command Reference

### 9.1 Bootstrap And Repo Setup

- `ctx init`: initialize the local runtime
- `ctx index [paths...]`: index the repository or selected paths
- `ctx reindex [paths...]`: rerun indexing for selected paths
- `ctx graph build`: build graph data by indexing the repo
- `ctx graph rebuild`: explicit rebuild alias
- `ctx doctor`: show CTX readiness for the current repo
- `ctx help`: show the full command guide

### 9.2 OpenCode-First Commands

Use these after `ctx opencode install` inside OpenCode:

- `/ctx-doctor`
- `/ctx-pack`
- `/ctx-retrieve`
- `/ctx-graph-query`
- `/ctx-memory-bootstrap`
- `/ctx-memory-search`
- `/ctx-memory-set`
- `/ctx-memory-get`
- `/ctx-memory-list`
- `/ctx-memory-delete`
- `/ctx-memory-import`
- `/ctx-memory-export`
- `/ctx-prune-logs`
- `/ctx-prune-diff`
- `/ctx-benchmark-memory-ab`
- `/ctx-benchmark-memory-suite`
- `/ctx-stats`

### 9.3 Context And Retrieval

- `ctx ask <query>`
- `ctx pack <query> [--json] [--attach file] [--budget n]`
- `ctx hook <query>`
- `ctx explain <query>`
- `ctx retrieve <query> [--limit n]`
- `ctx graph query <query>`

### 9.4 Graph Memory

- `ctx memory bootstrap [paths...]`
- `ctx memory search <query> [--scope s] [--limit n]`
- `ctx memory set <key> <body> [--scope s] [--source src]`
- `ctx memory get <key>`
- `ctx memory list [--scope s] [--limit n]`
- `ctx memory delete <key>`
- `ctx memory import --from <file> [--scope s] [--source src] [--prefix p]`
- `ctx memory export --to <file> [--scope s] [--limit n]`

### 9.5 Pruning

- `ctx prune logs`
- `ctx prune diff --query "..."`

### 9.6 MCP And Host Bootstrap

- `ctx opencode install`
- `ctx codex install`
- `ctx claude install`
- `ctx mcp serve --port 8765`
- `ctx mcp stdio`
- `ctx mcp config opencode`
- `ctx mcp config codex`
- `ctx mcp config claude`

### 9.7 Benchmark, Stats, And Security

- `ctx benchmark memory-ab ...`
- `ctx benchmark memory-suite ...`
- `ctx stats`
- `ctx pack "..." --attach .env` to verify sensitive attachment blocking

## 10. Validation And Expected Behavior

### Full test suite

```bash
cargo test --workspace
```

Expected behavior:

- all tests pass

### OpenCode bootstrap smoke

```bash
scripts/release/opencode-smoke.sh ./target/release/ctx
```

Expected behavior:

- validates `ctx opencode install`
- validates `opencode.json`
- validates `.opencode/commands/`
- validates `.opencode/instructions/ctx-host-first.md`

### Security smoke

```bash
ctx init
printf 'API_KEY=secret\n' > .env
ctx pack "fix auth" --attach .env
cat .ctx/audit.log
```

Expected behavior:

- CTX blocks the sensitive attachment
- CTX records a `privacy_decision` audit event locally

## 11. When To Use Which Path

Use the OpenCode path for normal work:

```bash
ctx init
ctx index
ctx opencode install
opencode
```

Use the Codex or Claude bootstrap only when you explicitly want those host-native integrations.

Use raw CLI commands outside OpenCode mainly for:

- initial bootstrap
- indexing
- CI or smoke flows
- manual debugging of CTX itself
- benchmark/report generation
