#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOURCE_FIXTURE="$ROOT_DIR/demo/fixtures/opencode-auth-lab"
CTX_BIN="${1:-$ROOT_DIR/target/debug/ctx}"

if [[ -n "${CTX_DEMO_FIXTURE:-}" ]]; then
  FIXTURE="$CTX_DEMO_FIXTURE"
else
  TMP_FIXTURE_ROOT="$(mktemp -d)"
  trap 'rm -rf "$TMP_FIXTURE_ROOT"' EXIT
  FIXTURE="$TMP_FIXTURE_ROOT/opencode-auth-lab"
  cp -R "$SOURCE_FIXTURE" "$FIXTURE"
fi

rm -rf "$FIXTURE/.ctx"
"$CTX_BIN" --repo-root "$FIXTURE" init >/dev/null
"$CTX_BIN" --repo-root "$FIXTURE" index >/dev/null

CTX_BIN="$CTX_BIN" FIXTURE="$FIXTURE" python3 - <<'PY'
import json
import os
import re
import select
import subprocess
import sys
import time

cmd = [
    os.environ["CTX_BIN"],
    "--repo-root",
    os.environ["FIXTURE"],
    "mcp",
    "stdio",
]
p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)


def send(obj):
    body = json.dumps(obj).encode()
    header = f"Content-Length: {len(body)}\r\n\r\n".encode()
    p.stdin.write(header + body)
    p.stdin.flush()


def recv(timeout=3):
    fd = p.stdout.fileno()
    buf = b""
    start = time.time()
    while b"\r\n\r\n" not in buf:
        if time.time() - start > timeout:
            return None
        ready, _, _ = select.select([fd], [], [], 0.1)
        if ready:
            buf += os.read(fd, 4096)
    head, rest = buf.split(b"\r\n\r\n", 1)
    match = re.search(br"Content-Length:\s*(\d+)", head)
    if not match:
        raise RuntimeError(f"missing Content-Length header: {head!r}")
    length = int(match.group(1))
    while len(rest) < length:
        if time.time() - start > timeout:
            raise TimeoutError("timed out while reading MCP body")
        ready, _, _ = select.select([fd], [], [], 0.1)
        if ready:
            rest += os.read(fd, 4096)
    return json.loads(rest[:length].decode())


send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
initialize = recv()
assert initialize["result"]["serverInfo"]["name"] == "ctx-mcp", initialize

send({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
notification = recv(timeout=0.5)
assert notification is None, notification

send({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
tools = recv()
tool_names = {tool["name"] for tool in tools["result"]["tools"]}
assert "memory_bootstrap_markdown" in tool_names, tools
assert "memory_search" in tool_names, tools
assert "get_relevant_context" in tool_names, tools

send({
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {"name": "memory_bootstrap_markdown", "arguments": {}},
})
bootstrap = recv()
assert "imported_files" in json.dumps(bootstrap), bootstrap

send({
    "jsonrpc": "2.0",
    "id": 4,
    "method": "tools/call",
    "params": {
        "name": "memory_search",
        "arguments": {"query": "auth root cause", "scope": "project", "limit": 10},
    },
})
search = recv()
assert "root cause" in json.dumps(search), search

send({
    "jsonrpc": "2.0",
    "id": 5,
    "method": "tools/call",
    "params": {
        "name": "get_relevant_context",
        "arguments": {"query": "fix refresh token rotation", "budget": 160},
    },
})
pack = recv(timeout=5)
assert "compact_context" in json.dumps(pack), pack

stderr = ""
if p.stderr is not None:
    ready, _, _ = select.select([p.stderr.fileno()], [], [], 0)
    if ready:
        stderr = os.read(p.stderr.fileno(), 4096).decode()
if stderr.strip():
    print(stderr, file=sys.stderr)
p.kill()
p.wait(timeout=1)
PY

echo "CTX demo MCP smoke passed: $FIXTURE"
