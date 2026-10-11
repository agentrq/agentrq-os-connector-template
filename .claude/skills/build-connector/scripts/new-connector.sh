#!/usr/bin/env bash
# Copyright 2026 Contextual, Inc. https://agentrq.com
# This notice may not be modified or removed.
# SPDX-License-Identifier: Apache-2.0

# Creates a connector crate from one of the bundled starter templates.
#
#   new-connector.sh <name> [--template minimal|api-key|oauth2] [--dir <parent>]
#                           [--sdk-path <path to sdk/connectors-sdk>]
#
# <name> is the connector id: a lowercase letter, then up to 62 lowercase
# letters, digits or '-'. The crate is written to <parent>/<name> (default:
# the current directory) and must not exist yet. Without --sdk-path the SDK
# comes from git (CONNECTORS_SDK_GIT, default https://github.com/agentrq/connectors).
set -euo pipefail

usage() {
  sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//' >&2
  exit 2
}

name=""
template="minimal"
parent="."
sdk_path=""
while [ $# -gt 0 ]; do
  case "$1" in
    --template) template="${2:-}"; shift 2 ;;
    --dir) parent="${2:-}"; shift 2 ;;
    --sdk-path) sdk_path="${2:-}"; shift 2 ;;
    -h|--help) usage ;;
    -*) echo "unknown option: $1" >&2; usage ;;
    *) [ -z "$name" ] || usage; name="$1"; shift ;;
  esac
done

[ -n "$name" ] || usage
if ! [[ "$name" =~ ^[a-z][a-z0-9-]{0,62}$ ]]; then
  echo "invalid connector name '$name': use ^[a-z][a-z0-9-]{0,62}\$, such as my-crm" >&2
  exit 2
fi

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source_dir="$here/../templates/$template"
if [ ! -f "$source_dir/manifest.json" ]; then
  echo "unknown template '$template': use minimal, api-key or oauth2" >&2
  exit 2
fi

crate="$parent/$name"
if [ -e "$crate" ]; then
  echo "$crate already exists" >&2
  exit 1
fi

if [ -n "$sdk_path" ]; then
  sdk_path="$(cd "$sdk_path" && pwd)"
  sdk_dep="connectors-sdk = { path = \"$sdk_path\" }"
else
  sdk_dep="connectors-sdk = { git = \"${CONNECTORS_SDK_GIT:-https://github.com/agentrq/connectors}\" }"
fi

mkdir -p "$crate/src"
# Every template names itself <template>-starter; the crate takes the new name.
sed "s/$template-starter/$name/g" "$source_dir/manifest.json" > "$crate/manifest.json"
sed "s/$template-starter/$name/g" "$source_dir/src/lib.rs" > "$crate/src/lib.rs"

cat > "$crate/Cargo.toml" <<EOF
[package]
name = "$name"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
crate-type = ["cdylib", "rlib"] # cdylib for the component, rlib for host tests

[dependencies]
$sdk_dep
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[profile.release]
opt-level = "s"
lto = true
strip = true

# A crate of its own. Delete this table to add it to an existing workspace.
[workspace]
EOF

# The SDK needs Rust 1.99 or newer; rustup installs it and the target on first use.
cat > "$crate/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.99"
targets = ["wasm32-wasip2"]
EOF

printf '/target/\n' > "$crate/.gitignore"

wasm="${name//-/_}.wasm"
cat <<EOF
Created $crate from the $template template.

Next:
  1. Edit $crate/manifest.json: title, description, tools, auth, permissions.
  2. Edit $crate/src/lib.rs: one request per tool.
  3. Check: $here/check.sh $crate
     (cargo test on the host, then the wasm32-wasip2 build: target/wasm32-wasip2/release/$wasm)
EOF
