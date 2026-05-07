# Security and Privacy

CTX is local-first by default. Project data stays on the developer machine unless a future feature explicitly opts into remote behavior.

## Defaults

```toml
local_only = true
remote_upload_enabled = false
anonymous_telemetry_enabled = false
local_stats_enabled = true
audit_include_exclude = true
exclude_sensitive_files = true
```

Local stats are written to `.ctx/stats/latest.json`; they are not remote telemetry.

## Local Storage

| Artifact | Location |
|---|---|
| Config | `.ctx/config.toml` |
| Graph | `.ctx/graph.db` |
| Packs | `.ctx/packs/` |
| Stats | `.ctx/stats/latest.json` |
| Audit | `.ctx/audit.log` |

## Sensitive File Guardrails

When `security.exclude_sensitive_files = true`, CTX blocks attachments and skips indexed paths matching sensitive patterns.

Default patterns:

```toml
sensitive_patterns = [".env", "id_rsa", ".pem", ".key", "credentials", "secret"]
```

Default file ignores skip common local artifacts and generated noise:

```toml
ignored_files = ["*.db", "*.sqlite", "*.sqlite3", "*.pyc", "*.pyo", "*.pem", "*.log", ".env", "*.env", ".coverage", ".coverage.*", ".DS_Store", "Thumbs.db", "package-lock.json"]
```

Example:

```bash
ctx pack "fix auth" --attach .env
```

Expected behavior:

```text
attachment .env matches sensitive file patterns and was blocked
```

Default directory ignores also skip common build, cache, virtualenv, editor, and worktree folders such as `.venv`, `__pycache__`, `.pytest_cache`, `.vscode`, and `.claude`. Directory ignores support glob-style component patterns such as `*.egg-info`, and file ignores support basename or path globs such as `package-lock.json` and `docs/*.md`. These ignore rules apply to indexing and `ctx read`; `ctx pack --attach` still allows diagnostic files like `.log` attachments and only blocks sensitive patterns.

## WSL Performance Tip

On WSL, lots of small filesystem writes can make graph indexing noticeably slower than on native Linux paths. If you want a much faster ephemeral graph store, point the graph database at the RAM-backed `/dev/shm` filesystem:

```toml
[graph]
store = "/dev/shm/my-project-graph.db"
```

This is often much faster on WSL, but the graph DB is volatile and will be lost on reboot.

## MCP Boundary

OpenCode integration uses local stdio MCP:

```bash
ctx --repo-root /path/to/project mcp stdio
```

The HTTP JSON-RPC server is for localhost debugging and binds to `127.0.0.1`. Do not expose it to public networks.

## What CTX Protects Against

- accidental inclusion of common secret files in packs
- accidental indexing of secret-looking paths
- silent remote telemetry by default
- silent privacy include/exclude decisions
- token waste from build/dependency/cache directories

## What CTX Does Not Protect Against

- secrets embedded in normal source files that do not match configured patterns
- malicious local processes running as the same user
- host agent CLIs that independently upload prompts/files after CTX returns context
- public network exposure if a user manually tunnels the localhost MCP server

## Verification

```bash
cargo test -p ctx-config security_
cargo test -p ctx-telemetry privacy
cargo test -p ctx-core sensitive
```
