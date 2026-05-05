#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOURCE_FIXTURE="$ROOT_DIR/demo/fixtures/opencode-auth-lab"
CTX_BIN="${1:-$ROOT_DIR/target/debug/ctx}"
PYTHON_BIN="${PYTHON_BIN:-$(command -v python3 || command -v python || true)}"

if [[ -z "$PYTHON_BIN" ]]; then
  echo "python3 or python is required for MCP smoke" >&2
  exit 1
fi

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

CTX_BIN="$CTX_BIN" FIXTURE="$FIXTURE" "$PYTHON_BIN" - <<'PY'
import json
import os
import re
import subprocess
import sys
import threading
import time
from queue import Empty, Queue

cmd = [
    os.environ["CTX_BIN"],
    "--repo-root",
    os.environ["FIXTURE"],
    "mcp",
    "stdio",
]
p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
stdout_queue = Queue()
stderr_queue = Queue()


def pump(stream, queue):
    while True:
        chunk = stream.read(1)
        if not chunk:
            break
        queue.put(chunk)


threading.Thread(target=pump, args=(p.stdout, stdout_queue), daemon=True).start()
threading.Thread(target=pump, args=(p.stderr, stderr_queue), daemon=True).start()


def send(obj):
    body = json.dumps(obj).encode()
    header = f"Content-Length: {len(body)}\r\n\r\n".encode()
    p.stdin.write(header + body)
    p.stdin.flush()


def recv(timeout=3, required=True):
    buf = b""
    deadline = time.time() + timeout
    while b"\r\n\r\n" not in buf:
        remaining = deadline - time.time()
        if remaining <= 0:
            if required:
                raise TimeoutError("timed out while waiting for MCP headers")
            return None
        try:
            buf += stdout_queue.get(timeout=min(0.1, remaining))
        except Empty:
            if p.poll() is not None and not required:
                return None
    head, rest = buf.split(b"\r\n\r\n", 1)
    match = re.search(br"Content-Length:\s*(\d+)", head)
    if not match:
        raise RuntimeError(f"missing Content-Length header: {head!r}")
    length = int(match.group(1))
    while len(rest) < length:
        remaining = deadline - time.time()
        if remaining <= 0:
            raise TimeoutError("timed out while reading MCP body")
        try:
            rest += stdout_queue.get(timeout=min(0.1, remaining))
        except Empty:
            if p.poll() is not None:
                raise RuntimeError("MCP process exited before completing the response body")
    return json.loads(rest[:length].decode())


send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
initialize = recv()
assert initialize["result"]["serverInfo"]["name"] == "ctx-mcp", initialize

send({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
notification = recv(timeout=0.5, required=False)
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

stderr = b""
while True:
    try:
        stderr += stderr_queue.get_nowait()
    except Empty:
        break
if stderr.strip():
    print(stderr.decode(), file=sys.stderr)
p.kill()
p.wait(timeout=1)
PY

echo "CTX demo MCP smoke passed: $FIXTURE"
