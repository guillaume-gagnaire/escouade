#!/bin/sh
# Writes the Cargo.lock of a plugin in `run.wasm`: the one `escouade-plugin build` and the store read, on the crates of
# crates.io (docs/PLUGINS.md §8.19). In this repository cargo ignores it, so it is made from the plugin's folder alone,
# out of the workspace of `plugins/` and its `[patch.crates-io]`.
#
# It starts from the Cargo.lock of `src-tauri`: the versions of `rust_native.json` (the closed list of what runs natively
# in a build) stay, and cargo only drops what the plugin does not use. Network: crates.io.
#
#   sh scripts/plugin-lock.sh <id-court>...
set -e
root=$(cd "$(dirname "$0")/.." && pwd)
for name in "$@"; do
  folder="$root/plugins/$name"
  test -f "$folder/Cargo.toml"
  work=$(mktemp -d)
  mkdir -p "$work/$name"
  cp -R "$folder/Cargo.toml" "$folder/src" "$work/$name/"
  cp "$root/src-tauri/Cargo.lock" "$work/$name/Cargo.lock"
  # The toolchain of the repository, whatever the folder cargo runs from.
  (cd "$root/plugins" && cargo metadata --format-version 1 --manifest-path "$work/$name/Cargo.toml" >/dev/null)
  cp "$work/$name/Cargo.lock" "$folder/Cargo.lock"
  rm -rf "$work"
  grep -c '^name = ' "$folder/Cargo.lock"
done
