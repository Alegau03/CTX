#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: scripts/release/publish-github-release.sh <version> [dist_dir]" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="$1"
DIST_DIR="${2:-$ROOT_DIR/dist}"
TAG="v${VERSION}"

GH_BIN="${GH_BIN:-$(command -v gh || true)}"
if [[ -z "$GH_BIN" && -x "/opt/homebrew/bin/gh" ]]; then
  GH_BIN="/opt/homebrew/bin/gh"
fi
if [[ -z "$GH_BIN" ]]; then
  echo "gh not found on PATH and /opt/homebrew/bin/gh does not exist" >&2
  exit 1
fi

if [[ ! -d "$DIST_DIR" ]]; then
  echo "dist directory not found: $DIST_DIR" >&2
  exit 1
fi

MANIFEST="$DIST_DIR/release-manifest.json"
SUMS="$DIST_DIR/SHA256SUMS"
if [[ ! -f "$MANIFEST" ]]; then
  echo "release manifest not found: $MANIFEST" >&2
  exit 1
fi
if [[ ! -f "$SUMS" ]]; then
  echo "checksum file not found: $SUMS" >&2
  exit 1
fi

shopt -s nullglob
artifacts=( "$DIST_DIR"/ctx-"$VERSION"-*.tar.gz "$DIST_DIR"/ctx-"$VERSION"-*.zip )
shopt -u nullglob
if [[ "${#artifacts[@]}" -eq 0 ]]; then
  echo "no release artifacts found for version $VERSION in $DIST_DIR" >&2
  exit 1
fi

assets=( "${artifacts[@]}" "$SUMS" "$MANIFEST" )
notes_file="$(mktemp)"
trap 'rm -f "$notes_file"' EXIT

first_artifact="$(basename "${artifacts[0]}")"
first_target="${first_artifact#ctx-${VERSION}-}"
first_target="${first_target%.tar.gz}"
first_target="${first_target%.zip}"

sed \
  -e "s/<version>/${VERSION}/g" \
  -e "s/<target>/${first_target}/g" \
  "$ROOT_DIR/.github/RELEASE_TEMPLATE.md" > "$notes_file"

title="CTX v${VERSION}: OpenCode-first graph memory and local context runtime"

if "$GH_BIN" release view "$TAG" --repo Alegau03/CTX >/dev/null 2>&1; then
  "$GH_BIN" release edit "$TAG" --repo Alegau03/CTX --title "$title" --notes-file "$notes_file"
  "$GH_BIN" release upload "$TAG" --repo Alegau03/CTX --clobber "${assets[@]}"
else
  "$GH_BIN" release create "$TAG" --repo Alegau03/CTX --title "$title" --notes-file "$notes_file" "${assets[@]}"
fi

echo "Published GitHub Release $TAG with ${#assets[@]} assets"
