# Install CTX

CTX is distributed as a local CLI named `ctx`. The CLI bootstraps the local runtime, installs OpenCode project assets, and exposes MCP tools.

Daily usage is OpenCode-first:

1. install `ctx`
2. run `ctx init`, `ctx index`, and `ctx opencode install` in a repo
3. open `opencode`
4. use `/ctx-*` commands inside OpenCode

## Cargo Install

From the repository root:

```bash
cargo install --locked --path crates/ctx-cli
```

If `ctx` is installed but not found:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Verify:

```bash
ctx help
ctx doctor
```

## GitHub Releases

After public releases are published, download the archive for your platform:

Project repository:

```text
https://github.com/Alegau03/CTX
```

```bash
tar -xzf ctx-0.1.0-<target>.tar.gz
sudo install -m 0755 ctx-0.1.0-<target>/ctx /usr/local/bin/ctx
```

Verify checksum:

```bash
shasum -a 256 -c SHA256SUMS
```

Verify install:

```bash
ctx help
ctx doctor
```

## Homebrew

Local formula test:

```bash
brew install ./Formula/ctx.rb
```

The formula now points at the public GitHub repository. Before publishing a public tap or release, update only the final `sha256` in `Formula/ctx.rb` to match the uploaded archive.

## Enable A Repository

Run from a project root:

```bash
ctx init
ctx index
ctx opencode install
```

Expected result:

- `.ctx/config.toml` exists
- `.ctx/graph.db` exists
- `opencode.json` includes a local CTX MCP server
- `.opencode/commands/*.md` exists
- `.opencode/instructions/ctx-host-first.md` exists
- optional compatibility rule files such as `AGENTS.md`, `CLAUDE.md`, `CODEX.md`, and `.github/copilot-instructions.md` can later be imported into graph memory with `/ctx-memory-bootstrap`

Then open OpenCode:

```bash
opencode
```

Start inside OpenCode:

```text
/ctx
```

## Release Build

Build, test, package, and smoke-test the current platform:

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
dist/release-manifest.json
```

## Smoke Tests

Installed binary smoke:

```bash
scripts/release/install-smoke.sh ./target/release/ctx
```

OpenCode integration smoke:

```bash
scripts/release/opencode-smoke.sh ./target/release/ctx
```

Demo fixture smoke:

```bash
scripts/demo/opencode-auth-lab-smoke.sh ./target/release/ctx
scripts/demo/opencode-auth-lab-mcp-smoke.sh ./target/release/ctx
scripts/demo/opencode-auth-lab-benchmark.sh ./target/release/ctx
```

Final QA gate:

```bash
scripts/release/final-qa.sh
```

Archive verification:

```bash
scripts/release/verify-artifact.sh dist/ctx-<version>-<target>.tar.gz dist/SHA256SUMS
```

Release metadata:

```text
dist/release-manifest.json
```
