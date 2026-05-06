# CTX Release Playbook

## Release Outcome

A good CTX release lets a new user:

1. install `ctx`
2. enable a repo with `ctx opencode install`
3. open OpenCode and use `/ctx-*`
4. reproduce the demo benchmark evidence

## GitHub Release Title

```text
CTX v<version>: OpenCode-first graph memory and local context runtime
```

## Release Narrative

Lead with:

- CTX is a local-first context runtime, not another agent launcher
- OpenCode remains the primary user interface
- graph memory replaces repeated giant markdown rereads with queryable directives
- the repo includes a fixture project and reproducible benchmark report

## Benchmark Claim

## Benchmark Evidence

Current committed fixture result:

- `56.72%` token reduction on markdown rules vs graph memory
- `markdown=1.00` and `graph=1.00` query coverage
- `33.33%` markdown answer success vs `100.00%` graph-memory answer success
- graph quality win for the demo scenario with `markdown=0`, `graph=1`, `ties=0`

Proof files:

- `demo/fixtures/opencode-auth-lab/benchmarks/report.md`
- `demo/fixtures/opencode-auth-lab/benchmarks/report.json`
- `benchmarks/external/agentsmd/report.md`
- `benchmarks/external/agentsmd/report.json`

Keep public claims scoped to this fixture until broader benchmark reports are added.

## Demo Snippet

## OpenCode Demo

```bash
ctx init
ctx index
ctx opencode install
opencode
```

Inside OpenCode:

```text
/ctx
/ctx-memory-bootstrap
/ctx-memory-search auth
/ctx-pack fix refresh token bug
```

## Install Snippet

```bash
curl -fsSL https://raw.githubusercontent.com/Alegau03/CTX/main/scripts/install.sh | sh
ctx doctor
```

Alternative channels:

```bash
cargo install ctx-cli
npm i -g @alegau/ctx-bin
brew tap Alegau03/ctx
brew install ctx
ctx doctor
```

## Update Snippet

Preferred native check:

```bash
ctx update --check
```

Native update surface:

```bash
ctx update
```

Channel-specific fallbacks:

```bash
cargo install ctx-cli --force
curl -fsSL https://raw.githubusercontent.com/Alegau03/CTX/main/scripts/install.sh | sh
npm update -g @alegau/ctx-bin
brew upgrade ctx
```

## Verification

Release artifacts should be verified with:

```bash
scripts/release/verify-artifact.sh dist/ctx-<version>-<target>.tar.gz dist/SHA256SUMS
```

For multi-platform releases, publish one artifact per target and keep `SHA256SUMS` alongside all archives. If only one target ships, say that explicitly in the release notes.

Final gate:

```bash
scripts/release/final-qa.sh
```

## Release Workflow

The repository includes `.github/workflows/release.yml` for the public artifact matrix. On tag push or manual dispatch it builds:

- `aarch64-apple-darwin` on `macos-latest`
- `x86_64-apple-darwin` on `macos-15-intel`
- `x86_64-unknown-linux-gnu` on `ubuntu-latest`
- `x86_64-pc-windows-msvc` on `windows-latest`

The workflow assembles a combined `SHA256SUMS` and `release-manifest.json`, then publishes the GitHub Release assets for that tag.

## Checklist

- README and guide are OpenCode-first
- screenshots/video placeholder exists and demo script is ready
- benchmark reports are committed and reproducible
- published artifacts match the platforms promised in README and release notes
- package verification passes after unpacking
- Homebrew formula coordinates are updated before tap publication
- installer marker is present after `scripts/install.sh` install smoke
