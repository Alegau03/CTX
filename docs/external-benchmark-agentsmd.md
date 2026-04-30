# External Benchmark: `agentsmd/agents.md`

This benchmark adds one public-repository data point beyond the in-repo fixture.

## Repository

- Repo: [agentsmd/agents.md](https://github.com/agentsmd/agents.md)
- Scope: root `AGENTS.md`
- Why this repo: it is public, stable, and centered on agent guidance, which makes it a clean fit for CTX graph-memory benchmarking

## Query

```text
npm run dev npm run build lockfile development server
```

This query targets the repository guidance about:

- using `npm run dev` while iterating
- avoiding `npm run build` during agent sessions
- keeping lockfiles in sync after dependency changes
- restarting the development server after dependency changes

## Inputs

Committed benchmark inputs live in:

- `benchmarks/external/agentsmd/checklist.md`
- `benchmarks/external/agentsmd/markdown-answer.txt`
- `benchmarks/external/agentsmd/graph-answer.txt`

## Reproduce

```bash
scripts/demo/agentsmd-external-benchmark.sh ./target/debug/ctx
```

By default this regenerates:

- `benchmarks/external/agentsmd/report.md`
- `benchmarks/external/agentsmd/report.json`

## Current Snapshot

- Token reduction: `72.62%`
- Query coverage: `markdown=1.00`, `graph=0.89`
- Success rate: `markdown=0.50`, `graph=1.00`
- Quality wins: `markdown=0`, `graph=1`, `ties=0`

This benchmark is still narrower than the fixture benchmark: it measures one public `AGENTS.md` workflow on a real repository, not a full end-to-end coding task.
