# CTX Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `ctx update` with safe channel detection, installer marker support, and synced install/update documentation.

**Architecture:** The CLI owns update planning and output in a dedicated `update.rs` module, while the installer writes a lightweight metadata marker that makes official-installer detection deterministic. Public docs are updated in the same batch so every install channel advertises the same update story.

**Tech Stack:** Rust, Clap, assert_cmd, shell installer script, Markdown docs

---

### Task 1: Add failing CLI coverage for update behavior

**Files:**
- Modify: `crates/ctx-cli/tests/cli_behavior.rs`
- Test: `crates/ctx-cli/tests/cli_behavior.rs`

- [ ] **Step 1: Write the failing tests**

Add tests that cover:

- `ctx update --check` prints current version labels
- `ctx update --channel cargo --check` reports `cargo` as the selected channel
- `ctx update --channel brew` prints `brew upgrade ctx`
- ambiguous detection prints all supported update paths

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p ctx-cli update_ -- --nocapture
```

Expected:

- FAIL because `update` command does not exist yet

- [ ] **Step 3: Write minimal implementation**

Add the command surface and enough output logic to satisfy the new tests before adding deeper detection behavior.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p ctx-cli update_ -- --nocapture
```

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add crates/ctx-cli/tests/cli_behavior.rs crates/ctx-cli/src/main.rs crates/ctx-cli/src/update.rs
git commit -m "feat: add ctx update command surface"
```

### Task 2: Add installer marker support and deterministic detection

**Files:**
- Create: `crates/ctx-cli/src/update.rs`
- Modify: `scripts/install.sh`
- Test: `crates/ctx-cli/tests/cli_behavior.rs`

- [ ] **Step 1: Write the failing test**

Add a test that creates a fake installer marker, points CTX to it with an environment override, and expects:

- detected channel `installer`
- update action text that uses the official installer

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p ctx-cli installer_marker -- --nocapture
```

Expected:

- FAIL because marker detection is not implemented yet

- [ ] **Step 3: Write minimal implementation**

Implement:

- installer marker parsing
- detection precedence
- successful installer marker write in `scripts/install.sh`

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p ctx-cli installer_marker -- --nocapture
```

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add crates/ctx-cli/src/update.rs crates/ctx-cli/tests/cli_behavior.rs scripts/install.sh
git commit -m "feat: detect installer-based ctx installs"
```

### Task 3: Add safe fallback and guided outputs for other channels

**Files:**
- Modify: `crates/ctx-cli/src/update.rs`
- Test: `crates/ctx-cli/tests/cli_behavior.rs`

- [ ] **Step 1: Write the failing tests**

Add tests for:

- Homebrew output includes `brew upgrade ctx`
- npm output includes `npm update -g @alegau/ctx-bin`
- Cargo output includes `cargo install ctx --force`
- ambiguity output lists all commands

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p ctx-cli guided_update -- --nocapture
```

Expected:

- FAIL because guided update messaging is incomplete

- [ ] **Step 3: Write minimal implementation**

Implement:

- exact per-channel command rendering
- ambiguity-safe fallback messaging
- `--check` reporting without mutation

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p ctx-cli guided_update -- --nocapture
```

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add crates/ctx-cli/src/update.rs crates/ctx-cli/tests/cli_behavior.rs
git commit -m "feat: add safe guided updates for package-manager installs"
```

### Task 4: Sync public install and update documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/install.md`
- Modify: `docs/commands.md`
- Modify: `docs/release-playbook.md`

- [ ] **Step 1: Write the doc updates**

Document:

- `ctx update`
- `ctx update --check`
- channel-specific guidance
- installer marker-backed official installer behavior

- [ ] **Step 2: Verify docs are consistent**

Run:

```bash
rg -n "ctx update|npm update -g @alegau/ctx-bin|brew upgrade ctx|cargo install ctx --force" README.md docs/install.md docs/commands.md docs/release-playbook.md
```

Expected:

- every install/update surface uses the same commands

- [ ] **Step 3: Commit**

```bash
git add README.md docs/install.md docs/commands.md docs/release-playbook.md
git commit -m "docs: add ctx update and sync install guidance"
```

### Task 5: Final verification

**Files:**
- Modify: `crates/ctx-cli/src/main.rs`
- Modify: `crates/ctx-cli/src/update.rs`
- Modify: `crates/ctx-cli/tests/cli_behavior.rs`
- Modify: `scripts/install.sh`
- Modify: `README.md`
- Modify: `docs/install.md`
- Modify: `docs/commands.md`
- Modify: `docs/release-playbook.md`

- [ ] **Step 1: Run focused CLI tests**

```bash
cargo test -p ctx-cli update_ -- --nocapture
```

- [ ] **Step 2: Run the full ctx-cli test suite**

```bash
cargo test -p ctx-cli
```

- [ ] **Step 3: Smoke-check the help output**

```bash
cargo run -p ctx-cli -- help
cargo run -p ctx-cli -- update --check --channel cargo
```

- [ ] **Step 4: Commit final polish if needed**

```bash
git add crates/ctx-cli/src/main.rs crates/ctx-cli/src/update.rs crates/ctx-cli/tests/cli_behavior.rs scripts/install.sh README.md docs/install.md docs/commands.md docs/release-playbook.md
git commit -m "feat: ship easy ctx update flow"
```
