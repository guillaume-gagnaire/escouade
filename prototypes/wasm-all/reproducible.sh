#!/bin/sh
# One compilation of the modules of the prototype, as the store and the review would make it (docs/PLUGINS.md §8.5), under
# Windows (Git Bash) as under Linux: the pinned toolchain, the lock, the canonical rustc. Prints their SHA-256.
#   sh reproducible.sh [--plain]   (--plain: without the canonical rustc, to see what it changes)
set -e
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.."; pwd -W 2>/dev/null || pwd)
out=${ESCOUADE_OUT:-$here/target/reproducible}
export CARGO_HOME=${CARGO_HOME:-$HOME/.cargo}
# A toolchain of its own, the minimal profile: with rust-src, rustc writes the paths of the standard library where it finds them on this machine.
export RUSTUP_HOME=${ESCOUADE_RUSTUP_HOME:-$here/target/rustup}
[ -n "$ESCOUADE_RUSTUP_HOME" ] || rustup toolchain install 1.98.0 --profile minimal --target wasm32-wasip1 --no-self-update >/dev/null 2>&1
[ -d "$here/guest/codex/generated" ] || { echo "run node prepare.mjs first" >&2; exit 2; }
if [ "$1" != "--plain" ]; then
  CARGO_TARGET_DIR="$out/tools" cargo build --release --locked --quiet --manifest-path "$here/rustc-canon/Cargo.toml"
  export RUSTC_WRAPPER="$out/tools/release/rustc-canon"
  export ESCOUADE_CANON_ROOT="$root"
fi
cd "$here/guest"
CARGO_TARGET_DIR="$out" cargo build --release --target wasm32-wasip1 --locked --quiet
cd "$out/wasm32-wasip1/release"
sha256sum linear.wasm codex.wasm
wc -c linear.wasm codex.wasm
