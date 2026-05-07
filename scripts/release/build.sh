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
VERSION="${VERSION:-0.2.5}"
HOST_TARGET="$("$CARGO_BIN" -vV | awk '/host:/ { print $2 }')"
DIST_DIR="${CTX_DIST_DIR:-$ROOT_DIR/dist}"
MANIFEST_PATH="$DIST_DIR/release-manifest.json"

RUN_TESTS="${CTX_RELEASE_RUN_TESTS:-1}"

PYTHON_BIN="${PYTHON_BIN:-$(command -v python3 || command -v python || true)}"
if [[ -z "$PYTHON_BIN" ]]; then
  echo "python3 or python is required for release packaging" >&2
  exit 1
fi

declare -a TARGETS=()
if [[ -n "${CTX_TARGETS:-}" ]]; then
  while IFS= read -r target; do
    [[ -n "$target" ]] && TARGETS+=("$target")
  done < <(printf '%s\n' "$CTX_TARGETS" | tr ', ' '\n\n' | sed '/^$/d')
elif [[ -n "${CTX_TARGET:-}" ]]; then
  TARGETS=("$CTX_TARGET")
else
  TARGETS=("$HOST_TARGET")
fi

checksum_file="$DIST_DIR/SHA256SUMS"
mkdir -p "$DIST_DIR"
: > "$checksum_file"

declare -a MANIFEST_ITEMS=()
declare -a BUILT_ARCHIVES=()

compute_sha256() {
  local path="$1"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
  else
    sha256sum "$path" | awk '{print $1}'
  fi
}

create_zip_archive() {
  local source_dir="$1"
  local archive_path="$2"
  "$PYTHON_BIN" - "$source_dir" "$archive_path" <<'PY'
import pathlib
import sys
import zipfile

source = pathlib.Path(sys.argv[1])
archive = pathlib.Path(sys.argv[2])
with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as zf:
    for path in source.rglob("*"):
        if path.is_file():
            zf.write(path, path.relative_to(source.parent))
PY
}

if [[ "$RUN_TESTS" != "0" ]]; then
  "$CARGO_BIN" fmt --all --check
  "$CARGO_BIN" test --workspace
fi

for TARGET in "${TARGETS[@]}"; do
  PACKAGE_NAME="ctx-${VERSION}-${TARGET}"
  PACKAGE_DIR="$DIST_DIR/$PACKAGE_NAME"
  BIN_DIR="$ROOT_DIR/target/$TARGET/release"
  BIN_NAME="ctx"
  ARCHIVE_EXT="tar.gz"

  if [[ "$TARGET" == *windows* ]]; then
    BIN_NAME="ctx.exe"
    ARCHIVE_EXT="zip"
  fi

  build_args=(build --release --locked --bin ctx --target "$TARGET")
  "$CARGO_BIN" "${build_args[@]}"

  rm -rf "$PACKAGE_DIR"
  mkdir -p "$PACKAGE_DIR"
  cp "$BIN_DIR/$BIN_NAME" "$PACKAGE_DIR/$BIN_NAME"
  cp README.md LICENSE "$PACKAGE_DIR/" 2>/dev/null || true
  cp docs/install.md "$PACKAGE_DIR/INSTALL.md"

  ARCHIVE_PATH="$DIST_DIR/$PACKAGE_NAME.$ARCHIVE_EXT"
  rm -f "$ARCHIVE_PATH"
  if [[ "$ARCHIVE_EXT" == "zip" ]]; then
    create_zip_archive "$PACKAGE_DIR" "$ARCHIVE_PATH"
  else
    (
      cd "$DIST_DIR"
      tar -czf "$PACKAGE_NAME.tar.gz" "$PACKAGE_NAME"
    )
  fi

  SHA256_VALUE="$(compute_sha256 "$ARCHIVE_PATH")"
  printf '%s  %s\n' "$SHA256_VALUE" "$(basename "$ARCHIVE_PATH")" >> "$checksum_file"

  CTX_VERIFY_RUN_SMOKE=auto "$ROOT_DIR/scripts/release/verify-artifact.sh" "$ARCHIVE_PATH" "$checksum_file"

  MANIFEST_ITEMS+=("{\"target\":\"$TARGET\",\"package_name\":\"$PACKAGE_NAME\",\"archive\":\"$(basename "$ARCHIVE_PATH")\",\"archive_format\":\"$ARCHIVE_EXT\",\"binary\":\"$BIN_NAME\",\"sha256\":\"$SHA256_VALUE\",\"checksum_file\":\"SHA256SUMS\",\"install_doc\":\"INSTALL.md\",\"readme\":\"README.md\"}")
  BUILT_ARCHIVES+=("$ARCHIVE_PATH")
done

{
  printf '{\n'
  printf '  "version": "%s",\n' "$VERSION"
  printf '  "host_target": "%s",\n' "$HOST_TARGET"
  printf '  "artifacts": [\n'
  if [[ "${#MANIFEST_ITEMS[@]}" -gt 0 ]]; then
    for i in "${!MANIFEST_ITEMS[@]}"; do
      suffix=","
      if [[ "$i" -eq "$((${#MANIFEST_ITEMS[@]} - 1))" ]]; then
        suffix=""
      fi
      printf '    %s%s\n' "${MANIFEST_ITEMS[$i]}" "$suffix"
    done
  fi
  printf '  ],\n'
  printf '  "demo_fixture": "demo/fixtures/opencode-auth-lab",\n'
  printf '  "benchmark_report_markdown": "demo/fixtures/opencode-auth-lab/benchmarks/report.md",\n'
  printf '  "benchmark_report_json": "demo/fixtures/opencode-auth-lab/benchmarks/report.json"\n'
  printf '}\n'
} > "$MANIFEST_PATH"

for archive in "${BUILT_ARCHIVES[@]}"; do
  echo "Release artifact ready: $archive"
done
echo "Checksum file: $checksum_file"
echo "Release manifest: $MANIFEST_PATH"
