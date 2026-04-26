# CTX Inside Codex

## Goal

Make CTX usable inside Codex through native project-local assets, without reintroducing wrapper-style public commands.

## Current Implementation

Bootstrap:

```bash
ctx codex install
```

Generated assets:

- `.codex/config.toml`
- `.agents/skills/ctx-*/SKILL.md`

What the bootstrap does:

- registers CTX as a repo-local MCP server through `ctx --repo-root <repo> mcp stdio`
- exposes the current CTX surface as Codex skills
- keeps the host-selected Codex model in control

## Daily Usage

After bootstrap, open the repository in Codex and invoke skills such as:

```text
$ctx-doctor
$ctx-pack
$ctx-retrieve
$ctx-memory-list
```

The product expectation is:

- stay inside Codex for normal work
- use CTX through native Codex skills and MCP
- avoid wrapper-style public commands

## Current Limitation

Codex is currently skill-native rather than slash-command-native for this integration. That is still a host-native path, but the explicit surface is based on skills under `.agents/skills/` instead of OpenCode-style command markdown files.

## Verification

Bootstrap test:

```bash
ctx codex install
```

Expected result:

- `.codex/config.toml` contains `[mcp_servers.ctx]`
- `.agents/skills/ctx-pack/SKILL.md` exists
- `.agents/skills/ctx-doctor/SKILL.md` exists
