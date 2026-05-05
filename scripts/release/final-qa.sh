#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
PYTHON_BIN="${PYTHON_BIN:-$(command -v python3 || command -v python || true)}"

CARGO_BIN="${CARGO_BIN:-$(command -v cargo || true)}"
if [[ -z "$CARGO_BIN" && -x "$HOME/.cargo/bin/cargo" ]]; then
  CARGO_BIN="$HOME/.cargo/bin/cargo"
fi
if [[ -z "$CARGO_BIN" ]]; then
  echo "cargo not found on PATH and \$HOME/.cargo/bin/cargo does not exist" >&2
  exit 1
fi
if [[ -z "$PYTHON_BIN" ]]; then
  echo "python3 or python is required for final QA" >&2
  exit 1
fi

"$CARGO_BIN" fmt --all --check
"$CARGO_BIN" test --workspace
"$CARGO_BIN" build --locked --bin ctx
DEBUG_CTX="$ROOT_DIR/target/debug/ctx"
scripts/release/install-smoke.sh "$DEBUG_CTX"
scripts/release/opencode-smoke.sh "$DEBUG_CTX"
scripts/demo/opencode-auth-lab-smoke.sh "$DEBUG_CTX"
scripts/demo/opencode-auth-lab-mcp-smoke.sh "$DEBUG_CTX"
scripts/demo/opencode-auth-lab-benchmark.sh "$DEBUG_CTX"
CTX_RELEASE_RUN_TESTS=0 CARGO_BIN="$CARGO_BIN" scripts/release/build.sh
MANIFEST_PATH="dist/release-manifest.json"
if [[ ! -f "$MANIFEST_PATH" ]]; then
  echo "release manifest not found: $MANIFEST_PATH" >&2
  exit 1
fi

ARCHIVE_PATHS=()
while IFS= read -r archive_path; do
  [[ -n "$archive_path" ]] && ARCHIVE_PATHS+=("$archive_path")
done < <(
  "$PYTHON_BIN" - "$MANIFEST_PATH" <<'PY'
import json
import pathlib
import sys

manifest = pathlib.Path(sys.argv[1])
data = json.loads(manifest.read_text())
for item in data.get("artifacts", []):
    archive = item.get("archive")
    if archive:
        print(manifest.parent / archive)
PY
)

if [[ "${#ARCHIVE_PATHS[@]}" -eq 0 ]]; then
  echo "no release archives listed in $MANIFEST_PATH" >&2
  exit 1
fi

for archive_path in "${ARCHIVE_PATHS[@]}"; do
  scripts/release/verify-artifact.sh "$archive_path" dist/SHA256SUMS
done

echo "CTX final QA passed: ${ARCHIVE_PATHS[*]}"
