# OpenCode Demo Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a realistic OpenCode-first demo project, plus demo scripts, fixture data, benchmark inputs, and acceptance tests that prove CTX works end-to-end on a credible repository.

**Architecture:** The demo is a versioned fixture repository inside this workspace. It is intentionally designed to exercise graph indexing, graph memory bootstrap/search, retrieval, prune, pack, MCP, security, and benchmark flows from the same project. The primary user story remains OpenCode-first: bootstrap with `ctx opencode install`, then use `/ctx-*` inside OpenCode while automated CLI/MCP scripts validate the same underlying runtime behavior.

**Status:** Complete on `2026-04-26`. The fixture repo, smoke scripts, MCP validation, walkthrough docs, and versioned benchmark reports are now all present in the workspace and covered by automated tests.

**Tech Stack:** Rust workspace, OpenCode repo-local bootstrap, TypeScript fixture app, Vitest/Jest-style failure fixtures, markdown memory files, shell smoke scripts, Markdown/JSON benchmark reports, assert_cmd integration tests.

---

## File Map

**Create:**
- `demo/fixtures/opencode-auth-lab/README.md`
- `demo/fixtures/opencode-auth-lab/package.json`
- `demo/fixtures/opencode-auth-lab/tsconfig.json`
- `demo/fixtures/opencode-auth-lab/src/auth/tokens.ts`
- `demo/fixtures/opencode-auth-lab/src/auth/session.ts`
- `demo/fixtures/opencode-auth-lab/src/auth/audit.ts`
- `demo/fixtures/opencode-auth-lab/src/http/refresh-route.ts`
- `demo/fixtures/opencode-auth-lab/src/lib/retry.ts`
- `demo/fixtures/opencode-auth-lab/tests/auth/refresh-route.test.ts`
- `demo/fixtures/opencode-auth-lab/tests/auth/session.test.ts`
- `demo/fixtures/opencode-auth-lab/AGENTS.md`
- `demo/fixtures/opencode-auth-lab/CLAUDE.md`
- `demo/fixtures/opencode-auth-lab/CODEX.md`
- `demo/fixtures/opencode-auth-lab/.github/copilot-instructions.md`
- `demo/fixtures/opencode-auth-lab/logs/vitest-refresh-failure.log`
- `demo/fixtures/opencode-auth-lab/logs/noisy-ci.log`
- `demo/fixtures/opencode-auth-lab/diff/refresh-route.patch`
- `demo/fixtures/opencode-auth-lab/checklists/graph-memory-quality.md`
- `demo/fixtures/opencode-auth-lab/answers/markdown-answer.txt`
- `demo/fixtures/opencode-auth-lab/answers/graph-answer.txt`
- `demo/fixtures/opencode-auth-lab/benchmarks/memory-suite.toml`
- `demo/fixtures/opencode-auth-lab/expected/doctor.txt`
- `demo/fixtures/opencode-auth-lab/expected/memory-search-auth.txt`
- `demo/fixtures/opencode-auth-lab/expected/prune-logs.txt`
- `demo/fixtures/opencode-auth-lab/expected/pack-fragments.txt`
- `scripts/demo/opencode-auth-lab-smoke.sh`
- `scripts/demo/opencode-auth-lab-mcp-smoke.sh`
- `scripts/demo/opencode-auth-lab-benchmark.sh`
- `docs/demo-script.md`
- `docs/demo-walkthrough.md`
- `crates/ctx-cli/tests/demo_assets.rs`
- `crates/ctx-mcp/tests/demo_fixture_mcp.rs`

**Modify:**
- `README.md`
- `guide.md`
- `docs/install.md`
- `docs/superpowers/plans/2026-04-25-final-release-roadmap.md`
- `scripts/release/build.sh`
- `scripts/release/opencode-smoke.sh`
- `crates/ctx-cli/tests/cli_behavior.rs`

**Test:**
- `crates/ctx-cli/tests/demo_assets.rs`
- `crates/ctx-mcp/tests/demo_fixture_mcp.rs`
- `crates/ctx-cli/tests/cli_behavior.rs`

---

### Task 1: Define The Demo Fixture Repository

**Files:**
- Create: `demo/fixtures/opencode-auth-lab/README.md`
- Create: `demo/fixtures/opencode-auth-lab/package.json`
- Create: `demo/fixtures/opencode-auth-lab/tsconfig.json`

