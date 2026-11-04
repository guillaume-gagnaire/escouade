#!/bin/sh
# Makes plugin.wasm, the module of this plugin: `rustup target add wasm32-wasip1` first. To run after a change of src/.
set -e
here=$(cd "$(dirname "$0")" && pwd)
target=${CARGO_TARGET_DIR:-${TMPDIR:-/tmp}/escouade-reactor}
cd "$here"
CARGO_TARGET_DIR="$target" cargo build --release --target wasm32-wasip1 --locked --quiet
cp "$target/wasm32-wasip1/release/reactor.wasm" plugin.wasm
wc -c plugin.wasm
