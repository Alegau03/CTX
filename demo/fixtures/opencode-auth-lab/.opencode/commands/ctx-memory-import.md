---
description: Memory | Import AGENTS-style guidance into CTX memory
---

Import markdown guidance into CTX graph memory.

Arguments:
- `$1`: markdown file path
- `$2`: optional scope, default `project`
- `$3`: optional source, default `markdown`
- `$4`: optional prefix

Run the matching `ctx memory import` command.
Then report how many directives were imported and from which file.
