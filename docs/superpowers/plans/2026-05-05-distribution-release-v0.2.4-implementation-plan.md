# CTX v0.2.4 Distribution And Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish CTX `v0.2.4` across GitHub Releases, crates.io, npm, and the public Homebrew tap with verified artifacts and synchronized install/update docs.

**Architecture:** Release preparation happens on `dev`, with version coherence enforced before the release is built. GitHub Releases remains the binary source of truth, `@alegau/ctx-bin` follows the GitHub tag, and the Homebrew tap is updated from the verified release-ready version before the public install smoke.

**Tech Stack:** Rust workspace, shell release scripts, GitHub CLI, npm, Homebrew, Markdown docs

---

### Task 1: Bump release-visible version references to `0.2.4`

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `packages/ctx-bin/package.json`
- Modify: `Formula/ctx.rb`
- Modify: `scripts/release/build.sh`
- Modify: `README.md`
- Modify: `crates/ctx-mcp/src/lib.rs`
- Test: `crates/ctx-cli/tests/cli_behavior.rs`

- [ ] **Step 1: Write the failing test**

Add or update a focused release test that expects the public version references to use `0.2.4` instead of `0.1.0`.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p ctx release_ -- --nocapture
```

Expected:

- FAIL because at least one public version reference still points to `0.1.0`

- [ ] **Step 3: Write minimal implementation**

Update all release-visible version references to `0.2.4`, keeping test fixture strings at `0.1.0` only where they intentionally model an old artifact name in isolated tests.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p ctx release_ -- --nocapture
```

Expected:

- PASS

### Task 2: Harden release scripts and package metadata

**Files:**
- Modify: `scripts/release/publish-crate.sh`
- Modify: `scripts/release/publish-npm.sh`
- Modify: `scripts/release/prepare-homebrew-formula.sh`
- Modify: `Formula/ctx.rb`
- Modify: `packages/ctx-bin/README.md`
- Test: `crates/ctx-cli/tests/cli_behavior.rs`

- [ ] **Step 1: Write the failing test**

Add or extend a release asset test to assert:

- publish scripts mention the right public package names
- Homebrew formula metadata and script behavior are aligned with `v0.2.4`
- release docs/scripts stay coherent with install/update expectations

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p ctx release_assets_ -- --nocapture
```

Expected:

- FAIL because scripts or metadata still reflect pre-release assumptions

- [ ] **Step 3: Write minimal implementation**

Adjust release scripts and metadata so they are safe to run for the public `v0.2.4` publish flow.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p ctx release_assets_ -- --nocapture
```

Expected:

- PASS

### Task 3: Sync install and update documentation for `v0.2.4`

**Files:**
- Modify: `README.md`
- Modify: `docs/install.md`
- Modify: `docs/commands.md`
- Modify: `docs/release-playbook.md`
- Modify: `guide.md`
- Modify: `packages/ctx-bin/README.md`
- Test: `crates/ctx-cli/tests/cli_behavior.rs`

- [ ] **Step 1: Write the failing test**

Use the existing doc-oriented release tests as the red bar for public install/update wording and `v0.2.4` references.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p ctx release_docs_ -- --nocapture
```

Expected:

- FAIL until docs match the final public release shape

- [ ] **Step 3: Write minimal implementation**

Make the install, update, verification, and release docs consistent with the actual public commands and release flow.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p ctx release_docs_ -- --nocapture
```

Expected:

- PASS

### Task 4: Verify the release candidate on `dev`

**Files:**
- Modify: release-touched files only if verification surfaces issues

- [ ] **Step 1: Run the focused CLI suite**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p ctx
```

- [ ] **Step 2: Build the release artifact for the host target**

```bash
PATH="$HOME/.cargo/bin:$PATH" scripts/release/build.sh
```

- [ ] **Step 3: Run npm dry-run**

```bash
npm publish packages/ctx-bin --access public --dry-run
```

- [ ] **Step 4: Run final verification gate**

```bash
scripts/release/final-qa.sh
```

### Task 5: Publish `v0.2.4`

**Files:**
- Modify: `Formula/ctx.rb`
- Modify: tap repo formula copy under `Alegau03/homebrew-ctx`

- [ ] **Step 1: Merge `dev` into `main`**

```bash
git checkout main
git merge dev
```

- [ ] **Step 2: Tag and push release state**

```bash
git tag v0.2.4
git push origin main --tags
```

- [ ] **Step 3: Publish GitHub Release assets**

Create or update the `v0.2.4` release with:

- host or multi-target archives that actually exist
- `SHA256SUMS`
- `release-manifest.json`

- [ ] **Step 4: Publish crate and npm package**

```bash
PATH="$HOME/.cargo/bin:$PATH" scripts/release/publish-crate.sh
scripts/release/publish-npm.sh
```

- [ ] **Step 5: Update the Homebrew tap**

Use the verified checksum and push the updated formula to `Alegau03/homebrew-ctx`.

### Task 6: Public-channel smoke verification

**Files:**
- Modify: docs only if real-world publish outputs expose mismatches

- [ ] **Step 1: Verify GitHub Release asset presence**

```bash
gh release view v0.2.4 --repo Alegau03/CTX
```

- [ ] **Step 2: Verify public install/update surfaces**

Run the appropriate public commands where feasible:

```bash
cargo install ctx --force
npm i -g @alegau/ctx-bin
brew upgrade ctx || brew install ctx
ctx help
ctx doctor
ctx update --check
```

- [ ] **Step 3: Handoff**

Confirm the code/docs/release are done and explicitly leave README GIF replacement as the next manual step.