- [ ] **Step 1: Write the failing docs test that requires a demo fixture path in repo docs**

```rust
#[test]
fn docs_reference_the_opencode_auth_lab_demo_fixture() {
    let root = repo_root();
    let readme = std::fs::read_to_string(root.join("README.md")).expect("readme");
    let guide = std::fs::read_to_string(root.join("guide.md")).expect("guide");

    assert!(readme.contains("demo/fixtures/opencode-auth-lab"));
    assert!(guide.contains("opencode-auth-lab"));
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p ctx-cli docs_reference_the_opencode_auth_lab_demo_fixture -- --exact`
Expected: FAIL because the demo fixture is not referenced yet.

- [ ] **Step 3: Create the fixture repo metadata**

```json
{
  "name": "opencode-auth-lab",
  "private": true,
  "type": "module",
  "scripts": {
    "test": "vitest run",
    "test:auth": "vitest run tests/auth"
  },
  "devDependencies": {
    "typescript": "^5.9.0",
    "vitest": "^3.2.0"
  }
}
```

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "dist"
  },
  "include": ["src", "tests"]
}
```

- [ ] **Step 4: Create the fixture README explaining the intended bug story**

```md
# OpenCode Auth Lab

This fixture repo exists to validate CTX inside OpenCode.

The intended debugging story is:
- refresh-token tests fail noisily
- project habits start in AGENTS-style markdown
- CTX imports those rules into graph memory
- OpenCode uses `/ctx-*` to find the right rules, files, and symbols
- CTX benchmarks graph memory against markdown memory
```

- [ ] **Step 5: Run the docs test to verify it still fails for missing references until later tasks wire docs**

Run: `cargo test -p ctx-cli docs_reference_the_opencode_auth_lab_demo_fixture -- --exact`
Expected: still FAIL until README/guide are updated in a later task.

- [ ] **Step 6: Commit**

```bash
git add demo/fixtures/opencode-auth-lab/README.md demo/fixtures/opencode-auth-lab/package.json demo/fixtures/opencode-auth-lab/tsconfig.json
# git commit -m "feat: scaffold opencode demo fixture metadata"
```

### Task 2: Build The Source Graph Surface For The Demo Repo

**Files:**
- Create: `demo/fixtures/opencode-auth-lab/src/auth/tokens.ts`
- Create: `demo/fixtures/opencode-auth-lab/src/auth/session.ts`
- Create: `demo/fixtures/opencode-auth-lab/src/auth/audit.ts`
- Create: `demo/fixtures/opencode-auth-lab/src/http/refresh-route.ts`
- Create: `demo/fixtures/opencode-auth-lab/src/lib/retry.ts`
- Create: `demo/fixtures/opencode-auth-lab/tests/auth/refresh-route.test.ts`
- Create: `demo/fixtures/opencode-auth-lab/tests/auth/session.test.ts`

- [ ] **Step 1: Write the failing fixture-asset test for the expected source files**

```rust
#[test]
fn demo_fixture_contains_graph_relevant_source_and_test_files() {
    let root = repo_root().join("demo/fixtures/opencode-auth-lab");
    for path in [
        "src/auth/tokens.ts",
        "src/auth/session.ts",
        "src/http/refresh-route.ts",
        "src/lib/retry.ts",
        "tests/auth/refresh-route.test.ts",
    ] {
        assert!(root.join(path).exists(), "missing {path}");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ctx-cli demo_fixture_contains_graph_relevant_source_and_test_files -- --exact`
Expected: FAIL because the fixture source tree is missing.

- [ ] **Step 3: Create source files with intentional graph relationships**

```ts
export function issueRefreshToken(userId: string): string {
  return `refresh:${userId}`;
}

export function rotateRefreshToken(userId: string): string {
  return issueRefreshToken(userId);
}
```

```ts
import { rotateRefreshToken } from "./tokens";
import { appendAuditEvent } from "./audit";

export function refreshSession(userId: string): string {
  const token = rotateRefreshToken(userId);
  appendAuditEvent("refresh", userId);
  return token;
}
```

```ts
export function appendAuditEvent(kind: string, userId: string): string {
  return `${kind}:${userId}`;
}
```

```ts
import { refreshSession } from "../auth/session";
import { retry } from "../lib/retry";

export async function handleRefreshRoute(userId: string): Promise<string> {
  return retry(() => Promise.resolve(refreshSession(userId)));
}
```

```ts
export async function retry<T>(fn: () => Promise<T>): Promise<T> {
  return fn();
}
```

- [ ] **Step 4: Create tests with one intentionally noisy failing story**

```ts
import { describe, expect, it } from "vitest";
import { handleRefreshRoute } from "../../src/http/refresh-route";

describe("refresh route", () => {
  it("rotates refresh tokens", async () => {
    const token = await handleRefreshRoute("user-1");
    expect(token).toContain("refresh:user-1");
  });
});
```

- [ ] **Step 5: Run the fixture-asset test to verify it passes**

Run: `cargo test -p ctx-cli demo_fixture_contains_graph_relevant_source_and_test_files -- --exact`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add demo/fixtures/opencode-auth-lab/src demo/fixtures/opencode-auth-lab/tests crates/ctx-cli/tests/demo_assets.rs
# git commit -m "feat: add graph-oriented source fixture for demo repo"
```

### Task 3: Build The Graph Memory Seed Files And Expected Query Story

**Files:**
- Create: `demo/fixtures/opencode-auth-lab/AGENTS.md`
- Create: `demo/fixtures/opencode-auth-lab/CLAUDE.md`
- Create: `demo/fixtures/opencode-auth-lab/CODEX.md`
- Create: `demo/fixtures/opencode-auth-lab/.github/copilot-instructions.md`
- Create: `demo/fixtures/opencode-auth-lab/expected/memory-search-auth.txt`

- [ ] **Step 1: Write the failing test that requires all memory seed files**

```rust
#[test]
fn demo_fixture_contains_agents_style_memory_seed_files() {
    let root = repo_root().join("demo/fixtures/opencode-auth-lab");
    for path in [
        "AGENTS.md",
        "CLAUDE.md",
        "CODEX.md",
        ".github/copilot-instructions.md",
    ] {
        assert!(root.join(path).exists(), "missing {path}");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ctx-cli demo_fixture_contains_agents_style_memory_seed_files -- --exact`
Expected: FAIL.

- [ ] **Step 3: Create markdown files with overlapping but distinct rules**

```md
# AGENTS
- Run targeted auth tests before completion.
- Fix root cause instead of bypassing refresh-token failures.
- Update docs when command behavior changes.
```

```md
# CLAUDE
- Prefer graph memory over re-reading full instruction files.
- Use compact context before scanning broad logs.
```

```md
# CODEX
- Preserve local-only defaults.
- Keep OpenCode as the primary operator experience.
```

```md
# Copilot Instructions
- Prefer auth fixtures when debugging token rotation failures.
- Keep audit behavior visible in explanations.
```

- [ ] **Step 4: Create expected topic-search fragments**

```text
project rules for auth search should include:
- targeted auth tests
- root cause
- auth fixtures
```

- [ ] **Step 5: Run the fixture seed-file test to verify it passes**

Run: `cargo test -p ctx-cli demo_fixture_contains_agents_style_memory_seed_files -- --exact`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add demo/fixtures/opencode-auth-lab/AGENTS.md demo/fixtures/opencode-auth-lab/CLAUDE.md demo/fixtures/opencode-auth-lab/CODEX.md demo/fixtures/opencode-auth-lab/.github/copilot-instructions.md demo/fixtures/opencode-auth-lab/expected/memory-search-auth.txt
# git commit -m "feat: add graph-memory markdown seeds for demo repo"
```

### Task 4: Add Noisy Logs, Diff Fixtures, And Benchmark Inputs

**Files:**
- Create: `demo/fixtures/opencode-auth-lab/logs/vitest-refresh-failure.log`
- Create: `demo/fixtures/opencode-auth-lab/logs/noisy-ci.log`
- Create: `demo/fixtures/opencode-auth-lab/diff/refresh-route.patch`
- Create: `demo/fixtures/opencode-auth-lab/checklists/graph-memory-quality.md`
- Create: `demo/fixtures/opencode-auth-lab/answers/markdown-answer.txt`
- Create: `demo/fixtures/opencode-auth-lab/answers/graph-answer.txt`
- Create: `demo/fixtures/opencode-auth-lab/benchmarks/memory-suite.toml`
- Create: `demo/fixtures/opencode-auth-lab/expected/prune-logs.txt`
- Create: `demo/fixtures/opencode-auth-lab/expected/pack-fragments.txt`

- [ ] **Step 1: Write the failing benchmark-fixture test**

```rust
#[test]
fn demo_fixture_contains_logs_diff_and_benchmark_inputs() {
    let root = repo_root().join("demo/fixtures/opencode-auth-lab");
    for path in [
        "logs/vitest-refresh-failure.log",
        "logs/noisy-ci.log",
        "diff/refresh-route.patch",
        "benchmarks/memory-suite.toml",
        "checklists/graph-memory-quality.md",
        "answers/markdown-answer.txt",
        "answers/graph-answer.txt",
    ] {
        assert!(root.join(path).exists(), "missing {path}");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ctx-cli demo_fixture_contains_logs_diff_and_benchmark_inputs -- --exact`
Expected: FAIL.

- [ ] **Step 3: Create noisy failure logs with clear root-cause lines**

```text
 RUN  v3.2.0 /demo/fixtures/opencode-auth-lab
 ✓ auth/session.test.ts (4)
 × auth/refresh-route.test.ts > refresh route > rotates refresh tokens
   AssertionError: expected refresh:user-1 to contain rotated:user-1
   at tests/auth/refresh-route.test.ts:12:19
```

- [ ] **Step 4: Create diff and benchmark artifacts**

```toml
title = "CTX Demo Memory Benchmark"

[[cases]]
name = "auth_rules"
query = "run auth tests and fix root cause"
markdown = "AGENTS.md"
limit = 20
checklist = "checklists/graph-memory-quality.md"
markdown_answer = "answers/markdown-answer.txt"
graph_answer = "answers/graph-answer.txt"
```

- [ ] **Step 5: Run the benchmark-fixture test to verify it passes**

Run: `cargo test -p ctx-cli demo_fixture_contains_logs_diff_and_benchmark_inputs -- --exact`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add demo/fixtures/opencode-auth-lab/logs demo/fixtures/opencode-auth-lab/diff demo/fixtures/opencode-auth-lab/checklists demo/fixtures/opencode-auth-lab/answers demo/fixtures/opencode-auth-lab/benchmarks demo/fixtures/opencode-auth-lab/expected
# git commit -m "feat: add log diff and benchmark fixtures for demo repo"
```

### Task 5: Add Demo Smoke Scripts For OpenCode, CLI, And MCP

**Files:**
- Create: `scripts/demo/opencode-auth-lab-smoke.sh`
- Create: `scripts/demo/opencode-auth-lab-mcp-smoke.sh`
- Create: `scripts/demo/opencode-auth-lab-benchmark.sh`
- Modify: `scripts/release/build.sh`
- Modify: `scripts/release/opencode-smoke.sh`

- [ ] **Step 1: Write the failing test that expects demo scripts to exist and mention the fixture repo**

```rust
#[test]
fn demo_scripts_exist_and_target_the_opencode_auth_lab_fixture() {
    let root = repo_root();
    for path in [
        "scripts/demo/opencode-auth-lab-smoke.sh",
        "scripts/demo/opencode-auth-lab-mcp-smoke.sh",
        "scripts/demo/opencode-auth-lab-benchmark.sh",
    ] {
        let body = std::fs::read_to_string(root.join(path)).expect("script exists");
        assert!(body.contains("demo/fixtures/opencode-auth-lab"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ctx-cli demo_scripts_exist_and_target_the_opencode_auth_lab_fixture -- --exact`
Expected: FAIL.

- [ ] **Step 3: Create the OpenCode-first smoke script**

```bash
#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
FIXTURE="$ROOT_DIR/demo/fixtures/opencode-auth-lab"
CTX_BIN="${1:-$ROOT_DIR/target/debug/ctx}"

"$CTX_BIN" --repo-root "$FIXTURE" init
"$CTX_BIN" --repo-root "$FIXTURE" index
"$CTX_BIN" --repo-root "$FIXTURE" opencode install
"$CTX_BIN" --repo-root "$FIXTURE" memory bootstrap
"$CTX_BIN" --repo-root "$FIXTURE" memory search "auth root cause" --scope project --limit 10
cat "$FIXTURE/logs/vitest-refresh-failure.log" | "$CTX_BIN" --repo-root "$FIXTURE" prune logs
"$CTX_BIN" --repo-root "$FIXTURE" pack "fix refresh token rotation" --attach "$FIXTURE/logs/vitest-refresh-failure.log" --json
```

- [ ] **Step 4: Create the MCP smoke and benchmark scripts**

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"memory_bootstrap_markdown","arguments":{}}}\n' | "$CTX_BIN" --repo-root "$FIXTURE" mcp stdio
```

```bash
"$CTX_BIN" --repo-root "$FIXTURE" benchmark memory-suite \
  --spec "$FIXTURE/benchmarks/memory-suite.toml" \
  --report-out "$FIXTURE/benchmarks/report.md" \
  --json-out "$FIXTURE/benchmarks/report.json"
```

- [ ] **Step 5: Run the demo script existence test to verify it passes**

Run: `cargo test -p ctx-cli demo_scripts_exist_and_target_the_opencode_auth_lab_fixture -- --exact`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add scripts/demo scripts/release/build.sh scripts/release/opencode-smoke.sh crates/ctx-cli/tests/demo_assets.rs
# git commit -m "feat: add demo smoke and benchmark scripts"
```

### Task 6: Add Automated Acceptance Tests Around The Demo Fixture

**Files:**
- Create: `crates/ctx-cli/tests/demo_assets.rs`
- Create: `crates/ctx-mcp/tests/demo_fixture_mcp.rs`
- Modify: `crates/ctx-cli/tests/cli_behavior.rs`

- [ ] **Step 1: Write the failing end-to-end demo CLI test**

```rust
#[test]
fn demo_fixture_cli_smoke_runs_successfully() {
    let root = repo_root();
    let script = root.join("scripts/demo/opencode-auth-lab-smoke.sh");
    let ctx_bin = assert_cmd::cargo::cargo_bin("ctx");

    let output = std::process::Command::new(script)
        .arg(ctx_bin)
        .current_dir(&root)
        .output()
        .expect("run demo smoke");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ctx-cli demo_fixture_cli_smoke_runs_successfully -- --exact`
Expected: FAIL until scripts and fixture behavior are complete.

- [ ] **Step 3: Add MCP acceptance for graph-memory bootstrap/search through stdio**

```rust
#[test]
fn demo_fixture_mcp_smoke_runs_successfully() {
    let root = repo_root();
    let script = root.join("scripts/demo/opencode-auth-lab-mcp-smoke.sh");
    let ctx_bin = assert_cmd::cargo::cargo_bin("ctx");

    let output = std::process::Command::new(script)
        .arg(ctx_bin)
        .current_dir(&root)
        .output()
        .expect("run demo mcp smoke");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
```

- [ ] **Step 4: Add benchmark acceptance for report generation**

```rust
#[test]
fn demo_fixture_benchmark_script_writes_reports() {
    let root = repo_root();
    let script = root.join("scripts/demo/opencode-auth-lab-benchmark.sh");
    let ctx_bin = assert_cmd::cargo::cargo_bin("ctx");

    let output = std::process::Command::new(script)
        .arg(ctx_bin)
        .current_dir(&root)
        .output()
        .expect("run benchmark smoke");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(root.join("demo/fixtures/opencode-auth-lab/benchmarks/report.md").exists());
    assert!(root.join("demo/fixtures/opencode-auth-lab/benchmarks/report.json").exists());
}
```

- [ ] **Step 5: Run the new tests to verify they pass**

Run: `cargo test -p ctx-cli demo_fixture_ -- --nocapture`
Run: `cargo test -p ctx-mcp demo_fixture_ -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/ctx-cli/tests/demo_assets.rs crates/ctx-mcp/tests/demo_fixture_mcp.rs crates/ctx-cli/tests/cli_behavior.rs
# git commit -m "test: add end-to-end demo acceptance coverage"
```

### Task 7: Document The Demo Walkthrough And Community Script

**Files:**
- Create: `docs/demo-script.md`
- Create: `docs/demo-walkthrough.md`
- Modify: `README.md`
- Modify: `guide.md`
- Modify: `docs/install.md`

- [ ] **Step 1: Write the failing docs test that requires the new demo docs**

```rust
#[test]
fn docs_link_the_demo_walkthrough_and_script() {
    let root = repo_root();
    let readme = std::fs::read_to_string(root.join("README.md")).expect("readme");
    let guide = std::fs::read_to_string(root.join("guide.md")).expect("guide");

    assert!(readme.contains("docs/demo-walkthrough.md"));
    assert!(guide.contains("docs/demo-script.md"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ctx-cli docs_link_the_demo_walkthrough_and_script -- --exact`
Expected: FAIL.

- [ ] **Step 3: Write the demo walkthrough docs**

```md
# CTX Demo Walkthrough

1. Open the demo fixture repo.
2. Run `ctx init`, `ctx index`, and `ctx opencode install`.
3. Open OpenCode.
4. Run `/ctx-memory-bootstrap`.
5. Run `/ctx-memory-search auth root cause`.
6. Run `/ctx-retrieve refresh token`.
7. Run `/ctx-pack fix refresh token rotation`.
8. Compare `AGENTS.md` against graph memory with `/ctx-benchmark-memory-ab`.
```

```md
# CTX Demo Script

Use this as a live presentation order:
- show AGENTS-style markdown first
- import it into graph memory
- show topic search returning only relevant directives
- show prune logs removing noise
- show pack building compact context
- show benchmark proving token savings
```

- [ ] **Step 4: Update README and guide to point to the demo docs as the official next step after setup**

```md
- [docs/demo-walkthrough.md](docs/demo-walkthrough.md): end-to-end real-world validation story
- [docs/demo-script.md](docs/demo-script.md): presentation order for live demo or recording
```

- [ ] **Step 5: Run the docs-linking test to verify it passes**

Run: `cargo test -p ctx-cli docs_link_the_demo_walkthrough_and_script -- --exact`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add docs/demo-script.md docs/demo-walkthrough.md README.md guide.md docs/install.md
# git commit -m "docs: add demo walkthrough and presentation script"
```

### Task 8: Execute The Demo Benchmark Story And Version The Reports

**Files:**
- Modify: `docs/superpowers/plans/2026-04-25-final-release-roadmap.md`
- Modify: `README.md`
- Modify: `guide.md`
- Create: `demo/fixtures/opencode-auth-lab/benchmarks/report.md`
- Create: `demo/fixtures/opencode-auth-lab/benchmarks/report.json`

- [ ] **Step 1: Write the failing test that requires versioned benchmark reports**

```rust
#[test]
fn demo_fixture_contains_versioned_benchmark_reports() {
    let root = repo_root().join("demo/fixtures/opencode-auth-lab/benchmarks");
    assert!(root.join("report.md").exists());
    assert!(root.join("report.json").exists());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ctx-cli demo_fixture_contains_versioned_benchmark_reports -- --exact`
Expected: FAIL.

- [x] **Step 3: Run the benchmark script against the fixture and commit the outputs**

Run: `scripts/demo/opencode-auth-lab-benchmark.sh ./target/debug/ctx`
Expected: writes `demo/fixtures/opencode-auth-lab/benchmarks/report.md` and `report.json`.

- [x] **Step 4: Update roadmap and docs to mark the demo-validation slice complete**

```md
- Demo fixture now exists and is versioned in-repo.
- OpenCode-first graph-memory validation is reproducible on a real fixture.
- Benchmark reports are committed for the demo fixture.
```

- [x] **Step 5: Run the benchmark-report test to verify it passes**

Run: `cargo test -p ctx-cli demo_fixture_contains_versioned_benchmark_reports -- --exact`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add demo/fixtures/opencode-auth-lab/benchmarks/report.md demo/fixtures/opencode-auth-lab/benchmarks/report.json docs/superpowers/plans/2026-04-25-final-release-roadmap.md README.md guide.md
# git commit -m "feat: version demo benchmark reports"
```

---

## Self-Review Checklist

- Every explicit CTX capability we care about for the demo is exercised by the fixture: graph, memory bootstrap/search, retrieval, prune, pack, MCP, benchmark, security.
- The fixture repo is realistic enough to be credible, but small enough to remain deterministic in tests.
- The primary user story stays OpenCode-first even when validation scripts use raw CLI commands underneath.
- Benchmark reports are versioned so the repo contains evidence, not only claims.
- README stays product-focused while `guide.md` and demo docs carry the operational detail.

## Final Acceptance Criteria

The demo implementation is complete when:

- `demo/fixtures/opencode-auth-lab/` can be indexed and used end-to-end by CTX
- `ctx opencode install` works against the demo fixture and OpenCode commands are generated
- graph memory can be bootstrapped from AGENTS-style files and queried by topic
- noisy logs and diffs in the demo fixture are pruned into useful signal
- compact packs for the demo task contain graph, memory, and recent signal fragments
- MCP smokes pass on the demo fixture
- benchmark reports are generated and versioned for the demo fixture
- README, guide, install docs, roadmap, and demo docs all describe the same OpenCode-first validation story
