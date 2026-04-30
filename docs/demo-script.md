# CTX Demo Script

Use this order for a live demo or recording.

1. Show `demo/fixtures/opencode-auth-lab`.
2. Show `AGENTS.md`, `CLAUDE.md`, and `CODEX.md`, and explain that CTX can ingest these compatibility files into graph memory instead of rereading them wholesale every time.
3. Run `ctx init`, `ctx index`, and `ctx opencode install`.
4. Run `npm install` inside the fixture so the auth log demo uses real Vitest output.
5. Open `opencode`.
6. Run `/ctx` to show the command center.
7. Run `/ctx-memory-bootstrap` to import `27` markdown directives into graph memory.
8. Run `/ctx-memory-search auth root cause` to show targeted memory retrieval.
9. Run `/ctx-retrieve refresh token auth failure` to show code retrieval.
10. Run `/ctx-prune-logs npm run test:auth` if demonstrating noisy output cleanup.
11. Run `/ctx-pack fix refresh token rotation` to show compact task context.
12. Run `/ctx-benchmark-memory-suite benchmarks/memory-suite.toml benchmarks/report.md benchmarks/report.json`.
13. Open the report and highlight the current fixture result: `56.72%` fewer rule tokens with graph memory, `markdown=1.00` and `graph=1.00` query coverage, and a graph-quality win.

Closing message: CTX lets OpenCode retrieve the memory and code context it needs without repeatedly paying for full markdown files, huge logs, or broad diffs.
