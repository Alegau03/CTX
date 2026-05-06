# CTX Distribution And Installation Plan

## Goal

Ship CTX with the same public install confidence users expect from polished CLI products:

- `cargo install ctx`
- `curl -fsSL ... | sh`
- `npm i -g @alegau/ctx-bin`
- `brew tap Alegau03/ctx && brew install ctx`
- clear update paths for every channel
- optional native `ctx update` command that upgrades CTX to the latest version

This document is the single source of truth for the final distribution rollout.

## What Already Exists In The Repo

These pieces are already present and should be reused, not reinvented:

### Release And Verification

- `scripts/release/build.sh`
- `scripts/release/verify-artifact.sh`
- `scripts/release/install-smoke.sh`
- `scripts/release/opencode-smoke.sh`
- `scripts/release/final-qa.sh`

### Installer And Package Scaffolding

- `scripts/install.sh`
- `packages/ctx-bin/package.json`
- `packages/ctx-bin/install.js`
- `packages/ctx-bin/bin/ctx.js`
- `scripts/release/publish-crate.sh`
- `scripts/release/publish-npm.sh`
- `scripts/release/prepare-homebrew-formula.sh`

### Docs To Keep In Sync

- `README.md`
- `docs/install.md`
- `guide.md`
- `docs/release-playbook.md`
- `packages/ctx-bin/README.md`

## Final Public Distribution Surface

### Install

```bash
cargo install ctx
curl -fsSL https://raw.githubusercontent.com/Alegau03/CTX/main/scripts/install.sh | sh
npm i -g @alegau/ctx-bin
brew tap Alegau03/ctx
brew install ctx
```

### Update

```bash
cargo install ctx --force
curl -fsSL https://raw.githubusercontent.com/Alegau03/CTX/main/scripts/install.sh | sh
npm update -g @alegau/ctx-bin
brew upgrade ctx
```

### Native CTX Update Command

Desired UX:

```bash
ctx update
```

Expected behavior:

- detect the user install channel when possible
- if installed by the official installer, self-update via the latest GitHub Release
- if installed by Homebrew, print `brew upgrade ctx`
- if installed by npm, print `npm update -g @alegau/ctx-bin`
- if installed by Cargo, print `cargo install ctx --force`
- print the installed version and the target version before applying or suggesting the update

If channel detection is ambiguous, the command should fall back to a safe guided message instead of guessing.

## Release Order

1. finish doc cleanup and release-ready README
2. merge `dev` into `main`
3. tag the release
4. build and upload GitHub Release artifacts
5. publish crate to `crates.io`
6. publish `@alegau/ctx-bin` to npm
7. update and publish Homebrew tap
8. verify install and update on a clean machine
9. publish final screenshots, GIFs, and launch post

## Platform Matrix

Minimum public matrix:

| Platform | Artifact |
|---|---|
| macOS Apple Silicon | `ctx-<version>-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `ctx-<version>-x86_64-apple-darwin.tar.gz` |
| Linux x64 | `ctx-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Windows x64 | `ctx-<version>-x86_64-pc-windows-msvc.zip` |

Every release must also ship:

- `SHA256SUMS`
- `release-manifest.json`

## Step-By-Step Release Execution

### 1. Merge And Tag

- merge `dev` into `main`
- bump version consistently where needed
- create the release tag

Example:

```bash
git checkout main
git merge dev
git tag v0.x.y
git push origin main --tags
```

### 2. Build Release Artifacts

Run the release build on the matching OS runners or machines for each target:

```bash
scripts/release/build.sh
CTX_TARGET=x86_64-apple-darwin scripts/release/build.sh
CTX_TARGET=x86_64-unknown-linux-gnu scripts/release/build.sh
CTX_TARGET=x86_64-pc-windows-msvc scripts/release/build.sh
```

### 3. Verify Artifacts

```bash
scripts/release/verify-artifact.sh dist/ctx-<version>-<target>.tar.gz dist/SHA256SUMS
scripts/release/verify-artifact.sh dist/ctx-<version>-<target>.zip dist/SHA256SUMS
scripts/release/final-qa.sh
```

### 4. Publish GitHub Release

Upload:

- platform archives
- `SHA256SUMS`
- `release-manifest.json`

The GitHub Release is the source of truth for:

- installer downloads
- npm binary package downloads
- manual archive installs

### 5. Publish The Crate

The crate metadata has already been prepared so the public path is:

```bash
cargo install ctx
```

Publish with:

```bash
scripts/release/publish-crate.sh
```

Manual fallback:

```bash
cargo publish -p ctx
```

### 6. Publish npm Package

Publish with:

```bash
scripts/release/publish-npm.sh
```

Expected public UX:

```bash
npm i -g @alegau/ctx-bin
ctx help
```

### 7. Publish Homebrew Tap

Generate/update formula data with:

```bash
scripts/release/prepare-homebrew-formula.sh
```

Then push the formula to the Homebrew tap repo.

Expected public UX:

```bash
brew tap Alegau03/ctx
brew install ctx
brew upgrade ctx
```

## `ctx update` Implementation Plan

### CLI Surface

Add a new public command:

```bash
ctx update
```

Optional flags:

```bash
ctx update --check
ctx update --yes
ctx update --channel installer|cargo|npm|brew
```

### Behavior

#### `ctx update --check`

- prints current version
- fetches the latest release version
- reports whether an update is available
- does not modify the install

#### `ctx update`

- tries to detect the install channel
- if channel is `installer`, downloads and installs the latest release
- if channel is `cargo`, prints or runs `cargo install ctx --force`
- if channel is `npm`, prints or runs `npm update -g @alegau/ctx-bin`
- if channel is `brew`, prints or runs `brew upgrade ctx`
- if detection fails, prints exact update commands for every channel

### Detection Strategy

Preferred order:

1. explicit `--channel`
2. installer marker file
3. Homebrew path detection
4. npm global package detection
5. Cargo bin path detection
6. fallback guidance

### Files To Add For `ctx update`

- new update module in `crates/ctx-cli/src/`
- installer marker file path support in `scripts/install.sh`
- docs updates in:
  - `README.md`
  - `docs/install.md`
  - `docs/commands.md`
  - `docs/release-playbook.md`

## Final Documentation Checklist

Before public launch, docs must all say the same thing:

- README hero matches the real product surface
- install methods are public and not phrased as future work
- update methods are documented for every install channel
- `ctx update` is documented only once implemented
- screenshots/GIF placeholders are replaced with final assets
- no old implementation wave docs remain in the repo

## Final Verification Checklist

On a clean machine or shell, verify all of these:

### Cargo

```bash
cargo install ctx
ctx help
ctx doctor
```

### Installer

```bash
curl -fsSL https://raw.githubusercontent.com/Alegau03/CTX/main/scripts/install.sh | sh
ctx help
ctx doctor
```

### npm

```bash
npm i -g @alegau/ctx-bin
ctx help
ctx doctor
```

### Homebrew

```bash
brew tap Alegau03/ctx
brew install ctx
ctx help
ctx doctor
```

### Update Paths

```bash
cargo install ctx --force
curl -fsSL https://raw.githubusercontent.com/Alegau03/CTX/main/scripts/install.sh | sh
npm update -g @alegau/ctx-bin
brew upgrade ctx
```

### OpenCode Enablement

```bash
ctx init
ctx index
ctx opencode install
opencode
```

### In OpenCode

```text
/ctx
/ctx-plan <task>
/ctx-pack <task>
/ctx-read <file> digest
/ctx-run <shell command>
/ctx-gain
```
