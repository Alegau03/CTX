#!/usr/bin/env sh
set -eu

version="${1:?usage: prepare-homebrew-formula.sh <version> <sha256>}"
sha256="${2:?usage: prepare-homebrew-formula.sh <version> <sha256>}"
formula="${3:-Formula/ctx.rb}"

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT INT TERM

sed \
  -e "s#url \".*\"#url \"https://github.com/Alegau03/CTX/archive/refs/tags/v${version}.tar.gz\"#" \
  -e "s#sha256 \".*\"#sha256 \"${sha256}\"#" \
  "$formula" > "$tmp"

mv "$tmp" "$formula"
printf 'Updated %s for v%s\n' "$formula" "$version"
