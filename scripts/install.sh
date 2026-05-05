#!/usr/bin/env sh
set -eu

REPO_SLUG="${CTX_REPO_SLUG:-Alegau03/CTX}"
INSTALL_DIR="${CTX_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${CTX_VERSION:-latest}"
INSTALL_MARKER_PATH="${CTX_INSTALL_MARKER_PATH:-${XDG_DATA_HOME:-$HOME/.local/share}/ctx/install.json}"

log() {
  printf '%s\n' "$*"
}

fail() {
  printf 'ctx installer error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os:$arch" in
    Darwin:arm64) printf 'aarch64-apple-darwin' ;;
    Darwin:x86_64) printf 'x86_64-apple-darwin' ;;
    Linux:x86_64) printf 'x86_64-unknown-linux-gnu' ;;
    *)
      fail "unsupported platform: $os $arch"
      ;;
  esac
}

asset_name_for() {
  version="$1"
  target="$2"
  printf 'ctx-%s-%s.tar.gz' "$version" "$target"
}

download_base_for() {
  if [ "$VERSION" = "latest" ]; then
    printf 'https://github.com/%s/releases/latest/download' "$REPO_SLUG"
  else
    printf 'https://github.com/%s/releases/download/v%s' "$REPO_SLUG" "$VERSION"
  fi
}

checksum_ok() {
  file="$1"
  sums="$2"

  if command -v shasum >/dev/null 2>&1; then
    (cd "$(dirname "$file")" && shasum -a 256 -c "$sums" --ignore-missing >/dev/null 2>&1)
    return $?
  fi

  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$(dirname "$file")" && sha256sum -c "$sums" --ignore-missing >/dev/null 2>&1)
    return $?
  fi

  return 2
}

need_cmd curl
need_cmd tar

target="$(detect_target)"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT INT TERM

base_url="$(download_base_for)"

if [ "$VERSION" = "latest" ]; then
  version_label="$(curl -fsSL "https://github.com/$REPO_SLUG/releases/latest" | sed -n 's#.*tag/\\([^"]*\\)".*#\\1#p' | head -n1 | sed 's/^v//')"
  [ -n "$version_label" ] || fail "failed to resolve latest version"
else
  version_label="$VERSION"
fi

asset_name="$(asset_name_for "$version_label" "$target")"
archive_path="$tmpdir/$asset_name"
sums_path="$tmpdir/SHA256SUMS"

log "Installing CTX"
log "version: $version_label"
log "target:  $target"
log "source:  $base_url/$asset_name"

curl -fsSL "$base_url/$asset_name" -o "$archive_path" || fail "failed to download archive"
curl -fsSL "$base_url/SHA256SUMS" -o "$sums_path" || fail "failed to download SHA256SUMS"

if checksum_ok "$archive_path" "$sums_path"; then
  log "checksum: verified"
else
  status=$?
  if [ "$status" -eq 2 ]; then
    log "checksum: skipped (no supported hash checker found)"
  else
    fail "checksum verification failed"
  fi
fi

mkdir -p "$tmpdir/extract"
tar -xzf "$archive_path" -C "$tmpdir/extract" || fail "failed to extract archive"

binary_path="$(find "$tmpdir/extract" -type f -name ctx | head -n1)"
[ -n "$binary_path" ] || fail "ctx binary not found in archive"

mkdir -p "$INSTALL_DIR"
install -m 0755 "$binary_path" "$INSTALL_DIR/ctx"

mkdir -p "$(dirname "$INSTALL_MARKER_PATH")"
cat >"$INSTALL_MARKER_PATH" <<EOF
{"channel":"installer","version":"$version_label","install_dir":"$INSTALL_DIR","binary_path":"$INSTALL_DIR/ctx"}
EOF

log ""
log "CTX installed to $INSTALL_DIR/ctx"
log "installer marker: $INSTALL_MARKER_PATH"
log ""
log "Next:"
log "  export PATH=\"$INSTALL_DIR:\$PATH\""
log "  ctx help"
log "  ctx doctor"
