# Security and Privacy

## Defaults

- Local-first runtime. No mandatory remote service.
- MCP server binds to `127.0.0.1` only.
- Runtime artifacts remain under `.ctx/`.
- Sensitive attachment filtering is enabled by default via `security.sensitive_patterns`.

## Sensitive File Guardrail

When `security.exclude_sensitive_files = true`, `ctx pack --attach` blocks paths matching configured sensitive patterns (for example `.env`, `.pem`, `.key`, `id_rsa`, `credentials`, `secret`).

## Auditability

`run_pack` appends audit entries to `.ctx/audit.log` with query and reduction metrics.

## Telemetry

Current telemetry implementation writes local snapshots only (`.ctx/stats/latest.json`).
No remote upload path is implemented.
