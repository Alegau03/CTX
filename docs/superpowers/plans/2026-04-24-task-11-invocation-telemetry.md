# Task 11 Invocation + Telemetry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Task 11 by making CTX wrappers production-ready for Codex, Claude Code, OpenCode, and generic CLI adapters while recording fully local invocation metrics, stats, and audit evidence.

**Architecture:** Keep CTX as a local-first runtime layer that prepares compact context, forwards it to the user's chosen coding agent, and records only local metadata in `.ctx/`. The adapter layer owns command templates and process execution, the graph layer owns durable `runs` metadata, the telemetry layer owns stats/audit files, and the CLI only formats results for humans or JSON automation.

**Tech Stack:** Rust workspace, `clap`, `serde`, `serde_json`, `rusqlite`, local SQLite, local `.ctx/stats/latest.json`, local `.ctx/audit.log`, `assert_cmd` integration tests, fake test binaries for adapter smoke tests.

---

## Source Alignment

This plan covers Task 11 from `docs/superpowers/plans/2026-04-23-ctx-runtime-engine.md`, current gaps in `README.md`, and the adapter/telemetry requirements in `CTX_description.pdf`.

Requirements locked for this task:

- CTX remains an extension layer for existing CLI agents, not a replacement agent.
- `ctx codex "..."` must execute Codex non-interactively with CTX compact context.
- `ctx claude "..."` must execute Claude Code non-interactively with CTX compact context.
- `ctx opencode run "..."` must execute OpenCode non-interactively with CTX compact context.
- The wrappers must not disable native agent capabilities by default, including user settings, plugins, skills, hooks, MCP configuration, and provider selection.
- Missing binaries must produce a useful fallback prompt instead of failing the CTX context-building workflow.
- Non-zero agent exits must be recorded as failed runs and returned as CLI errors.
- Invocation metadata must be persisted in the local SQLite `runs` table.
- `ctx stats` must report token reduction and local latency overhead, including the latest adapter invocation when available.
- Audit logging must stay local and capture pack/prune/include/exclude/invocation decisions.
- Telemetry stays local and disabled for remote upload. Task 11 writes local stats only.
- JSON output must be stable enough for automation.
- Tests must prove command templates, execution fallback, run metadata, stats, audit, and CLI behavior.

External CLI contracts verified on 2026-04-24:

- Codex non-interactive mode uses `codex exec "prompt"`.
- Claude Code print mode uses `claude -p "prompt"` or `claude --print "prompt"`.
- OpenCode non-interactive mode uses `opencode run [message..]`.

## Current State

Existing implementation already has these pieces:

- `crates/ctx-adapters/src/lib.rs` can prepare basic `AdapterInvocation` values.
- `ctx codex "..."` and `ctx opencode run "..."` build a context pack and try to execute a binary.
- `ctx claude "..."` only prints `adapter=claude` plus compact context and does not execute Claude Code.
- `ctx stats` prints `.ctx/stats/latest.json` created by `run_pack`.
- `GraphStore::record_run(command, status)` stores only `command`, `status`, and `created_at`.
- `.ctx/audit.log` receives a plain `run_pack` audit line from `ctx-core`.

Task 11 is complete only when those pieces become one coherent, tested invocation telemetry system.

## File Structure

Modify these files:

- `crates/ctx-adapters/Cargo.toml`: add dev dependencies needed for fake binary tests.
- `crates/ctx-adapters/src/lib.rs`: define stable adapter templates, prompt passing, execution result, fallback result, and execution helpers.
- `crates/ctx-adapters/src/codex.rs`: use `codex exec` template.
- `crates/ctx-adapters/src/claude.rs`: use `claude -p` template.
- `crates/ctx-adapters/src/opencode.rs`: keep `opencode run` template.
- `crates/ctx-adapters/src/generic.rs`: support configurable generic command through explicit builder input.
- `crates/ctx-adapters/tests/adapters.rs`: expand template, prompt, fallback, and fake binary execution coverage.
- `crates/ctx-graph/src/schema.sql`: extend `runs` for invocation metadata while preserving existing columns.
- `crates/ctx-graph/src/lib.rs`: add migration helper, `RunRecord`, `RunInsert`, `record_invocation_run`, `recent_runs`, and keep `record_run` backwards-compatible.
- `crates/ctx-graph/tests/graph_features.rs`: test new runs metadata and old schema migration.
- `crates/ctx-telemetry/src/lib.rs`: re-export new modules and preserve existing public API.
- `crates/ctx-telemetry/src/stats.rs`: move stats snapshot read/write and add invocation-aware stats fields.
- `crates/ctx-telemetry/src/audit.rs`: add structured local audit JSONL-compatible records while keeping human-readable lines.
- `crates/ctx-telemetry/tests/stats.rs`: test extended stats compatibility.
- `crates/ctx-telemetry/tests/audit.rs`: test local audit append/read behavior.
- `crates/ctx-core/Cargo.toml`: add `ctx-adapters` dependency to orchestrate invocations outside the CLI.
- `crates/ctx-core/src/lib.rs`: add `run_agent_invocation`, local stats writing, graph run registration, audit event writing, and result serialization.
- `crates/ctx-core/tests/core_behavior.rs`: test adapter invocation through fake binaries and fallback metadata.
- `crates/ctx-cli/src/main.rs`: route `codex`, `claude`, and `opencode run` through the same core invocation path and support `--json` output.
- `crates/ctx-cli/tests/cli_behavior.rs`: add end-to-end wrapper tests with fake binaries and stats assertions.
- `README.md`: update command descriptions, Test section, Task 11 status, and execution queue.

Do not create `tests/e2e/test_adapter_invocation.rs` as a Python-style test file because this repository currently uses Rust integration tests inside crate `tests/` directories. The acceptance coverage belongs in `crates/ctx-cli/tests/cli_behavior.rs`, `crates/ctx-core/tests/core_behavior.rs`, and `crates/ctx-adapters/tests/adapters.rs`.

## Design Decisions

Adapter command templates:

```text
codex    -> program: codex,    args: ["exec"], prompt as final argument
claude   -> program: claude,   args: ["-p"],   prompt as final argument
opencode -> program: opencode, args: ["run"],  prompt as final argument
generic  -> program and args parsed from an explicit command template, prompt as final argument
```

