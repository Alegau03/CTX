# CTX Release Playbook

This document is the release-facing companion to the roadmap.

It explains how to present CTX publicly once a tagged build is ready, what proof to link, and what story the GitHub release should tell.

## Release Outcome

A good CTX release should let a new user do three things without guesswork:

1. install `ctx`
2. connect it to a repository with `ctx opencode install`
3. validate the result on the in-repo demo and benchmark evidence

## GitHub Release Title

Use a title in this shape:

```text
CTX v<version>: OpenCode-first graph memory and local context runtime
```

Example:

```text
CTX v0.1.0: OpenCode-first graph memory and local context runtime
```

## Release Narrative

Lead with these points:

- CTX is a local-first context runtime for coding agents
- OpenCode is the primary daily path
- graph memory replaces repeated markdown rereads with queryable directives
- the repository contains a real fixture project plus committed benchmark evidence

Recommended release structure:

1. What CTX is
2. Why graph memory matters
3. How to install it
4. How to test it in OpenCode
5. What benchmark evidence is included

## Highlights

Call out these release highlights:

- OpenCode-native bootstrap via `ctx opencode install`
- graph-memory bootstrap from `AGENTS.md`, `CLAUDE.md`, `CODEX.md`, and Copilot instructions
- compact pack, retrieval, prune, and MCP runtime
- in-repo demo fixture with smoke scripts
- committed A/B benchmark reports for markdown vs graph memory
- reproducible release archive verification via `release-manifest.json`

## OpenCode Demo

For the public release, the canonical demo should point to:

- [docs/demo-walkthrough.md](docs/demo-walkthrough.md)
- [docs/demo-script.md](docs/demo-script.md)
- `demo/fixtures/opencode-auth-lab`

Suggested release snippet:

```bash
ctx init
ctx index
ctx opencode install
opencode
```

Then inside OpenCode:

```text
/ctx-memory-bootstrap
/ctx-memory-search auth
/ctx-pack fix refresh token bug
```

## Benchmark Evidence

Use the committed demo benchmark reports as the proof anchor:

- `demo/fixtures/opencode-auth-lab/benchmarks/report.md`
- `demo/fixtures/opencode-auth-lab/benchmarks/report.json`

Current headline result for the fixture:

- `75.81%` average token reduction
- graph quality win on the `AGENTS.md -> graph memory` scenario

When posting publicly, link the report files directly and keep claims scoped to the fixture unless broader benchmark runs are added.

## Install

Point users to:

- [docs/install.md](docs/install.md)
- [guide.md](../guide.md)

Recommended release install section:

```bash
tar -xzf ctx-<version>-<target>.tar.gz
sudo install -m 0755 ctx-<version>-<target>/ctx /usr/local/bin/ctx
ctx doctor
```

## Verification

The release body should mention that the packaged archive is verified with:

```bash
scripts/release/verify-artifact.sh dist/ctx-<version>-<target>.tar.gz dist/SHA256SUMS
```

This is important because CTX now validates the actual tarball, not just a local build tree.

## Community Messaging

Use short, concrete language. Avoid vague “AI productivity” claims.

Preferred phrases:

- local-first context runtime
- OpenCode-first workflow
- graph memory instead of giant markdown rereads
- only retrieve the directives you need
- committed benchmark evidence and demo fixture

Avoid phrases like:

- magical context engine
- universal agent replacement
- autonomous coding platform

## Release Checklist

Before publishing:

- the archive exists in `dist/`
- `SHA256SUMS` exists
- `release-manifest.json` exists
- demo smoke and MCP smoke pass
- benchmark reports are committed
- README, guide, install docs, and roadmap agree on current status
- the release notes include install, demo, and benchmark proof links
