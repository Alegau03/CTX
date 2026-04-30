---
description: Setup | Check CTX repo health and next steps
---

Current CTX doctor report:

!`'/Users/alessandrogautieri/Documents/coding/ctx/target/debug/ctx' --repo-root 'demo/fixtures/opencode-auth-lab' doctor`

Interpret the report deterministically:
- if `ready: true`, say CTX is ready; treat `next:` as the recommended workflow step, not missing setup
- if `ready: false`, say CTX is not ready and print the exact `next:` command
- print the exact `next:` command verbatim
- do not inspect files manually
- do not contradict the `ready:` line
