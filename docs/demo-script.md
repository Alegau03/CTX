# CTX Demo Script

Use this order for a live demo or recording.

1. Show the fixture repo at `demo/fixtures/opencode-auth-lab/`.
2. Show the classic markdown rules in `AGENTS.md` and `.github/copilot-instructions.md`.
3. Run `ctx init`, `ctx index`, and `ctx opencode install`.
4. Open OpenCode.
5. Run `/ctx-memory-bootstrap`.
6. Run `/ctx-memory-search auth root cause`.
7. Run `/ctx-retrieve refresh token auth failure`.
8. Run `/ctx-prune-logs` on the noisy Vitest log.
9. Run `/ctx-pack fix refresh token rotation`.
10. Run `/ctx-benchmark-memory-ab` or the benchmark suite to show the token delta story.

The key message is that CTX lets the host retrieve only the relevant memory and code context instead of re-reading large markdown files and noisy logs every time.
