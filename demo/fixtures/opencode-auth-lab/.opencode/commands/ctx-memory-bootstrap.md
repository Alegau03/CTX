---
description: Memory | Bootstrap graph memory from AGENTS-style markdown
---

Bootstrap CTX graph memory from conventional markdown rule files.

Arguments:
- `$ARGUMENTS`: optional explicit file paths

If no arguments are provided, run `'/Users/alessandrogautieri/Documents/coding/ctx/target/debug/ctx' --repo-root 'demo/fixtures/opencode-auth-lab' memory bootstrap` so CTX scans common files such as:
- `AGENTS.md`
- `CLAUDE.md`
- `CODEX.md`
- `.github/copilot-instructions.md`

Rules:
- run only the exact CTX command
- do not scan the repository manually to count files or directives

Then show how many files and directives were imported.
