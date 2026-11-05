#!/bin/sh
# Compiles the plugins in Rust that already build as modules, from their crates, into <target>/plugins-wasm/<name>.wasm:
# what plugins_wasm_parity_tests.rs plays in the app's host, and CI's proof that the kit compiles for wasm32-wasip1
# (docs/PLUGINS.md §8.10, A4). `rust-toolchain.toml` brings the target.
#
# A plugin's crate is an rlib, for `escouade-plugins`, until Z3: `--crate-type cdylib` makes it a reactor here, with no
# change to its manifest. This is no reproducible build: that is `escouade-plugin build` (A5).
#
#   sh scripts/plugins-wasm.sh [name...]     (by default, all of them)
set -e
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/../src-tauri"
target=${CARGO_TARGET_DIR:-target}
# The profile of a module (docs/PLUGINS.md §8.5.2), over the app's, whose panics unwind.
export CARGO_PROFILE_RELEASE_PANIC=abort
export CARGO_PROFILE_RELEASE_LTO=fat
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_OPT_LEVEL=3
export CARGO_PROFILE_RELEASE_STRIP=true
mkdir -p "$target/plugins-wasm"
for name in ${*:-linear codex}; do
  crate=escouade-plugin-$name
  cargo rustc --locked --release --target wasm32-wasip1 -p "$crate" --lib --crate-type cdylib
  cp "$target/wasm32-wasip1/release/escouade_plugin_$name.wasm" "$target/plugins-wasm/$name.wasm"
  wc -c "$target/plugins-wasm/$name.wasm"
done
