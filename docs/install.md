# Install CTX

CTX is distributed as a local-first CLI named `ctx`.

Supported install paths:

- GitHub Releases binary archive
- Cargo source install
- Homebrew formula/tap
- Local developer build

## GitHub Releases

Download the archive for your platform from the release page:

```bash
tar -xzf ctx-0.1.0-<target>.tar.gz
sudo install -m 0755 ctx-0.1.0-<target>/ctx /usr/local/bin/ctx
```

Verify checksum:

```bash
shasum -a 256 -c SHA256SUMS
```

Verify installation:

```bash
ctx help
ctx doctor
```

Expected first-run output before `ctx init`:

```text
CTX Doctor
config: missing
next: ctx init
```

## Cargo Install

From the repository root:

```bash
cargo install --locked --path crates/ctx-cli
```

Verify:

```bash
ctx help
ctx doctor
```

## Homebrew

Formula template:

```bash
brew install ./Formula/ctx.rb
```

For a public tap, copy `Formula/ctx.rb` into the tap repository and replace:

- `homepage`
- `url`
- `sha256`

Then users can install with:

```bash
brew tap <owner>/ctx
brew install ctx
```

## Local Developer Build

```bash
cargo build --release --locked --bin ctx
./target/release/ctx help
```

Optional local PATH install:

```bash
cargo install --locked --path crates/ctx-cli
```

## First Useful Command Sequence

Run this inside an existing project:

```bash
ctx doctor
ctx init
ctx doctor
ctx index
ctx ask "where is retry logic implemented?"
```

Expected behavior:

- first `ctx doctor` reports missing config and recommends `ctx init`;
- `ctx init` creates `.ctx/config.toml`, `.ctx/graph.db`, `.ctx/packs`, `.ctx/stats` and `.ctx/audit.log`;
- second `ctx doctor` reports `config: ok`, `graph: ok`, `audit_log: ok`, `local_only: true` and `remote_upload_enabled: false`;
- `ctx index` writes project files/symbols into the local graph;
- `ctx ask` prints compact context without invoking an agent.

## Release Build

Build, test, package and smoke-test the current platform:

```bash
scripts/release/build.sh
```

Useful environment variables:

```bash
CTX_RELEASE_RUN_TESTS=0 scripts/release/build.sh
CTX_TARGET=x86_64-unknown-linux-gnu scripts/release/build.sh
CTX_DIST_DIR=/tmp/ctx-dist scripts/release/build.sh
```

Release output:

```text
dist/ctx-<version>-<target>.tar.gz
dist/SHA256SUMS
```

## Install Smoke Test

Smoke-test an installed or packaged binary:

```bash
scripts/release/install-smoke.sh ./target/release/ctx
```

What it verifies:

- `ctx help`
- `ctx doctor` before init
- `ctx init`
- `ctx doctor` after init
- `ctx index`
- `ctx pack`
- `ctx stats`
- `ctx mcp stdio`

## Troubleshooting

If `ctx doctor` reports `config: missing`:

```bash
ctx init
```

If `ctx doctor` reports graph/runtime files missing after init:

```bash
ctx init
ctx index
```

If shell cannot find `ctx`, check PATH:

```bash
which ctx
echo "$PATH"
```