The Claude adapter must not add `--bare` by default because that would skip native discovery of hooks, skills, plugins, MCP servers, auto memory, and project docs. CTX's default behavior should preserve the host agent's normal capabilities.

The Codex adapter must not add `--skip-git-repo-check` by default because Codex's git-repository check is a safety feature. Tests should use fake binaries instead of weakening real safety defaults.

The OpenCode adapter should keep `opencode run` because the official CLI supports non-interactive prompt execution through that command.

The generic adapter is configuration-oriented and does not need a first-class public CLI command in Task 11. It must be available as a library contract so Task 12 can expose `ctx wrap <agent> --prompt ...` cleanly.

## Data Contracts

### AdapterInvocation

Target final shape:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterInvocation {
    pub agent: Agent,
    pub program: String,
    pub args: Vec<String>,
    pub prompt: String,
}
```

No environment secrets are serialized into invocation results.

### AdapterExecutionResult

Target final shape:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterExecutionResult {
    pub agent: Agent,
    pub command: String,
    pub status: AdapterRunStatus,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub fallback_used: bool,
    pub fallback_reason: Option<String>,
}
```

### AdapterRunStatus

Target final shape:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterRunStatus {
    Succeeded,
    Failed,
    Fallback,
}
```

### Core AdapterRunReport

Target final shape:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct AdapterRunReport {
    pub agent: String,
    pub command: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub fallback_used: bool,
    pub fallback_reason: Option<String>,
    pub original_tokens: usize,
    pub packed_tokens: usize,
    pub reduction_pct: f64,
    pub pack_path: Option<String>,
    pub prompt_preview: Option<String>,
}
```

`prompt_preview` is populated only for fallback/human output so the user can copy/use the prepared prompt when the real CLI is missing.

### Runs Table

Target final `runs` columns:

```sql
CREATE TABLE IF NOT EXISTS runs (
  id INTEGER PRIMARY KEY,
  task_id INTEGER,
  command TEXT NOT NULL,
  status TEXT NOT NULL,
  agent TEXT,
  exit_code INTEGER,
  duration_ms INTEGER,
  original_tokens INTEGER,
  packed_tokens INTEGER,
  reduction_pct REAL,
  fallback_used INTEGER NOT NULL DEFAULT 0,
  pack_path TEXT,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

Existing databases must be migrated in-place by `GraphStore::init_schema()`.

### Stats Snapshot

Target final fields:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSnapshot {
    pub original_tokens: usize,
    pub packed_tokens: usize,
    pub reduction_pct: f64,
    pub latency_ms: u64,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub fallback_used: bool,
    #[serde(default)]
    pub pack_path: Option<String>,
}
```

Existing JSON snapshots without adapter fields must continue to deserialize.

## Task 11.1: Adapter Templates And Prompt Contract

**Files:**

- Modify: `crates/ctx-adapters/src/lib.rs`
- Modify: `crates/ctx-adapters/src/codex.rs`
- Modify: `crates/ctx-adapters/src/claude.rs`
- Modify: `crates/ctx-adapters/src/opencode.rs`
- Modify: `crates/ctx-adapters/src/generic.rs`
- Modify: `crates/ctx-adapters/tests/adapters.rs`

- [ ] **Step 1: Write failing template tests**

Add these tests to `crates/ctx-adapters/tests/adapters.rs`:

```rust
#[test]
fn codex_uses_non_interactive_exec_template() {
    let invocation = prepare_invocation(Agent::Codex, "review diff", "compact ctx");
    assert_eq!(invocation.program, "codex");
    assert_eq!(invocation.args, vec!["exec".to_string()]);
    assert!(invocation.prompt.contains("review diff"));
    assert!(invocation.prompt.contains("[CTX COMPACT CONTEXT]"));
    assert!(invocation.prompt.contains("compact ctx"));
}

#[test]
fn claude_uses_print_mode_without_bare_mode() {
    let invocation = prepare_invocation(Agent::Claude, "fix flaky test", "compact ctx");
    assert_eq!(invocation.program, "claude");
    assert_eq!(invocation.args, vec!["-p".to_string()]);
    assert!(!invocation.args.iter().any(|arg| arg == "--bare"));
    assert!(invocation.prompt.contains("fix flaky test"));
    assert!(invocation.prompt.contains("compact ctx"));
}

#[test]
fn opencode_uses_run_template() {
    let invocation = prepare_invocation(Agent::OpenCode, "explain build", "compact ctx");
    assert_eq!(invocation.program, "opencode");
    assert_eq!(invocation.args, vec!["run".to_string()]);
    assert!(invocation.prompt.contains("explain build"));
}
```

- [ ] **Step 2: Run adapter tests and verify failure**

Run:

```bash
cargo test -p ctx-adapters
```

Expected:

```text
codex_uses_non_interactive_exec_template ... FAILED
claude_uses_print_mode_without_bare_mode ... FAILED
```

- [ ] **Step 3: Update Codex template**

Change `crates/ctx-adapters/src/codex.rs` so `prepare()` returns `args: vec!["exec".to_string()]`.

- [ ] **Step 4: Update Claude template**

Change `crates/ctx-adapters/src/claude.rs` so `prepare()` returns `args: vec!["-p".to_string()]`.

- [ ] **Step 5: Keep OpenCode template stable**

Confirm `crates/ctx-adapters/src/opencode.rs` still returns `args: vec!["run".to_string()]`.

- [ ] **Step 6: Run adapter tests and verify pass**

Run:

```bash
cargo test -p ctx-adapters
```

Expected:

```text
test result: ok
```

## Task 11.2: Adapter Execution Result And Fallback Semantics

**Files:**

- Modify: `crates/ctx-adapters/Cargo.toml`
- Modify: `crates/ctx-adapters/src/lib.rs`
- Modify: `crates/ctx-adapters/tests/adapters.rs`

- [ ] **Step 1: Add fake-binary test helpers**

Add dev dependencies to `crates/ctx-adapters/Cargo.toml`:

```toml
[dev-dependencies]
tempfile.workspace = true
```

- [ ] **Step 2: Write execution tests**

Add tests to `crates/ctx-adapters/tests/adapters.rs`:

