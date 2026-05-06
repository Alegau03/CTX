#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: scripts/release/verify-artifact.sh <artifact.tar.gz|artifact.zip> [SHA256SUMS]" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
CHECKSUMS="${2:-$(dirname "$ARTIFACT")/SHA256SUMS}"
CHECKSUMS="$(cd "$(dirname "$CHECKSUMS")" && pwd)/$(basename "$CHECKSUMS")"
PYTHON_BIN="${PYTHON_BIN:-$(command -v python3 || command -v python || true)}"
VERIFY_MODE="${CTX_VERIFY_RUN_SMOKE:-auto}"

if [[ ! -f "$ARTIFACT" ]]; then
  echo "artifact not found: $ARTIFACT" >&2
  exit 1
fi

if [[ ! -f "$CHECKSUMS" ]]; then
  echo "checksum file not found: $CHECKSUMS" >&2
  exit 1
fi

artifact_name="$(basename "$ARTIFACT")"
expected_sha="$(
  awk -v artifact_name="$artifact_name" '
    {
      candidate = $NF
      sub(/^.*\//, "", candidate)
      if (candidate == artifact_name) {
        print $1
        exit
      }
    }
  ' "$CHECKSUMS"
)"
if [[ -z "$expected_sha" ]]; then
  echo "checksum for $artifact_name not found in $CHECKSUMS" >&2
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

if [[ "$ARTIFACT" == *.zip ]]; then
  if [[ -z "$PYTHON_BIN" ]]; then
    echo "python3 or python is required to unpack zip artifacts" >&2
    exit 1
  fi
  "$PYTHON_BIN" - "$ARTIFACT" "$WORK_DIR" <<'PY'
import pathlib
import sys
import zipfile

archive = pathlib.Path(sys.argv[1])
dest = pathlib.Path(sys.argv[2])
with zipfile.ZipFile(archive) as zf:
    zf.extractall(dest)
PY
else
  tar -xzf "$ARTIFACT" -C "$WORK_DIR"
fi

PACKAGE_ROOT="$(find "$WORK_DIR" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
if [[ -z "$PACKAGE_ROOT" ]]; then
  echo "unable to locate unpacked package directory" >&2
  exit 1
fi

CTX_BIN="$PACKAGE_ROOT/ctx"
if [[ -f "$PACKAGE_ROOT/ctx.exe" ]]; then
  CTX_BIN="$PACKAGE_ROOT/ctx.exe"
fi

if [[ ! -f "$CTX_BIN" ]]; then
  echo "ctx binary missing or not executable: $CTX_BIN" >&2
  exit 1
fi

archive_stem="$artifact_name"
archive_stem="${archive_stem%.tar.gz}"
archive_stem="${archive_stem%.zip}"
host_target=""
rustc_bin="$(command -v rustc || true)"
if [[ -z "$rustc_bin" && -x "$HOME/.cargo/bin/rustc" ]]; then
  rustc_bin="$HOME/.cargo/bin/rustc"
fi
if [[ -n "$rustc_bin" ]]; then
  host_target="$("$rustc_bin" -vV | awk '/host:/ { print $2 }')"
fi

IFS='-' read -r _pkg_name _pkg_version target_part1 target_part2 target_part3 target_part4 target_part5 <<<"$archive_stem"
archive_target="$target_part1"
for part in "$target_part2" "$target_part3" "$target_part4" "$target_part5"; do
  [[ -n "$part" ]] && archive_target="${archive_target}-$part"
done

should_run_smoke=0
case "$VERIFY_MODE" in
  1|true|always)
    should_run_smoke=1
    ;;
  0|false|never)
    should_run_smoke=0
    ;;
  auto)
    if [[ -n "$host_target" && -n "$archive_target" && "$host_target" == "$archive_target" ]]; then
      should_run_smoke=1
    fi
    ;;
  *)
    echo "invalid CTX_VERIFY_RUN_SMOKE value: $VERIFY_MODE" >&2
    exit 1
    ;;
esac

if [[ "$should_run_smoke" == "1" ]]; then
  "$ROOT_DIR/scripts/release/install-smoke.sh" "$CTX_BIN"
  "$ROOT_DIR/scripts/release/opencode-smoke.sh" "$CTX_BIN"
  if [[ "$archive_target" == *windows* || "$host_target" == *windows* ]]; then
    # The Windows packaging gate should prove the shipped binary boots and installs
    # correctly. The richer demo fixture smoke already runs in local QA and on the
    # Unix runners, but remains brittle under the Git-for-Windows host shell.
    echo "Skipping demo fixture smoke on Windows host: packaging and OpenCode install already verified"
  else
    "$ROOT_DIR/scripts/demo/opencode-auth-lab-smoke.sh" "$CTX_BIN"
    "$ROOT_DIR/scripts/demo/opencode-auth-lab-mcp-smoke.sh" "$CTX_BIN"
    "$ROOT_DIR/scripts/demo/opencode-auth-lab-benchmark.sh" "$CTX_BIN"
  fi
else
  echo "Skipping runtime smoke for non-host artifact: $artifact_name"
fi

echo "CTX release artifact verification passed: $ARTIFACT"
