# CTX Demo Script

Use this order for a live demo or recording.

1. Show `demo/fixtures/opencode-auth-lab`.
2. Show `AGENTS.md`, `CLAUDE.md`, and `CODEX.md`, and explain that CTX can ingest these compatibility files into graph memory instead of rereading them wholesale every time.
3. Run `ctx init`, `ctx index`, and `ctx opencode install`.
4. Open `opencode`.
5. Run `/ctx` to show the command center.
6. Run `/ctx-memory-bootstrap` to import markdown rules into graph memory.
7. Run `/ctx-memory-search auth root cause` to show targeted memory retrieval.
8. Run `/ctx-retrieve refresh token auth failure` to show code retrieval.
9. Run `/ctx-prune-logs npm test -- --grep "refresh"` if demonstrating noisy output cleanup.
10. Run `/ctx-pack fix refresh token rotation` to show compact task context.
11. Run `/ctx-benchmark-memory-suite benchmarks/memory-suite.toml benchmarks/report.md benchmarks/report.json`.
12. Open the report and highlight the current fixture result: `75.81%` fewer rule tokens with graph memory and a graph-quality win.

Closing message: CTX lets OpenCode retrieve the memory and code context it needs without repeatedly paying for full markdown files, huge logs, or broad diffs.
