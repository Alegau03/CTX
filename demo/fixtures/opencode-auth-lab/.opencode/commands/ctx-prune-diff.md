---
description: Debug | Prune the current git diff for a task
---

Prune the current git diff with CTX.

Arguments:
- `$ARGUMENTS`: the query to use for diff pruning

Run `git diff | ctx prune diff --query "$ARGUMENTS"` in the current repository.
Then show the compact diff and explain why the remaining hunks matter.
