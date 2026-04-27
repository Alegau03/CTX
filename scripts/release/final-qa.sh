#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

CARGO_BIN="${CARGO_BIN:-$(command -v cargo || true)}"
if [[ -z "$CARGO_BIN" && -x "$HOME/.cargo/bin/cargo" ]]; then
  CARGO_BIN="$HOME/.cargo/bin/cargo"
fi
if [[ -z "$CARGO_BIN" ]]; then
  echo "cargo not found on PATH and \$HOME/.cargo/bin/cargo does not exist" >&2
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
ARCHIVE_PATH="$(find dist -maxdepth 1 -name 'ctx-*.tar.gz' | sort | tail -n 1)"
if [[ -z "$ARCHIVE_PATH" ]]; then
  echo "release archive not found in dist/" >&2
  exit 1
fi
scripts/release/verify-artifact.sh "$ARCHIVE_PATH" dist/SHA256SUMS

echo "CTX final QA passed: $ARCHIVE_PATH"
