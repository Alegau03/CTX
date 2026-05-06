# CTX Update Design

## Goal

Add a public `ctx update` command that makes CTX installation feel polished and predictable across the supported public distribution channels.

The command must:

- report the installed version and latest available version
- detect the install channel when possible
- update installer-based installs directly
- guide Cargo, npm, and Homebrew users with the exact correct upgrade command
- avoid guessing when the install channel is ambiguous

## Scope

This design covers:

- new public CLI command: `ctx update`
- installer metadata marker written by `scripts/install.sh`
- documentation updates for install and update flows

This design does not cover:

- changing package names
- adding new release channels
- building a self-updater for Cargo, npm, or Homebrew

## User Experience

### Check-only mode

`ctx update --check` prints:

- current version
- latest version from GitHub Releases
- detected channel when available
- whether an update is available

It never changes the installation.

### Default update mode

`ctx update`:

- detects the install channel
- prints the current version and target version
- updates directly only when the install channel is the official installer
- prints guided commands for Cargo, npm, and Homebrew
- falls back to safe multi-channel guidance if detection is uncertain

### Non-interactive mode

`ctx update --yes` may execute the update action only when the channel is known and safe.

For this iteration:

- `installer`: execute the official install script
- `cargo`, `npm`, `brew`: print the exact command to run and optionally execute it only when detection is certain

To keep risk low, the first implementation will execute only the installer channel automatically and keep the other channels guided, even with `--yes`.

## CLI Surface

New command:

```bash
ctx update
```

Flags:

```bash
ctx update --check
ctx update --yes
ctx update --channel installer
ctx update --channel cargo
ctx update --channel npm
ctx update --channel brew
```

## Channel Detection Strategy

Detection order:

1. explicit `--channel`
2. installer marker file
3. Homebrew-style executable path
4. npm wrapper path or `ctx-bin` / `@alegau/ctx-bin` path hints
5. Cargo home path hints
6. fallback to guided ambiguity output

### Installer marker

The official install script writes a marker JSON file in the user data directory.

Proposed location:

- macOS/Linux default: `${XDG_DATA_HOME:-$HOME/.local/share}/ctx/install.json`

Fields:

- `channel`
- `version`
- `install_dir`
- `binary_path`

This marker is authoritative for installer-based installs.

## Release Lookup

The command uses GitHub Releases as the source of truth for the latest version.

Initial implementation:

- call GitHub's `releases/latest` endpoint
- parse the tag name
- normalize `v0.x.y` to `0.x.y`

If the latest version cannot be resolved, the command should fail with a clear network/release error.

## Safety Rules

- never overwrite the current install based on uncertain channel detection
- never silently choose a package manager path
- always show current version and target version before any action
- prefer guidance over automation when confidence is low

## Implementation Areas

### CLI

- extend `crates/ctx-cli/src/main.rs`
- add `crates/ctx-cli/src/update.rs`
- add CLI tests in `crates/ctx-cli/tests/cli_behavior.rs`

### Installer

- update `scripts/install.sh` to write the installer marker after a successful install

### Docs

- `README.md`
- `docs/install.md`
- `docs/commands.md`
- `docs/release-playbook.md`

## Test Strategy

Add CLI behavior tests for:

- `ctx update --check`
- explicit channel override
- installer marker detection
- ambiguity fallback
- safe output for Cargo, npm, and Homebrew guidance

The tests should use environment overrides for deterministic behavior where possible instead of relying on real network state.
