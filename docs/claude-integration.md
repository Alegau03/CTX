# CTX Inside Claude Code

## Goal

Make CTX usable inside Claude Code through native project-local assets, without bringing back wrapper-style public commands.

## Current Implementation

Bootstrap:

```bash
ctx claude install
```

Generated assets:

- `.mcp.json`
- `.claude/skills/ctx-*/SKILL.md`

What the bootstrap does:

- registers CTX as a repo-local MCP server through `ctx --repo-root <repo> mcp stdio`
- exposes the current CTX surface as native Claude Code skills
- keeps the host-selected Claude Code model in control

## Daily Usage

After bootstrap, open the repository in Claude Code and invoke skills such as:

```text
/ctx-doctor
/ctx-pack fix auth bug
/ctx-retrieve refresh token auth
/ctx-memory-list
```

The product expectation is:

- stay inside Claude Code for normal work
- use CTX through native Claude Code skills and MCP
- avoid wrapper-style public commands

## Verification

Bootstrap test:

```bash
ctx claude install
```

Expected result:

- `.mcp.json` contains `mcpServers.ctx`
- `.claude/skills/ctx-pack/SKILL.md` exists
- `.claude/skills/ctx-doctor/SKILL.md` exists