```rust
#[cfg(unix)]
#[test]
fn execute_invocation_reports_success_with_fake_binary() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    let tmp = tempdir().expect("tempdir");
    let bin = tmp.path().join("fake-agent");
    fs::write(&bin, "#!/bin/sh\necho \"$@\" > invoked.txt\nexit 0\n").expect("write fake");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod");

    let invocation = AdapterInvocation {
        agent: Agent::Generic,
        program: bin.to_string_lossy().to_string(),
        args: vec!["run".to_string()],
        prompt: "hello ctx".to_string(),
    };

    let result = execute_invocation_with_result(&invocation).expect("execute");
    assert_eq!(result.status, AdapterRunStatus::Succeeded);
    assert_eq!(result.exit_code, Some(0));
    assert!(!result.fallback_used);
    assert!(result.command.contains("fake-agent"));
}

#[test]
fn missing_binary_returns_fallback_result() {
    let invocation = AdapterInvocation {
        agent: Agent::Claude,
        program: "ctx-definitely-missing-agent".to_string(),
        args: vec!["-p".to_string()],
        prompt: "hello ctx".to_string(),
    };

    let result = execute_invocation_with_result(&invocation).expect("fallback result");
    assert_eq!(result.status, AdapterRunStatus::Fallback);
    assert!(result.fallback_used);
    assert!(result.fallback_reason.unwrap().contains("not found"));
}
```

- [ ] **Step 3: Run tests and verify compile failure**

Run:

```bash
cargo test -p ctx-adapters
```

Expected:

```text
cannot find function `execute_invocation_with_result`
cannot find type `AdapterRunStatus`
```

- [ ] **Step 4: Implement execution result types**

Add to `crates/ctx-adapters/src/lib.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterRunStatus {
    Succeeded,
    Failed,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterExecutionResult {
    pub agent: Agent,
    pub command: String,
    pub status: AdapterRunStatus,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub fallback_used: bool,
    pub fallback_reason: Option<String>,
}
```

- [ ] **Step 5: Implement `execute_invocation_with_result`**

Add to `crates/ctx-adapters/src/lib.rs`:

```rust
pub fn execute_invocation_with_result(
    invocation: &AdapterInvocation,
) -> io::Result<AdapterExecutionResult> {
    let started = std::time::Instant::now();
    let command = invocation.command_preview();
    match Command::new(&invocation.program)
        .args(&invocation.args)
        .arg(&invocation.prompt)
        .status()
    {
        Ok(status) => Ok(AdapterExecutionResult {
            agent: invocation.agent,
            command,
            status: if status.success() {
                AdapterRunStatus::Succeeded
            } else {
                AdapterRunStatus::Failed
            },
            exit_code: status.code(),
            duration_ms: started.elapsed().as_millis() as u64,
            fallback_used: false,
            fallback_reason: None,
        }),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(AdapterExecutionResult {
            agent: invocation.agent,
            command,
            status: AdapterRunStatus::Fallback,
            exit_code: None,
            duration_ms: started.elapsed().as_millis() as u64,
            fallback_used: true,
            fallback_reason: Some(format!("program '{}' not found in PATH", invocation.program)),
        }),
        Err(err) => Err(err),
    }
}
```

- [ ] **Step 6: Keep backwards-compatible `execute_invocation`**

Keep the existing `execute_invocation()` function for any external caller that still expects `io::Result<ExitStatus>`.

- [ ] **Step 7: Run adapter tests**

Run:

```bash
cargo test -p ctx-adapters
```

Expected:

```text
test result: ok
```

## Task 11.3: Generic Adapter Contract

**Files:**

- Modify: `crates/ctx-adapters/src/generic.rs`
- Modify: `crates/ctx-adapters/src/lib.rs`
- Modify: `crates/ctx-adapters/tests/adapters.rs`

- [ ] **Step 1: Write generic adapter tests**

Add tests:

```rust
#[test]
fn generic_adapter_uses_default_agent_when_no_command_is_configured() {
    let invocation = prepare_invocation(Agent::Generic, "summarize", "compact ctx");
    assert_eq!(invocation.program, "agent");
    assert!(invocation.args.is_empty());
}

#[test]
fn generic_adapter_parses_explicit_command_template() {
    let invocation = prepare_generic_invocation(
        "custom-agent --one-shot --format text",
        "summarize",
        "compact ctx",
    );
    assert_eq!(invocation.program, "custom-agent");
    assert_eq!(invocation.args, vec!["--one-shot".to_string(), "--format".to_string(), "text".to_string()]);
    assert!(invocation.prompt.contains("summarize"));
}
```

- [ ] **Step 2: Run adapter tests and verify failure**

Run:

```bash
cargo test -p ctx-adapters generic_adapter
```

Expected:

```text
cannot find function `prepare_generic_invocation`
```

- [ ] **Step 3: Implement exported generic builder**

Add to `crates/ctx-adapters/src/lib.rs`:

```rust
pub fn prepare_generic_invocation(
    command_template: &str,
    query: &str,
    compact_context: &str,
) -> AdapterInvocation {
    generic::prepare_from_template(command_template, query, compact_context)
}
```

Add to `crates/ctx-adapters/src/generic.rs`:

```rust
pub fn prepare_from_template(command_template: &str, query: &str, compact_context: &str) -> AdapterInvocation {
    let mut parts = command_template.split_whitespace();
    let program = parts.next().unwrap_or("agent").to_string();
    let args = parts.map(ToString::to_string).collect::<Vec<_>>();
    AdapterInvocation {
        agent: Agent::Generic,
        program,
        args,
        prompt: compose_prompt(query, compact_context),
    }
}
```

- [ ] **Step 4: Make default generic path call the same parser**

Change `generic::prepare()` to call:

```rust
prepare_from_template("agent", query, compact_context)
```

- [ ] **Step 5: Run adapter tests**

Run:

```bash
cargo test -p ctx-adapters
```

Expected:

```text
test result: ok
```

## Task 11.4: Runs Table Migration And Metadata API

**Files:**

- Modify: `crates/ctx-graph/src/schema.sql`
- Modify: `crates/ctx-graph/src/lib.rs`
- Modify: `crates/ctx-graph/tests/graph_features.rs`

- [ ] **Step 1: Write run metadata tests**

Add to `crates/ctx-graph/tests/graph_features.rs`:

