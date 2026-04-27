# OpenCode-Native Final QA

This checklist is the final human-readable QA pass for CTX as an OpenCode-first product.

## Goal

Confirm that a new user can install CTX, bootstrap a repository, open OpenCode, and use the graph-memory workflow without wrapper-centric detours.

## Automated Gate

Run:

```bash
scripts/release/final-qa.sh
```

This script builds the release archive, verifies the final tarball, and reruns the install, OpenCode, demo, MCP, and benchmark validations.

## Manual OpenCode-Native QA

Use the fixture repository first:

```bash
ctx --repo-root demo/fixtures/opencode-auth-lab init
ctx --repo-root demo/fixtures/opencode-auth-lab index
ctx --repo-root demo/fixtures/opencode-auth-lab opencode install
cd demo/fixtures/opencode-auth-lab
opencode
```

Inside OpenCode, verify these commands in order:

1. `/ctx`
2. `/ctx-doctor`
3. `/ctx-memory-bootstrap`
4. `/ctx-memory-search auth`
5. `/ctx-retrieve refresh route`
6. `/ctx-pack fix refresh token bug`
7. `/ctx-prune-logs refresh token`
8. `/ctx-benchmark-memory-suite benchmarks/memory-suite.toml benchmarks/report.md benchmarks/report.json`

## What Should Be True

- OpenCode sees CTX commands under `.opencode/commands/`
- `/ctx` shows a categorized CTX command center with a clear recommended next step
- `opencode.json` keeps CTX registered as a local MCP server
- `/ctx-memory-bootstrap` imports rules from `AGENTS.md`-style files
- `/ctx-memory-search auth` surfaces only relevant directives
- `/ctx-pack` includes graph and memory context instead of a giant markdown dump
- log pruning keeps root-cause signal and removes noise
- benchmark commands complete and regenerate the report files

## Repo-Level QA

Also verify on a non-demo repository:

1. `ctx init`
2. `ctx index`
3. `ctx opencode install`
4. open `opencode`
5. run `/ctx-doctor`
6. run `/ctx-pack <real task>`
7. run `/ctx-memory-bootstrap` if the repo has an `AGENTS.md`-style file

## Release Readiness Questions

A release is ready if all answers are yes:

- Can a user stay inside OpenCode for daily CTX usage?
- Does graph memory work better than rereading a giant markdown file on the demo fixture?
- Are the benchmark reports already committed and reproducible?
- Does the packaged archive pass verification after unpacking?
- Do README, guide, install docs, demo docs, and roadmap all tell the same story?
