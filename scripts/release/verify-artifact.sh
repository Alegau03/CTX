#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: scripts/release/verify-artifact.sh <artifact.tar.gz> [SHA256SUMS]" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
CHECKSUMS="${2:-$(dirname "$ARTIFACT")/SHA256SUMS}"
CHECKSUMS="$(cd "$(dirname "$CHECKSUMS")" && pwd)/$(basename "$CHECKSUMS")"

if [[ ! -f "$ARTIFACT" ]]; then
  echo "artifact not found: $ARTIFACT" >&2
  exit 1
fi

if [[ ! -f "$CHECKSUMS" ]]; then
  echo "checksum file not found: $CHECKSUMS" >&2
  exit 1
fi

expected_sha="$(awk 'NR==1 {print $1}' "$CHECKSUMS")"
if [[ -z "$expected_sha" ]]; then
  echo "checksum file is empty: $CHECKSUMS" >&2
  exit 1
fi

if command -v shasum >/dev/null 2>&1; then
  actual_sha="$(shasum -a 256 "$ARTIFACT" | awk '{print $1}')"
else
  actual_sha="$(sha256sum "$ARTIFACT" | awk '{print $1}')"
fi

if [[ "$expected_sha" != "$actual_sha" ]]; then
  echo "checksum mismatch for $ARTIFACT" >&2
  echo "expected: $expected_sha" >&2
  echo "actual:   $actual_sha" >&2
  exit 1
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

tar -xzf "$ARTIFACT" -C "$WORK_DIR"
PACKAGE_ROOT="$(find "$WORK_DIR" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
if [[ -z "$PACKAGE_ROOT" ]]; then
  echo "unable to locate unpacked package directory" >&2
  exit 1
fi

CTX_BIN="$PACKAGE_ROOT/ctx"
if [[ ! -x "$CTX_BIN" ]]; then
  echo "ctx binary missing or not executable: $CTX_BIN" >&2
  exit 1
fi

"$ROOT_DIR/scripts/release/install-smoke.sh" "$CTX_BIN"
"$ROOT_DIR/scripts/release/opencode-smoke.sh" "$CTX_BIN"
"$ROOT_DIR/scripts/demo/opencode-auth-lab-smoke.sh" "$CTX_BIN"
"$ROOT_DIR/scripts/demo/opencode-auth-lab-mcp-smoke.sh" "$CTX_BIN"
"$ROOT_DIR/scripts/demo/opencode-auth-lab-benchmark.sh" "$CTX_BIN"

echo "CTX release artifact verification passed: $ARTIFACT"
