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
VERSION="${VERSION:-0.1.0}"
TARGET="${CTX_TARGET:-$("$CARGO_BIN" -vV | awk '/host:/ { print $2 }')}"
DIST_DIR="${CTX_DIST_DIR:-$ROOT_DIR/dist}"
PACKAGE_NAME="ctx-${VERSION}-${TARGET}"
PACKAGE_DIR="$DIST_DIR/$PACKAGE_NAME"
ARCHIVE_PATH="$DIST_DIR/$PACKAGE_NAME.tar.gz"
MANIFEST_PATH="$DIST_DIR/release-manifest.json"

RUN_TESTS="${CTX_RELEASE_RUN_TESTS:-1}"

if [[ "$RUN_TESTS" != "0" ]]; then
  "$CARGO_BIN" fmt --all --check
  "$CARGO_BIN" test --workspace
fi

build_args=(build --release --locked --bin ctx)
if [[ -n "${CTX_TARGET:-}" ]]; then
  build_args+=(--target "$TARGET")
fi
"$CARGO_BIN" "${build_args[@]}"

BIN_DIR="$ROOT_DIR/target/release"
if [[ -n "${CTX_TARGET:-}" ]]; then
  BIN_DIR="$ROOT_DIR/target/$TARGET/release"
fi

rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR"
cp "$BIN_DIR/ctx" "$PACKAGE_DIR/ctx"
cp README.md LICENSE "$PACKAGE_DIR/" 2>/dev/null || true
cp docs/install.md "$PACKAGE_DIR/INSTALL.md"

"$ROOT_DIR/scripts/release/install-smoke.sh" "$PACKAGE_DIR/ctx"
"$ROOT_DIR/scripts/release/opencode-smoke.sh" "$PACKAGE_DIR/ctx"
"$ROOT_DIR/scripts/demo/opencode-auth-lab-smoke.sh" "$PACKAGE_DIR/ctx"
"$ROOT_DIR/scripts/demo/opencode-auth-lab-mcp-smoke.sh" "$PACKAGE_DIR/ctx"
"$ROOT_DIR/scripts/demo/opencode-auth-lab-benchmark.sh" "$PACKAGE_DIR/ctx"

mkdir -p "$DIST_DIR"
(
  cd "$DIST_DIR"
  tar -czf "$PACKAGE_NAME.tar.gz" "$PACKAGE_NAME"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$PACKAGE_NAME.tar.gz" > SHA256SUMS
  else
    sha256sum "$PACKAGE_NAME.tar.gz" > SHA256SUMS
  fi
)

if command -v shasum >/dev/null 2>&1; then
  SHA256_VALUE="$(shasum -a 256 "$ARCHIVE_PATH" | awk '{print $1}')"
else
  SHA256_VALUE="$(sha256sum "$ARCHIVE_PATH" | awk '{print $1}')"
fi

cat > "$MANIFEST_PATH" <<EOF
{
  "version": "$VERSION",
  "target": "$TARGET",
  "package_name": "$PACKAGE_NAME",
  "archive": "$(basename "$ARCHIVE_PATH")",
  "sha256": "$SHA256_VALUE",
  "checksum_file": "SHA256SUMS",
  "install_doc": "INSTALL.md",
  "readme": "README.md",
  "demo_fixture": "demo/fixtures/opencode-auth-lab",
  "benchmark_report_markdown": "demo/fixtures/opencode-auth-lab/benchmarks/report.md",
  "benchmark_report_json": "demo/fixtures/opencode-auth-lab/benchmarks/report.json"
}
EOF

"$ROOT_DIR/scripts/release/verify-artifact.sh" "$ARCHIVE_PATH" "$DIST_DIR/SHA256SUMS"

echo "Release artifact ready: $ARCHIVE_PATH"
echo "Checksum file: $DIST_DIR/SHA256SUMS"
echo "Release manifest: $MANIFEST_PATH"
