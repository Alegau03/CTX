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

- `75.81%` token reduction on markdown rules vs graph memory
- `33.33%` markdown answer success vs `100.00%` graph-memory answer success
- graph quality win for the demo scenario

Proof files:

- `demo/fixtures/opencode-auth-lab/benchmarks/report.md`
- `demo/fixtures/opencode-auth-lab/benchmarks/report.json`

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
tar -xzf ctx-<version>-<target>.tar.gz
sudo install -m 0755 ctx-<version>-<target>/ctx /usr/local/bin/ctx
ctx doctor
```

## Verification

Release artifacts should be verified with:

```bash
scripts/release/verify-artifact.sh dist/ctx-<version>-<target>.tar.gz dist/SHA256SUMS
```

Final gate:

```bash
scripts/release/final-qa.sh
```

## Checklist

- README and guide are OpenCode-first
- screenshots/video placeholder exists and demo script is ready
- benchmark reports are committed and reproducible
- release archive and SHA256SUMS are generated
- package verification passes after unpacking
- Homebrew formula coordinates are updated before tap publication
