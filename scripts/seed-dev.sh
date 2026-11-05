#!/bin/sh
# The seed of a build of development (ESC-338, docs/PLUGINS.md §8.7, §8.14): builds the module of each plugin in Rust of plugins/
# (or of the folders given), then writes src-tauri/seed/ as the release would, not signed: only a build of development takes
# it. `rustup target add wasm32-wasip1` first. Then run the app with ESCOUADE_SEED_DIR=src-tauri/seed.
set -e
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

if [ $# -gt 0 ]; then
  folders="$*"
else
  # A plugin in Rust says so in its manifest: `"run": { "wasm": "Cargo.toml" }`.
  folders=$(grep -l '"wasm"[[:space:]]*:' plugins/*/escouade-plugin.json 2>/dev/null | xargs -r -n 1 dirname)
fi

target=${CARGO_TARGET_DIR:-$root/src-tauri/target/plugins-wasm}
built=
for folder in $folders; do
  crate=$(sed -n 's/^name[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' "$folder/Cargo.toml" | head -n 1 | tr - _)
  CARGO_TARGET_DIR="$target" cargo build --release --target wasm32-wasip1 --locked --quiet --manifest-path "$folder/Cargo.toml"
  cp "$target/wasm32-wasip1/release/$crate.wasm" "$folder/plugin.wasm"
  built="$built $folder"
done

# shellcheck disable=SC2086
npx --yes tsx@4.23.15 store/scripts/seed.ts dev $built
