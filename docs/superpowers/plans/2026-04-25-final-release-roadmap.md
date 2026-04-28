# CTX Final Release Roadmap

Goal: ship CTX as an OpenCode-first local context runtime that users can install, enable in a repository, and use naturally from inside OpenCode.

## Current Product Position

CTX is OpenCode-first. The daily user path is:

```bash
ctx init
ctx index
ctx opencode install
opencode
```

Then inside OpenCode:

```text
/ctx
```

Wrapper-first public CLI entrypoints have been removed. Future host work should be native to that host, not a revival of wrapper commands.

## Completed

- [x] OpenCode repo-local bootstrap through `ctx opencode install`
- [x] `opencode.json` merge that preserves existing non-CTX MCP servers
- [x] local MCP stdio registration for OpenCode
- [x] generated `.opencode/commands/*.md` surface for CTX commands
- [x] `/ctx` command center with command categories and recommended next steps
- [x] `.opencode/instructions/ctx-host-first.md` host-first guidance
- [x] removal of wrapper-first public CLI entrypoints
- [x] graph memory CRUD, markdown import/bootstrap/export, and topic search
- [x] markdown-vs-graph memory benchmark harness with Markdown/JSON reports
- [x] OpenCode auth-lab fixture and demo smoke scripts
- [x] Rust/Python/TypeScript/JavaScript symbol extraction
- [x] dependency/call graph enrichment from indexed symbols
- [x] parser-aware log/diff pruning coverage
- [x] richer packer sections for memory, recent diff, failures, and decisions
- [x] release archive build, checksum, manifest, and verification scripts
- [x] final QA script for release validation

## Current Evidence

Fixture: `demo/fixtures/opencode-auth-lab`

- markdown rule tokens: `744`
- graph memory tokens: `180`
- token reduction: `75.81%`
- markdown answer success: `33.33%`
- graph-memory answer success: `100.00%`
- quality winner: `graph`

Reports:

- `demo/fixtures/opencode-auth-lab/benchmarks/report.md`
- `demo/fixtures/opencode-auth-lab/benchmarks/report.json`

## Remaining Before Public GitHub Release

1. Add screenshots and video demo assets after manual OpenCode validation.
2. Test the OpenCode flow on at least one real external repository.
3. Re-run and commit updated benchmark reports after real-repo validation if the benchmark suite changes.
4. Finalize public repository URL in workspace metadata and release docs.
5. Finalize Homebrew tap coordinates and update `Formula/ctx.rb` with real release URL/SHA256.
6. Publish a GitHub release with install instructions, demo script, benchmark proof, and checksum verification.

## Release Gate

Before tagging:

```bash
cargo fmt --all
cargo test --workspace
scripts/demo/opencode-auth-lab-smoke.sh ./target/debug/ctx
scripts/demo/opencode-auth-lab-mcp-smoke.sh ./target/debug/ctx
scripts/demo/opencode-auth-lab-benchmark.sh ./target/debug/ctx
scripts/release/final-qa.sh
```

## Non-Goals For v0.1

- wrapper-style daily commands for other hosts
- cloud sync or remote telemetry
- claiming benchmark results beyond the committed fixture and any validated real repos
- automatic global installation into every OpenCode project without explicit user action
