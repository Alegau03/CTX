#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

VERSION="${CTX_VERSION:-$(grep -m1 '^version = ' Cargo.toml | sed -E 's/version = "([^"]+)"/\1/' || true)}"
VERSION="${VERSION:-0.1.0}"
TARGET="${CTX_TARGET:-$(rustc -vV | awk '/host:/ { print $2 }')}"
DIST_DIR="${CTX_DIST_DIR:-$ROOT_DIR/dist}"
PACKAGE_NAME="ctx-${VERSION}-${TARGET}"
PACKAGE_DIR="$DIST_DIR/$PACKAGE_NAME"

RUN_TESTS="${CTX_RELEASE_RUN_TESTS:-1}"

if [[ "$RUN_TESTS" != "0" ]]; then
  cargo fmt --all --check
  cargo test --workspace
fi

build_args=(build --release --locked --bin ctx)
if [[ -n "${CTX_TARGET:-}" ]]; then
  build_args+=(--target "$TARGET")
fi
cargo "${build_args[@]}"

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

echo "Release artifact ready: $DIST_DIR/$PACKAGE_NAME.tar.gz"
echo "Checksum file: $DIST_DIR/SHA256SUMS"
