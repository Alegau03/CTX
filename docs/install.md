# Install CTX

CTX is distributed as a local CLI named `ctx`. The CLI bootstraps the runtime, installs OpenCode project assets, and exposes local MCP tools.

## Quick Install

### Cargo

```bash
cargo install ctx-cli
```

### One-Line Installer

```bash
curl -fsSL https://raw.githubusercontent.com/Alegau03/CTX/main/scripts/install.sh | sh
```

### npm

```bash
npm i -g @alegau/ctx-bin
```

### Homebrew

```bash
brew tap Alegau03/ctx
brew install ctx
```

### Source Install

```bash
git clone https://github.com/Alegau03/CTX.git
cd CTX
cargo install --locked --path crates/ctx-cli
```

Verify the installed binary:

```bash
ctx help
ctx doctor
ctx update --check
```

## Update CTX

Use the update path that matches how CTX was installed.

### Native Update Command

```bash
ctx update
ctx update --check
```

What it does:

- prints the installed version and latest available version
- detects the install channel when possible
- reruns the official installer for installer-based installs
- prints the exact correct update command for Cargo, npm, and Homebrew installs
- falls back to safe multi-channel guidance if detection is ambiguous

### Cargo

```bash
cargo install ctx-cli --force
```

### Installer Script

```bash
curl -fsSL https://raw.githubusercontent.com/Alegau03/CTX/main/scripts/install.sh | sh
```

### npm

```bash
npm update -g @alegau/ctx-bin
```

### Homebrew

```bash
brew upgrade ctx
```

### `--channel` Override

If CTX cannot safely infer how it was installed, force the expected channel:

```bash
ctx update --channel installer
ctx update --channel cargo
ctx update --channel npm
ctx update --channel brew
```

## GitHub Releases

Release artifacts live at:

```text
https://github.com/Alegau03/CTX/releases
```

Current archive naming:

| Platform | Artifact |
|---|---|
| macOS Apple Silicon | `ctx-<version>-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `ctx-<version>-x86_64-apple-darwin.tar.gz` |
| Linux x64 | `ctx-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Windows x64 | `ctx-<version>-x86_64-pc-windows-msvc.zip` |

Verify checksums:

```bash
shasum -a 256 -c SHA256SUMS
```

Install from an archive on macOS or Linux:

```bash
tar -xzf ctx-<version>-<target>.tar.gz
sudo install -m 0755 ctx-<version>-<target>/ctx /usr/local/bin/ctx
```

Archive installs are not automatically identifiable by `ctx update`, so use an explicit channel override or rerun the official installer if you want to move to the installer-managed path.

## Enable CTX In A Repository

```bash
cd /path/to/your/project
ctx init
ctx index
ctx opencode install
```

Optional lean setup:

```bash
ctx opencode install --profile core
```

Then open OpenCode:

```bash
opencode
```

Start with:

```text
/ctx
```

## Release Build

Build, test, package, and smoke-test the current platform:

```bash
scripts/release/build.sh
```

For the public multi-platform release matrix, use the GitHub Actions workflow in `.github/workflows/release.yml`. It builds the release artifacts on:

- `macos-latest` for `aarch64-apple-darwin`
- `macos-15-intel` for `x86_64-apple-darwin`
- `ubuntu-latest` for `x86_64-unknown-linux-gnu`
- `windows-latest` for `x86_64-pc-windows-msvc`

Useful environment variables:

```bash
CTX_RELEASE_RUN_TESTS=0 scripts/release/build.sh
rustup target add x86_64-apple-darwin
CTX_TARGET=x86_64-apple-darwin scripts/release/build.sh
CTX_DIST_DIR=/tmp/ctx-dist scripts/release/build.sh
```

Release output:

```text
dist/ctx-<version>-<target>.tar.gz
dist/ctx-<version>-<target>.zip
dist/SHA256SUMS
dist/release-manifest.json
```

## Verification

Installed binary smoke:

```bash
scripts/release/install-smoke.sh ./target/release/ctx
```

OpenCode integration smoke:

```bash
scripts/release/opencode-smoke.sh ./target/release/ctx
```

Archive verification:

```bash
scripts/release/verify-artifact.sh dist/ctx-<version>-<target>.tar.gz dist/SHA256SUMS
scripts/release/verify-artifact.sh dist/ctx-<version>-<target>.zip dist/SHA256SUMS
```

Final release gate:

```bash
scripts/release/final-qa.sh
```
