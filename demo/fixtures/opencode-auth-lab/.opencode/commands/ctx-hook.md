---
description: Debug | Generate a CTX hook or pre-prompt payload
---

Generate a CTX hook payload for this task.

Arguments:
- `$ARGUMENTS`: the task query

!`'/Users/alessandrogautieri/Documents/coding/ctx/target/debug/ctx' --repo-root 'demo/fixtures/opencode-auth-lab' hook "$ARGUMENTS" --json`

Print `hook_prompt` first.
Then print a single compact metadata line with `packed_tokens`, `reduction_pct`, and `pack_path`.
Keep any usage note to one short sentence.
