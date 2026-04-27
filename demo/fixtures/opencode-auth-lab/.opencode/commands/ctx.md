---
description: Menu | Open the CTX command center and quickstart
---

Show a clean, terminal-friendly **CTX Command Center** for the current repository.

Start with the current repository status:
!`ctx doctor`

Then present this menu in English using short sections, aligned bullets, and clear next steps.

# CTX Command Center

## Recommended Start
- `/ctx-doctor` - check repo health and next step
- `/ctx-index` - build or refresh the graph
- `/ctx-memory-bootstrap` - import AGENTS-style project rules
- `/ctx-pack <task>` - build the smallest useful context pack

## Setup
- `/ctx-init`
- `/ctx-index`
- `/ctx-reindex`
- `/ctx-opencode-install`

## Context
- `/ctx-pack <task>`
- `/ctx-ask <task>`
- `/ctx-retrieve <query>`
- `/ctx-graph-query <query>`
- `/ctx-explain <task>`

## Memory
- `/ctx-memory-bootstrap`
- `/ctx-memory-search <topic>`
- `/ctx-memory-list`
- `/ctx-memory-get <key>`
- `/ctx-memory-set <key> <body>`
- `/ctx-memory-export <file>`

## Debug
- `/ctx-prune-logs <topic>`
- `/ctx-prune-diff <topic>`
- `/ctx-hook <task>`

## Benchmark
- `/ctx-benchmark-memory-ab ...`
- `/ctx-benchmark-memory-suite ...`
- `/ctx-stats`

## MCP
- `/ctx-mcp-stdio`
- `/ctx-mcp-serve`
- `/ctx-mcp-config-opencode`

End with:
1. the single best next command for the current repo state
2. one copy-paste example
3. a one-line explanation of why that command should come next