```rust
#[test]
fn invocation_runs_persist_full_metadata() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("graph.db");
    let store = GraphStore::open(&db).expect("open");
    store.init_schema().expect("schema");

    let run_id = store
        .record_invocation_run(&ctx_graph::RunInsert {
            command: "claude -p \"fix auth\"".to_string(),
            status: "succeeded".to_string(),
            agent: Some("claude".to_string()),
            exit_code: Some(0),
            duration_ms: Some(42),
            original_tokens: Some(1200),
            packed_tokens: Some(240),
            reduction_pct: Some(80.0),
            fallback_used: false,
            pack_path: Some(".ctx/packs/123.json".to_string()),
        })
        .expect("record run");

    assert!(run_id > 0);
    let runs = store.recent_runs(5).expect("recent runs");
    assert_eq!(runs[0].agent.as_deref(), Some("claude"));
    assert_eq!(runs[0].status, "succeeded");
    assert_eq!(runs[0].packed_tokens, Some(240));
    assert!(!runs[0].fallback_used);
}
```

- [ ] **Step 2: Write old-schema migration test**

Add:

```rust
#[test]
fn init_schema_migrates_existing_runs_table() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("graph.db");
    {
        let conn = rusqlite::Connection::open(&db).expect("open raw");
        conn.execute_batch(
            "CREATE TABLE runs (
              id INTEGER PRIMARY KEY,
              task_id INTEGER,
              command TEXT NOT NULL,
              status TEXT NOT NULL,
              created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .expect("old schema");
    }

    let store = GraphStore::open(&db).expect("open store");
    store.init_schema().expect("migrate schema");
    store
        .record_invocation_run(&ctx_graph::RunInsert {
            command: "codex exec \"review\"".to_string(),
            status: "fallback".to_string(),
            agent: Some("codex".to_string()),
            exit_code: None,
            duration_ms: Some(1),
            original_tokens: Some(100),
            packed_tokens: Some(20),
            reduction_pct: Some(80.0),
            fallback_used: true,
            pack_path: None,
        })
        .expect("record after migrate");

    let runs = store.recent_runs(1).expect("recent");
    assert_eq!(runs[0].agent.as_deref(), Some("codex"));
    assert!(runs[0].fallback_used);
}
```

- [ ] **Step 3: Add `rusqlite` dev import if needed**

`ctx-graph` already depends on `rusqlite`, so the test can use `rusqlite::Connection` directly.

- [ ] **Step 4: Run graph tests and verify failure**

Run:

```bash
cargo test -p ctx-graph invocation_runs init_schema_migrates
```

Expected:

```text
cannot find type `RunInsert`
cannot find method `record_invocation_run`
cannot find method `recent_runs`
```

- [ ] **Step 5: Extend schema**

Update `crates/ctx-graph/src/schema.sql` `runs` table to include the target metadata columns. Add indexes:

```sql
CREATE INDEX IF NOT EXISTS idx_runs_created_at ON runs(created_at);
CREATE INDEX IF NOT EXISTS idx_runs_agent_created_at ON runs(agent, created_at);
```

- [ ] **Step 6: Add schema migration helper**

In `GraphStore::init_schema()`, after `execute_batch(schema.sql)`, call a helper that checks `PRAGMA table_info(runs)` and adds missing columns.

Use this exact helper shape:

```rust
fn ensure_column(&self, table: &str, column: &str, ddl: &str) -> Result<()> {
    let mut stmt = self
        .conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .context("failed to inspect table columns")?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .context("failed to query table columns")?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if !columns.iter().any(|existing| existing == column) {
        self.conn
            .execute_batch(ddl)
            .with_context(|| format!("failed to add column {table}.{column}"))?;
    }
    Ok(())
}
```

- [ ] **Step 7: Add migration calls**

Call:

```rust
self.ensure_column("runs", "agent", "ALTER TABLE runs ADD COLUMN agent TEXT")?;
self.ensure_column("runs", "exit_code", "ALTER TABLE runs ADD COLUMN exit_code INTEGER")?;
self.ensure_column("runs", "duration_ms", "ALTER TABLE runs ADD COLUMN duration_ms INTEGER")?;
self.ensure_column("runs", "original_tokens", "ALTER TABLE runs ADD COLUMN original_tokens INTEGER")?;
self.ensure_column("runs", "packed_tokens", "ALTER TABLE runs ADD COLUMN packed_tokens INTEGER")?;
self.ensure_column("runs", "reduction_pct", "ALTER TABLE runs ADD COLUMN reduction_pct REAL")?;
self.ensure_column("runs", "fallback_used", "ALTER TABLE runs ADD COLUMN fallback_used INTEGER NOT NULL DEFAULT 0")?;
self.ensure_column("runs", "pack_path", "ALTER TABLE runs ADD COLUMN pack_path TEXT")?;
```

- [ ] **Step 8: Add run structs and APIs**

