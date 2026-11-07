#!/bin/sh
# Compiles the plugins in Rust as modules, once, in a target they share, into <target>/plugins-wasm/<id-court>/plugin.wasm:
# what the tests of the app play in its host, and what a build of development runs for a plugin of the registry until the
# seed gives it (docs/PLUGINS.md §8.6.4, §8.7). `rust-toolchain.toml` brings the target.
#
# Those whose manifest says `run.wasm` are the workspace of `plugins/`, against `crates/` by its `[patch.crates-io]`.
#
# This is no reproducible build, nor what the store publishes: that is `escouade-plugin build`, from the plugin's folder
# alone, on the crates of crates.io (§8.17).
#
#   sh scripts/plugins-wasm.sh [id-court...]     (by default, all of them)
set -e
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/../src-tauri"
# Relative, like plugins/.cargo/config.toml: the two workspaces share it. An absolute CARGO_TARGET_DIR is taken as it is.
target=${CARGO_TARGET_DIR:-target}

ported=$(cd ../plugins && grep -l '"wasm"[[:space:]]*:' ./*/escouade-plugin.json | xargs -n 1 dirname | xargs -n 1 basename)
wanted=${*:-$ported}

crate_of() {
  sed -n 's/^name[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' "../plugins/$1/Cargo.toml" | head -n 1
}

# Copies the module of the plugin $1, compiled by its crate.
keep() {
  mkdir -p "$target/plugins-wasm/$1"
  rm -f "$target/plugins-wasm/$1/"*.cwasm
  cp "$target/wasm32-wasip1/release/$(crate_of "$1" | tr - _).wasm" "$target/plugins-wasm/$1/plugin.wasm"
  wc -c "$target/plugins-wasm/$1/plugin.wasm"
}

packages=
for name in $wanted; do
  case " $(echo $ported) " in
    *" $name "*) packages="$packages -p $(crate_of "$name")" ;;
  esac
done
if [ -n "$packages" ]; then
  # shellcheck disable=SC2086
  (cd ../plugins && cargo build --locked --release --target wasm32-wasip1 $packages)
fi

# The profile of a module, over the app's, whose panics unwind.
export CARGO_PROFILE_RELEASE_PANIC=abort
export CARGO_PROFILE_RELEASE_LTO=fat
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_OPT_LEVEL=3
export CARGO_PROFILE_RELEASE_STRIP=true
for name in $wanted; do
  case " $(echo $ported) " in
    *" $name "*) ;;
    *) cargo rustc --locked --release --target wasm32-wasip1 -p "$(crate_of "$name")" --lib --crate-type cdylib ;;
  esac
  keep "$name"
done
