#!/bin/sh
# Makes plugin.wasm, the module of the demonstration plugin: `rustup target add wasm32-wasip1` first. To run after a change
# of src/ or of crates/escouade-plugin-protocol.
set -e
here=$(cd "$(dirname "$0")" && pwd)
target=${CARGO_TARGET_DIR:-${TMPDIR:-/tmp}/escouade-minimal}
cd "$here"
CARGO_TARGET_DIR="$target" cargo build --release --target wasm32-wasip1 --quiet
cp "$target/wasm32-wasip1/release/escouade_plugin_minimal.wasm" plugin.wasm
wc -c plugin.wasm