Add public structs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunInsert {
    pub command: String,
    pub status: String,
    pub agent: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub original_tokens: Option<usize>,
    pub packed_tokens: Option<usize>,
    pub reduction_pct: Option<f64>,
    pub fallback_used: bool,
    pub pack_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: i64,
    pub command: String,
    pub status: String,
    pub agent: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub original_tokens: Option<usize>,
    pub packed_tokens: Option<usize>,
    pub reduction_pct: Option<f64>,
    pub fallback_used: bool,
    pub pack_path: Option<String>,
    pub created_at: String,
}
```

Add `record_invocation_run()` and `recent_runs()` using explicit SQL column lists.

- [ ] **Step 9: Keep `record_run` backwards-compatible**

Change `record_run(command, status)` to call `record_invocation_run()` with all optional fields as `None` and `fallback_used: false`.

- [ ] **Step 10: Run graph tests**

Run:

```bash
cargo test -p ctx-graph
```

Expected:

```text
test result: ok
```

## Task 11.5: Telemetry Stats Module Split And Compatibility

**Files:**

- Create: `crates/ctx-telemetry/src/stats.rs`
- Modify: `crates/ctx-telemetry/src/lib.rs`
- Modify: `crates/ctx-telemetry/tests/stats.rs`

- [ ] **Step 1: Write backwards compatibility test**

Add to `crates/ctx-telemetry/tests/stats.rs`:

```rust
#[test]
fn reads_legacy_stats_snapshot_without_adapter_fields() {
    let tmp = tempdir().expect("tempdir");
    let stats_dir = tmp.path().join(".ctx/stats");
    std::fs::create_dir_all(&stats_dir).expect("mkdir");
    std::fs::write(
        stats_dir.join("latest.json"),
        r#"{"original_tokens":1000,"packed_tokens":250,"reduction_pct":75.0,"latency_ms":12}"#,
    )
    .expect("write legacy stats");

    let loaded = read_latest_stats(&stats_dir).expect("read legacy");
    assert_eq!(loaded.packed_tokens, 250);
    assert_eq!(loaded.agent, None);
    assert!(!loaded.fallback_used);
}
```

- [ ] **Step 2: Write invocation stats test**

Add:

```rust
#[test]
fn writes_invocation_fields_in_latest_stats() {
    let tmp = tempdir().expect("tempdir");
    let stats_dir = tmp.path().join(".ctx/stats");

    let snapshot = StatsSnapshot {
        original_tokens: 1000,
        packed_tokens: 200,
        reduction_pct: 80.0,
        latency_ms: 44,
        agent: Some("claude".to_string()),
        command: Some("claude -p \"fix\"".to_string()),
        status: Some("succeeded".to_string()),
        exit_code: Some(0),
        fallback_used: false,
        pack_path: Some(".ctx/packs/1.json".to_string()),
    };

    write_latest_stats(&stats_dir, &snapshot).expect("write");
    let body = std::fs::read_to_string(stats_dir.join("latest.json")).expect("read body");
    assert!(body.contains("claude"));
    assert!(body.contains("fallback_used"));
}
```

- [ ] **Step 3: Run telemetry tests and verify failure**

Run:

```bash
cargo test -p ctx-telemetry stats
```

Expected:

```text
missing fields in initializer or no field `agent` on type `StatsSnapshot`
```

- [ ] **Step 4: Move stats code into `stats.rs`**

Create `crates/ctx-telemetry/src/stats.rs` and move `StatsSnapshot`, `write_latest_stats`, and `read_latest_stats` into it.

- [ ] **Step 5: Extend `StatsSnapshot`**

Use the target struct from the Data Contracts section with `#[serde(default)]` fields.

- [ ] **Step 6: Update existing stats tests**

Update existing `StatsSnapshot` initializers to include:

```rust
agent: None,
command: None,
status: None,
exit_code: None,
fallback_used: false,
pack_path: None,
```

- [ ] **Step 7: Re-export stats API**

In `crates/ctx-telemetry/src/lib.rs`:

```rust
pub mod stats;
pub use stats::{StatsSnapshot, read_latest_stats, write_latest_stats};
```

- [ ] **Step 8: Run telemetry tests**

Run:

```bash
cargo test -p ctx-telemetry
```

Expected:

```text
test result: ok
```

## Task 11.6: Local Audit Module

**Files:**

- Create: `crates/ctx-telemetry/src/audit.rs`
- Modify: `crates/ctx-telemetry/src/lib.rs`
- Create: `crates/ctx-telemetry/tests/audit.rs`

- [ ] **Step 1: Write audit tests**

Create `crates/ctx-telemetry/tests/audit.rs`:

```rust
use ctx_telemetry::{AuditEvent, append_audit_event, append_audit_line};
use tempfile::tempdir;

#[test]
fn appends_human_readable_audit_line() {
    let tmp = tempdir().expect("tempdir");
    let audit_path = tmp.path().join(".ctx/audit.log");

    append_audit_line(&audit_path, "run_pack query=\"fix auth\" packed_tokens=200")
        .expect("append line");

    let body = std::fs::read_to_string(audit_path).expect("read audit");
    assert!(body.contains("run_pack"));
    assert!(body.contains("packed_tokens=200"));
}

#[test]
fn appends_structured_audit_event_as_json_line() {
    let tmp = tempdir().expect("tempdir");
    let audit_path = tmp.path().join(".ctx/audit.log");

    append_audit_event(
        &audit_path,
        &AuditEvent {
            kind: "adapter_invocation".to_string(),
            message: "ctx invoked claude".to_string(),
            agent: Some("claude".to_string()),
            command: Some("claude -p \"fix auth\"".to_string()),
            status: Some("succeeded".to_string()),
            fallback_used: false,
            pack_path: Some(".ctx/packs/1.json".to_string()),
        },
    )
    .expect("append event");

    let body = std::fs::read_to_string(audit_path).expect("read audit");
    assert!(body.contains("adapter_invocation"));
    assert!(body.contains("ctx invoked claude"));
    assert!(body.contains("\"fallback_used\":false"));
}
```

- [ ] **Step 2: Run telemetry tests and verify failure**

Run:

```bash
cargo test -p ctx-telemetry audit
```

Expected:

```text
unresolved imports `AuditEvent`, `append_audit_event`, `append_audit_line`
```

- [ ] **Step 3: Add `AuditEvent` and append helpers**

Create `crates/ctx-telemetry/src/audit.rs`:

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub kind: String,
    pub message: String,
    pub agent: Option<String>,
    pub command: Option<String>,
    pub status: Option<String>,
    pub fallback_used: bool,
    pub pack_path: Option<String>,
}

pub fn append_audit_line(audit_path: &Path, line: &str) -> Result<()> {
    if let Some(parent) = audit_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create audit parent {}", parent.display()))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(audit_path)
        .with_context(|| format!("failed to open {}", audit_path.display()))?;
    writeln!(file, "{line}").context("failed to append audit line")?;
    Ok(())
}

