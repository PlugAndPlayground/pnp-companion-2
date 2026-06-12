#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository="${TM_REPOSITORY:-git@github.com:tailrmade/tm.git}"
ref="${TM_REF:-main}"
checkout_dir="$(mktemp -d "${TMPDIR:-/tmp}/tm-build.XXXXXX")"

cleanup() {
  rm -rf "$checkout_dir"
}
trap cleanup EXIT

git clone --depth 1 --branch "$ref" "$repository" "$checkout_dir"

(
  cd "$checkout_dir"
  yarn install --immutable
  yarn build:self-hosted
)

TM_DIR="$checkout_dir/dist" "$script_dir/build-tm-local.sh"
