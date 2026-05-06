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

VERSION="${CTX_VERSION:-$(grep -m1 '^version = ' Cargo.toml | sed -E 's/version = "([^"]+)"/\1/' || true)}"
if [[ -z "$VERSION" ]]; then
  echo "could not determine release version from Cargo.toml" >&2
  exit 1
fi

declare -a DEFAULT_CRATES=(
  "ctx-token"
  "ctx-config"
  "ctx-ast"
  "ctx-graph"
  "ctx-intake"
  "ctx-telemetry"
  "ctx-semantic"
  "ctx-hooks"
  "ctx-prune"
  "ctx-pack"
  "ctx-core"
  "ctx-mcp"
  "ctx-cli"
)

if [[ $# -gt 0 ]]; then
  CRATES=( "$@" )
else
  CRATES=( "${DEFAULT_CRATES[@]}" )
fi

wait_for_crate_version() {
  local crate="$1"
  local version="$2"
  local attempts="${CTX_CRATE_PUBLISH_WAIT_ATTEMPTS:-40}"
  local sleep_seconds="${CTX_CRATE_PUBLISH_WAIT_SECONDS:-3}"

  for ((attempt = 1; attempt <= attempts; attempt++)); do
    if curl -fsSL "https://crates.io/api/v1/crates/${crate}" | grep "\"num\":\"${version}\"" >/dev/null 2>&1; then
      echo "crates.io index now exposes ${crate} ${version}"
      return 0
    fi
    sleep "$sleep_seconds"
  done

  echo "timed out waiting for crates.io to expose ${crate} ${version}" >&2
  return 1
}

for i in "${!CRATES[@]}"; do
  crate_name="${CRATES[$i]}"
  echo "Publishing crate: ${crate_name}"
  "$CARGO_BIN" publish -p "$crate_name"

  if [[ "$i" -lt "$((${#CRATES[@]} - 1))" ]]; then
    wait_for_crate_version "$crate_name" "$VERSION"
  fi
done