pub fn append_audit_event(audit_path: &Path, event: &AuditEvent) -> Result<()> {
    let line = serde_json::to_string(event).context("failed to serialize audit event")?;
    append_audit_line(audit_path, &line)
}
```

- [ ] **Step 4: Re-export audit API**

In `crates/ctx-telemetry/src/lib.rs`:

```rust
pub mod audit;
pub use audit::{AuditEvent, append_audit_event, append_audit_line};
```

- [ ] **Step 5: Run telemetry tests**

Run:

```bash
cargo test -p ctx-telemetry
```

Expected:

```text
test result: ok
```

## Task 11.7: Core Invocation Orchestrator

**Files:**

- Modify: `crates/ctx-core/Cargo.toml`
- Modify: `crates/ctx-core/src/lib.rs`
- Modify: `crates/ctx-core/tests/core_behavior.rs`

- [ ] **Step 1: Add adapter dependency to core**

Update `crates/ctx-core/Cargo.toml`:

```toml
ctx-adapters = { path = "../ctx-adapters" }
```

- [ ] **Step 2: Write core fallback test**

Add to `crates/ctx-core/tests/core_behavior.rs`:

```rust
#[test]
fn run_agent_invocation_records_fallback_metadata_when_binary_missing() {
    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");

    let report = ctx_core::run_agent_invocation(
        tmp.path(),
        ctx_adapters::Agent::Claude,
        "explain flaky test",
        Some(500),
        None,
    )
    .expect("run invocation");

    assert_eq!(report.agent, "claude");
    assert_eq!(report.status, "fallback");
    assert!(report.fallback_used);
    assert!(report.prompt_preview.unwrap().contains("[CTX COMPACT CONTEXT]"));

    let stats = std::fs::read_to_string(tmp.path().join(".ctx/stats/latest.json")).expect("stats");
    assert!(stats.contains("claude"));
    assert!(stats.contains("fallback"));

    let audit = std::fs::read_to_string(tmp.path().join(".ctx/audit.log")).expect("audit");
    assert!(audit.contains("adapter_invocation"));
}
```

- [ ] **Step 3: Run core test and verify failure**

Run:

```bash
cargo test -p ctx-core run_agent_invocation_records_fallback_metadata_when_binary_missing
```

Expected:

```text
cannot find function `run_agent_invocation`
```

- [ ] **Step 4: Add `AdapterRunReport` to core**

Add the target `AdapterRunReport` struct from the Data Contracts section to `crates/ctx-core/src/lib.rs`.

- [ ] **Step 5: Implement `run_agent_invocation`**

Add a function with this signature:

```rust
pub fn run_agent_invocation(
    repo_root: &Path,
    agent: ctx_adapters::Agent,
    query: &str,
    budget: Option<usize>,
    attach: Option<&Path>,
) -> Result<AdapterRunReport>
```

Function behavior:

```text
1. Call `run_pack(repo_root, query, budget, attach)`.
2. Build invocation with `ctx_adapters::prepare_invocation(agent, query, &packed.compact_context)`.
3. Execute with `ctx_adapters::execute_invocation_with_result(&invocation)`.
4. Map adapter result to `AdapterRunReport`.
5. Open graph store from config if graph is enabled.
6. Insert `RunInsert` with agent, command, status, exit code, duration, token counts, fallback, and pack path.
7. Write `.ctx/stats/latest.json` with invocation-aware `StatsSnapshot`.
8. Append `.ctx/audit.log` structured `AuditEvent` with kind `adapter_invocation`.
9. Return report for CLI formatting.
10. If adapter result is `Failed`, still record telemetry and return report. CLI decides whether to exit non-zero.
```

Status mapping:

```rust
fn adapter_status_label(status: ctx_adapters::AdapterRunStatus) -> &'static str {
    match status {
        ctx_adapters::AdapterRunStatus::Succeeded => "succeeded",
        ctx_adapters::AdapterRunStatus::Failed => "failed",
        ctx_adapters::AdapterRunStatus::Fallback => "fallback",
    }
}
```

Prompt preview behavior:

```rust
let prompt_preview = if execution.fallback_used {
    Some(invocation.prompt.clone())
} else {
    None
};
```

- [ ] **Step 6: Replace core private audit helper**

Keep `append_audit_entry(repo_root, line)` if other tests rely on plain `run_pack` lines, but implement it by calling `ctx_telemetry::append_audit_line(&repo_root.join(".ctx/audit.log"), line)`.

- [ ] **Step 7: Run core tests**

Run:

```bash
cargo test -p ctx-core
```

Expected:

```text
test result: ok
```

## Task 11.8: CLI Wrappers Use Core Invocation Path

**Files:**

- Modify: `crates/ctx-cli/src/main.rs`
- Modify: `crates/ctx-cli/tests/cli_behavior.rs`

- [ ] **Step 1: Write CLI Claude fallback test**

Add to `crates/ctx-cli/tests/cli_behavior.rs`:

```rust
#[test]
fn claude_wrapper_uses_real_adapter_path_and_fallback_output() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["claude", "explain flaky test"])
        .current_dir(tmp.path())
        .env("PATH", "")
        .assert()
        .success()
        .stdout(predicate::str::contains("adapter=claude"))
        .stdout(predicate::str::contains("command=claude -p"))
        .stdout(predicate::str::contains("[CTX COMPACT CONTEXT]"));

    assert!(tmp.path().join(".ctx/stats/latest.json").exists());
}
```

- [ ] **Step 2: Write CLI JSON test**

Add:

```rust
#[test]
fn adapter_wrapper_json_outputs_run_report() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["codex", "review risky diff", "--json"])
        .current_dir(tmp.path())
        .env("PATH", "")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"agent\": \"codex\""))
        .stdout(predicate::str::contains("\"status\": \"fallback\""))
        .stdout(predicate::str::contains("\"fallback_used\": true"));
}
```

- [ ] **Step 3: Run CLI tests and verify failure**

Run:

```bash
cargo test -p ctx-cli claude_wrapper_uses_real_adapter_path_and_fallback_output adapter_wrapper_json_outputs_run_report
```

Expected:

```text
claude wrapper test fails because current command only prints compact context
json test fails because wrapper ignores --json
```

- [ ] **Step 4: Replace CLI adapter wrapper implementation**

Change imports in `crates/ctx-cli/src/main.rs`:

```rust
use ctx_adapters::Agent;
use ctx_core::run_agent_invocation;
```

Remove direct `execute_invocation` and `prepare_invocation` imports.

- [ ] **Step 5: Route Claude through wrapper**

Change `Commands::Claude` arm to call the same wrapper as Codex and OpenCode:

```rust
Commands::Claude { query } => {
    run_adapter_wrapper(
        &repo_root,
        Agent::Claude,
        &query,
        cli.budget,
        cli.attach.as_deref(),
        cli.json,
    )?;
}
```

- [ ] **Step 6: Update wrapper signature**

Change `run_adapter_wrapper` to:

```rust
fn run_adapter_wrapper(
    repo_root: &std::path::Path,
    agent: Agent,
    query: &str,
    budget: Option<usize>,
    attach: Option<&std::path::Path>,
    json: bool,
) -> Result<()>
```

- [ ] **Step 7: Format wrapper output**

Use this behavior:

```text
If json is true:
  print serde_json::to_string_pretty(&report)
  return Err only when report.status == "failed"
