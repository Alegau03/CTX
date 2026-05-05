#!/usr/bin/env sh
set -eu

package_dir="${1:-packages/ctx-bin}"

if [ ! -f "$package_dir/package.json" ]; then
  echo "package.json not found in $package_dir" >&2
  exit 1
fi

printf 'Publishing npm package from %s\n' "$package_dir"
(
  cd "$package_dir"
  npm publish --access public
)
