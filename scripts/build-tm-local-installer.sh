#!/usr/bin/env bash
# Builds "Tailrmade Local" installers: the companion with a TM build bundled.
# Usage: scripts/build-tm-local-installer.sh [path/to/tm/dist]
set -euo pipefail

companion_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tm_dir="${TM_DIR:-${1:-"$companion_root/tm"}}"
tm_dir="$(cd "$tm_dir" && pwd)"

if [[ ! -f "$tm_dir/index.html" ]]; then
  echo "TM directory must contain index.html: $tm_dir" >&2
  exit 1
fi

if [[ "$tm_dir" != "$companion_root/tm" ]]; then
  rm -rf "$companion_root/tm"
  mkdir -p "$companion_root/tm"
  cp -R "$tm_dir/." "$companion_root/tm/"
fi

cd "$companion_root"
npm ci
npx tauri build --config tauri.tmlocal.conf.json "${@:2}"
echo "Installers: $companion_root/target/release/bundle"