If report.status == "fallback":
  print warning to stderr
  print adapter, command, and prompt preview to stdout
  return Ok
If report.status == "failed":
  return Err with non-zero adapter status
If report.status == "succeeded":
  return Ok
```

- [ ] **Step 8: Run CLI tests**

Run:

```bash
cargo test -p ctx-cli claude_wrapper_uses_real_adapter_path_and_fallback_output adapter_wrapper_json_outputs_run_report
```

Expected:

```text
test result: ok
```

## Task 11.9: CLI Fake Binary Success Tests

**Files:**

- Modify: `crates/ctx-cli/tests/cli_behavior.rs`

- [ ] **Step 1: Add fake agent helper**

Add helper near existing test helpers:

```rust
#[cfg(unix)]
fn write_fake_agent_bin(dir: &std::path::Path, name: &str) {
    let bin = dir.join(name);
    let script = "#!/bin/sh\necho \"$0 $@\" >> ../invocations.log\nexit 0\n";
    fs::write(&bin, script).expect("write fake agent");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod fake agent");
}
```

- [ ] **Step 2: Add success-path test for Claude**

Add:

```rust
#[cfg(unix)]
#[test]
fn claude_wrapper_invokes_fake_claude_binary_and_records_success() {
    let tmp = tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("mkdir bin");
    write_fake_agent_bin(&bin_dir, "claude");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["claude", "explain flaky test"])
        .current_dir(tmp.path())
        .env("PATH", bin_dir.to_string_lossy().to_string())
        .assert()
        .success();

    let stats = fs::read_to_string(tmp.path().join(".ctx/stats/latest.json")).expect("stats");
    assert!(stats.contains("claude"));
    assert!(stats.contains("succeeded"));

    let audit = fs::read_to_string(tmp.path().join(".ctx/audit.log")).expect("audit");
    assert!(audit.contains("adapter_invocation"));
    assert!(audit.contains("claude"));
}
```

- [ ] **Step 3: Add success-path test for Codex**

Add:

```rust
#[cfg(unix)]
#[test]
fn codex_wrapper_invokes_fake_codex_binary_and_records_success() {
    let tmp = tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("mkdir bin");
    write_fake_agent_bin(&bin_dir, "codex");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["codex", "review diff"])
        .current_dir(tmp.path())
        .env("PATH", bin_dir.to_string_lossy().to_string())
        .assert()
        .success();

    let stats = fs::read_to_string(tmp.path().join(".ctx/stats/latest.json")).expect("stats");
    assert!(stats.contains("codex"));
    assert!(stats.contains("succeeded"));
}
```

- [ ] **Step 4: Add success-path test for OpenCode**

Add:

```rust
#[cfg(unix)]
#[test]
fn opencode_wrapper_invokes_fake_opencode_binary_and_records_success() {
    let tmp = tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("mkdir bin");
    write_fake_agent_bin(&bin_dir, "opencode");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["opencode", "run", "explain diff"])
        .current_dir(tmp.path())
        .env("PATH", bin_dir.to_string_lossy().to_string())
        .assert()
        .success();

    let stats = fs::read_to_string(tmp.path().join(".ctx/stats/latest.json")).expect("stats");
    assert!(stats.contains("opencode"));
    assert!(stats.contains("succeeded"));
}
```

- [ ] **Step 5: Run CLI adapter tests**

Run:

```bash
cargo test -p ctx-cli wrapper
```

Expected:

```text
test result: ok
```

## Task 11.10: Stats Command Shows Invocation Metrics

**Files:**

- Modify: `crates/ctx-cli/src/main.rs`
- Modify: `crates/ctx-cli/tests/cli_behavior.rs`

- [ ] **Step 1: Add stats-after-adapter test**

Add to `crates/ctx-cli/tests/cli_behavior.rs`:

```rust
#[test]
fn stats_after_adapter_run_includes_agent_latency_and_fallback() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .args(["claude", "explain flaky test"])
        .current_dir(tmp.path())
        .env("PATH", "")
        .assert()
        .success();

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("stats")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("original_tokens"))
        .stdout(predicate::str::contains("packed_tokens"))
        .stdout(predicate::str::contains("latency_ms"))
        .stdout(predicate::str::contains("agent"))
        .stdout(predicate::str::contains("fallback_used"));
}
```

- [ ] **Step 2: Run test and verify pass after previous tasks**

Run:

```bash
cargo test -p ctx-cli stats_after_adapter_run_includes_agent_latency_and_fallback
```

Expected:

```text
test result: ok
```

- [ ] **Step 3: Keep `ctx stats` file-based for now**

Do not introduce a new graph aggregation command in Task 11. `ctx stats` must read latest local stats and include invocation metadata. Historical aggregation can be a later enhancement in Task 16 benchmark/reporting.

## Task 11.11: JSON Output Contract For Wrappers

**Files:**

- Modify: `crates/ctx-cli/tests/cli_behavior.rs`
- Modify: `crates/ctx-core/src/lib.rs`

- [ ] **Step 1: Add JSON shape test**

Add:

```rust
#[test]
fn adapter_json_contract_contains_required_fields() {
    let tmp = tempdir().expect("tempdir");

    Command::cargo_bin("ctx")
        .expect("bin")
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = Command::cargo_bin("ctx")
        .expect("bin")
        .args(["opencode", "run", "explain this diff", "--json"])
        .current_dir(tmp.path())
        .env("PATH", "")
        .output()
        .expect("run ctx");

    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["agent"], "opencode");
    assert!(value["command"].as_str().unwrap().contains("opencode run"));
    assert_eq!(value["status"], "fallback");
    assert_eq!(value["fallback_used"], true);
    assert!(value["original_tokens"].as_u64().unwrap() >= value["packed_tokens"].as_u64().unwrap());
    assert!(value["reduction_pct"].is_number());
}
```

- [ ] **Step 2: Run test and fix serialization mismatches**

Run:

```bash
cargo test -p ctx-cli adapter_json_contract_contains_required_fields
```

Expected:

```text
test result: ok
```

If field names differ, update `AdapterRunReport` instead of weakening the test. This JSON contract is part of Task 11 acceptance.

## Task 11.12: README Update

**Files:**

- Modify: `README.md`

- [ ] **Step 1: Update Current Status**

Change adapter status from:

```text
adapter runtime reali per `codex` e `opencode` (invocazione CLI + fallback)
```

To:

```text
adapter runtime reali per `codex`, `claude` e `opencode` con invocation telemetry locale e fallback prompt-safe
```

- [ ] **Step 2: Update command table**

Update command rows:

```markdown
| `ctx codex "..."` | Builds a CTX pack, runs `codex exec` with compact context, and records local invocation stats | `ctx codex "review risky diff"` | Runs Codex if available; otherwise prints fallback prompt and records `fallback_used=true` |
| `ctx claude "..."` | Builds a CTX pack, runs `claude -p` with compact context, and records local invocation stats | `ctx claude "explain flaky test"` | Runs Claude Code print mode if available; otherwise prints fallback prompt and records `fallback_used=true` |
| `ctx opencode run "..."` | Builds a CTX pack, runs `opencode run` with compact context, and records local invocation stats | `ctx opencode run "explain this diff"` | Runs OpenCode if available; otherwise prints fallback prompt and records `fallback_used=true` |
| `ctx stats` | Reads latest local token reduction, latency, adapter status, and fallback metadata | after `ctx claude "..."`, run `ctx stats` | JSON includes `original_tokens`, `packed_tokens`, `reduction_pct`, `latency_ms`, `agent`, `status`, `fallback_used` |
```

- [ ] **Step 3: Update module-level tests**

Change telemetry description to mention audit and invocation stats:

```markdown
| Telemetry | `cargo test -p ctx-telemetry` | stats, invocation metadata compatibility, audit log, benchmark summary/report |
```

- [ ] **Step 4: Mark Task 11 done**

Change Task 11 row to:

```markdown
| 11 | Invocation + telemetry | Done | codex/claude/opencode real invocation, fallback behavior, local runs metadata, stats and audit complete |
```

- [ ] **Step 5: Remove completed queue items**

Remove current ordered queue items that say:

```text
Chiudere `Task 11`...
Implementare adapter CLI `Claude` reale...
```

Renumber remaining queue from the alias workflow / hook mode items.

## Task 11.13: Full Verification

**Files:**

- No source edits unless tests reveal a real issue.

- [ ] **Step 1: Run focused adapter suite**

Run:

```bash
cargo test -p ctx-adapters
```

Expected:

```text
test result: ok
```

- [ ] **Step 2: Run focused graph suite**

Run:

```bash
cargo test -p ctx-graph
```

Expected:

```text
test result: ok
```

- [ ] **Step 3: Run focused telemetry suite**

Run:

```bash
cargo test -p ctx-telemetry
```

Expected:

```text
test result: ok
```

- [ ] **Step 4: Run focused core suite**

Run:

```bash
cargo test -p ctx-core
```

Expected:

```text
test result: ok
```

- [ ] **Step 5: Run focused CLI suite**

Run:

```bash
cargo test -p ctx-cli
```

Expected:

```text
test result: ok
```

- [ ] **Step 6: Run full workspace suite**

Run:

```bash
cargo test --workspace
```

Expected:

```text
test result: ok
```

- [ ] **Step 7: Manual smoke fallback**

Run with missing binaries:

```bash
PATH="" cargo run -p ctx-cli --bin ctx -- claude "explain flaky test"
```

Expected:

```text
adapter=claude
command=claude -p "..."
[CTX COMPACT CONTEXT]
```

Then run:

```bash
cargo run -p ctx-cli --bin ctx -- stats
```

Expected JSON contains:

```json
{
  "agent": "claude",
  "status": "fallback",
  "fallback_used": true
}
```

- [ ] **Step 8: Manual smoke JSON**

Run:

```bash
PATH="" cargo run -p ctx-cli --bin ctx -- opencode run "explain this diff" --json
```

Expected JSON contains these fields:

```json
{
  "agent": "opencode",
  "status": "fallback",
  "fallback_used": true,
  "original_tokens": 100,
  "packed_tokens": 50,
  "reduction_pct": 50.0
}
```

The numeric token values above are illustrative. The required behavior is that all fields exist, `original_tokens >= packed_tokens`, and `reduction_pct` is numeric.

## Acceptance Criteria

Task 11 is done only if all criteria are true:

- `ctx codex "..."` invokes `codex exec` when `codex` exists in `PATH`.
- `ctx claude "..."` invokes `claude -p` when `claude` exists in `PATH`.
- `ctx opencode run "..."` invokes `opencode run` when `opencode` exists in `PATH`.
- Missing binaries produce fallback output with `adapter=...`, `command=...`, and the full prepared CTX prompt.
- Failed binaries produce a non-zero CTX CLI error after local metadata is recorded.
- `.ctx/graph.db` `runs` table stores `agent`, `command`, `status`, `exit_code`, `duration_ms`, `original_tokens`, `packed_tokens`, `reduction_pct`, `fallback_used`, and `pack_path`.
- Existing graph databases with the old `runs` table migrate without data loss.
- `.ctx/stats/latest.json` includes token reduction, local latency, adapter status, and fallback metadata.
- `.ctx/audit.log` includes `run_pack` and `adapter_invocation` entries.
- `--json` wrapper output is stable and includes all required `AdapterRunReport` fields.
- The implementation does not add default flags that disable native Claude Code skills/plugins/hooks/MCP or default Codex/OpenCode behavior.
- `README.md` marks Task 11 as `Done` and documents how to test every wrapper and stats behavior.
- `cargo test --workspace` passes.

## Commit Plan

Commit after coherent checkpoints:

```bash
git add crates/ctx-adapters crates/ctx-graph
git commit -m "feat: harden adapter invocation contracts"
```

```bash
git add crates/ctx-telemetry crates/ctx-core
git commit -m "feat: record local invocation telemetry"
```

```bash
git add crates/ctx-cli README.md
git commit -m "feat: complete agent wrapper telemetry"
```

If the user wants a single commit instead, stage all Task 11 files together after full verification.

## Post-Task Notes

After Task 11 is complete, the next natural tasks are:

- Task 12: expose `ctx ask ...` and `ctx wrap <agent> --prompt ...` aliases using the now-stable adapter contract.
- Task 12: implement hook mode as a real pre-prompt command path.
- Task 13: reuse adapter metadata to improve MCP integration presets.
- Task 14: harden privacy controls around local-only telemetry and audit visibility.
- Task 16: aggregate historical `runs` data into publishable benchmark reports.
