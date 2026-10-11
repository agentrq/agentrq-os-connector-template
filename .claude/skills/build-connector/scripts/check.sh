#!/usr/bin/env bash
# Copyright 2026 Contextual, Inc. https://agentrq.com
# This notice may not be modified or removed.
# SPDX-License-Identifier: Apache-2.0

# Tests a connector crate on the host, builds its component, and optionally
# installs it and lists its tools.
#
#   check.sh [<crate dir>] [--install]
#
# --install needs the `connectors` binary on PATH (or CONNECTORS_BIN set to a
# command, such as the Docker alias in the build-connector skill).
set -euo pipefail

crate="."
install=0
for arg in "$@"; do
  case "$arg" in
    --install) install=1 ;;
    -h|--help) sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//' >&2; exit 2 ;;
    *) crate="$arg" ;;
  esac
done
cd "$crate"
[ -f Cargo.toml ] && [ -f manifest.json ] || { echo "$PWD has no Cargo.toml and manifest.json" >&2; exit 2; }

step() { printf '\n== %s\n' "$*"; }

step "cargo test (host)"
cargo test --quiet

step "cargo build --release --target wasm32-wasip2"
cargo build --release --target wasm32-wasip2 --quiet

name="$(sed -n 's/^name *= *"\(.*\)"/\1/p' Cargo.toml | head -n1)"
target_dir="$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
wasm="$target_dir/wasm32-wasip2/release/${name//-/_}.wasm"
[ -f "$wasm" ] || { echo "no component at $wasm" >&2; exit 1; }
printf 'component: %s (%s bytes)\n' "$wasm" "$(wc -c < "$wasm" | tr -d ' ')"

if [ "$install" = 1 ]; then
  bin="${CONNECTORS_BIN:-connectors}"
  connector="$(sed -n 's/^ *"name": *"\([^"]*\)".*/\1/p' manifest.json | head -n1)"
  step "$bin install"
  $bin install "$wasm"
  step "$bin $connector tools"
  $bin "$connector" tools
fi
