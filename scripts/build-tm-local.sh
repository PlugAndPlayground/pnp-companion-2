#!/usr/bin/env bash
set -euo pipefail

companion_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tm_dir="${TM_DIR:-${1:-"$companion_root/tm"}}"
output_root="${OUTPUT_ROOT:-"$companion_root/artifacts"}"
tm_dir="$(cd "$tm_dir" && pwd)"

if [[ ! -f "$tm_dir/index.html" ]]; then
  echo "TM directory must contain index.html: $tm_dir" >&2
  exit 1
fi

(
  cd "$companion_root"
  cargo build --release
)

binary_name="tm-companion"
[[ "${OS:-}" == "Windows_NT" ]] && binary_name="tm-companion.exe"
binary="$companion_root/target/release/$binary_name"
companion_only="$output_root/companion-only"
tm_local="$output_root/tm-local"

rm -rf "$companion_only" "$tm_local"
mkdir -p "$companion_only" "$tm_local/tm"
cp "$binary" "$companion_only/$binary_name"
cp "$binary" "$tm_local/$binary_name"
cp -R "$tm_dir/." "$tm_local/tm/"

echo "Companion-only binary: $companion_only/$binary_name"
echo "TM Local distribution: $tm_local"
