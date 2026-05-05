# CTX Demo Script

This is the recording sequence for the public OpenCode demo.

## Goal

Show CTX from install to real usage:

- install `ctx`
- enable CTX inside a project
- open OpenCode
- show the live right-sidebar `CTX Dashboard`
- use `/ctx-*` commands instead of dumping large markdown files or noisy logs into the prompt

## Recording Flow

### 1. Install CTX

Use the release archive path from [README.md](../README.md#install-ctx), then verify:

```bash
ctx help
ctx doctor
```

### 2. Move Into The Demo Project

```bash
cd /path/to/demo-project
ctx init
ctx index
ctx opencode install --profile full
opencode
```

Optional lean install for a shorter demo surface:

```bash
ctx opencode install --profile core
```

### 3. Start Inside OpenCode

Run:

```text
/ctx
/ctx-doctor
/ctx-memory-bootstrap
/ctx-memory-search auth
/ctx-plan fix auth refresh regression
/ctx-retrieve refresh token auth failure
/ctx-read src/auth.ts outline
/ctx-pack fix auth refresh regression
/ctx-compare fix auth refresh regression
/ctx-dashboard
/ctx-gain
/ctx-run npm run test:auth
/ctx-prune-logs npm run test:auth
/ctx-stats
```

Sidebar shots to capture during the demo:

- the right-sidebar `CTX Dashboard` immediately after `ctx opencode install --profile full`
- the same sidebar after `/ctx-read`, `/ctx-pack`, and `/ctx-run` so savings and cache numbers visibly change

Optional Toolbooks segment:

```text
/ctx-toolbook-import glab docs/glab.md
/ctx-toolbook-search glab "merge request create"
/ctx-toolbook-pack glab "create merge request for auth fix"
```

Optional Learning segment:

```text
/ctx-learn auth.refresh_regression "When auth refresh fails, check token rotation and stale session flags first."
/ctx-memory-search refresh regression
```

## Numbers To Mention

Fixture benchmark:

- token reduction: `56.72%`
- query coverage: `markdown=1.00`, `graph=1.00`
- quality wins: `markdown=0`, `graph=1`, `ties=0`

External benchmark snapshot:

- token reduction: `72.62%`
- query coverage: `markdown=1.00`, `graph=0.89`
- success rate: `markdown=0.50`, `graph=1.00`

## Close

Point viewers to:

- [README.md](../README.md)
- [docs/commands.md](commands.md)
- [guide.md](../guide.md)
