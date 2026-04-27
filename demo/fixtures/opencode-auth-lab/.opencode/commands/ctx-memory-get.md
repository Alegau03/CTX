---
description: Memory | Read one CTX memory directive by key
---

Read a CTX memory directive from the current repository.

Argument:
- `$1`: directive key

Run `ctx memory get "$1"` and show the result.
If the directive is missing, say that clearly and suggest the matching CTX memory set action.
