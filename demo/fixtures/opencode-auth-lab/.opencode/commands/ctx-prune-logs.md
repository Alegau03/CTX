---
description: Debug | Prune noisy logs and keep root-cause signal
---

Prune noisy logs with CTX.

Arguments:
- `$ARGUMENTS`: the shell command that produces logs

Run the provided shell command in the current repository and pipe its combined output into `ctx prune logs`.
Then show the pruned output and explain the highest-signal root cause lines.
