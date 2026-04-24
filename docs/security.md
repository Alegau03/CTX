# Security and Privacy

CTX is designed as a local-first context runtime for coding agents. The default posture is conservative: project data stays on the developer machine, sensitive files are excluded before packing/indexing, and privacy decisions are written to a local audit log.

## Default Posture

- `security.local_only = true`
- `security.remote_upload_enabled = false`
- `security.anonymous_telemetry_enabled = false`
- `security.local_stats_enabled = true`
- `security.audit_include_exclude = true`
- `security.exclude_sensitive_files = true`

Local stats are not remote telemetry. They are written to `.ctx/stats/latest.json` so users can inspect token reduction, latency, adapter status and fallback behavior after a local run.

## Local Storage

CTX runtime artifacts live under `.ctx/`:

- `.ctx/config.toml`: project configuration
- `.ctx/graph.db`: local SQLite knowledge graph
- `.ctx/packs/`: generated compact context artifacts
- `.ctx/stats/latest.json`: last local run stats
- `.ctx/audit.log`: local audit log

CTX does not implement a remote upload path by default. If a future remote feature is added, it must be opt-in and must fail validation when `security.local_only = true`.

## Sensitive File Guardrails

When `security.exclude_sensitive_files = true`, CTX blocks attachments and skips indexed code paths that match `security.sensitive_patterns`.

Default sensitive patterns:

```toml
sensitive_patterns = [".env", "id_rsa", ".pem", ".key", "credentials", "secret"]
```

Example:

```bash
ctx pack "fix auth" --attach .env
```

Expected behavior:

```text
attachment .env matches sensitive file patterns and was blocked
```

An audit event is appended to `.ctx/audit.log`:

```json
{"kind":"privacy_decision","decision":"excluded","path":".env","reason":"sensitive_pattern","local_only":true,"remote_upload_enabled":false,"message":"blocked sensitive attachment before packing"}
```

## Ignore Rules

CTX skips configured noisy/runtime directories during indexing.

Default ignored directories:

```toml
ignored_dirs = [".git", ".ctx", "target", "node_modules", "build", "dist", "artifacts", ".next", ".cache", "coverage"]
```

This keeps build outputs, dependency folders, local CTX artifacts and caches out of the graph by default.

## Auditability

CTX writes local audit information to `.ctx/audit.log`.

Examples of audited events:

- blocked sensitive attachment during `ctx pack`
- skipped sensitive source path during `ctx index`
- context pack summary with included/excluded section counts
- adapter invocation metadata for `ctx codex`, `ctx claude`, `ctx opencode` and `ctx wrap`

The audit log is intentionally local and append-only from CTX's point of view. Users can delete it manually if they want to reset local history.

## MCP Trust Boundary

The HTTP MCP-compatible server binds to `127.0.0.1`. The stdio MCP mode runs as a local process launched by the client.

Recommended usage:

```bash
ctx --repo-root /path/to/project mcp stdio
```

Trust assumptions:

- The local user controls the repository and CTX process.
- MCP clients that can launch `ctx mcp stdio` can request context from that repository.
- CTX does not authenticate local stdio clients; access control is delegated to the local machine/user account.
- Do not expose the HTTP server to public networks.

## What CTX Protects Against

- Accidental inclusion of common secret files in context packs.
- Accidental indexing of secret-looking code paths.
- Silent remote telemetry by default.
- Silent privacy-related include/exclude decisions.
- Token waste from dependency/build/cache directories.

## What CTX Does Not Protect Against

- Secrets embedded inside otherwise normal source files that do not match configured patterns.
- Malicious local processes running as the same user.
- Agent CLIs that independently upload prompts or files after CTX hands them context.
- Public network exposure if a user manually proxies or tunnels the local MCP HTTP port.

## Verification Commands

Run privacy/config tests:

```bash
cargo test -p ctx-config security_
```

Run audit tests:

```bash
cargo test -p ctx-telemetry privacy
```

Run core sensitive-file behavior tests:

```bash
cargo test -p ctx-core sensitive
```

Manual smoke test:

```bash
tmpdir="$(mktemp -d)"
cd "$tmpdir"
ctx init
printf 'API_KEY=secret\n' > .env
ctx pack "fix auth" --attach .env
cat .ctx/audit.log
```

Expected result:

- `ctx pack` exits with a sensitive attachment error.
- `.ctx/audit.log` contains a `privacy_decision` event with `decision = "excluded"` and `reason = "sensitive_pattern"`.
