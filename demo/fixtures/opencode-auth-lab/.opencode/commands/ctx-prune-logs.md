---
description: Debug | Prune noisy logs and keep root-cause signal
---

Prune noisy logs with CTX.

Arguments:
- `$ARGUMENTS`: the exact shell command that produces logs

`$ARGUMENTS` must be a real shell command such as `npm test -- --grep "refresh"` or `pytest -k auth -q`.
Do not treat `$ARGUMENTS` as a topic, label, or search phrase.
If `$ARGUMENTS` does not look runnable, stop and tell the user to provide the exact shell command to execute.

Run the provided shell command in the current repository and pipe its combined output into `ctx prune logs --max-lines 50`.
Then show the pruned output and explain the highest-signal root cause lines.
