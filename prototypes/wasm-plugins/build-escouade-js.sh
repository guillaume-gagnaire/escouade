#!/bin/sh
# Builds Escouade's Javy plugin (escouade-js/) and initializes it with the pinned Javy: target/escouade-js.wasm.
# Needs rustup's wasm32-wasip1 and the Javy of the README (ESCOUADE_JAVY, else %TEMP%/esc319/dl-javy/javy.exe).
set -e
here=$(cd "$(dirname "$0")" && pwd)
javy=${ESCOUADE_JAVY:-$(cygpath -u "$TEMP" 2>/dev/null || echo /tmp)/esc319/dl-javy/javy.exe}
cd "$here/escouade-js"
# rquickjs-sys runs bindgen with the clang of the WASI SDK it downloads, whose own headers it misses on Windows.
cargo build --release --target wasm32-wasip1 || true
inc=$(ls -d target/wasm32-wasip1/release/build/rquickjs-sys-*/out/wasi-sdk/lib/clang/*/include | head -1)
[ -n "$inc" ] && export BINDGEN_EXTRA_CLANG_ARGS="-I$(cd "$inc" && pwd -W 2>/dev/null || pwd)"
# Javy's own flags (its .cargo/config.toml); the target_features section kept, which `javy init-plugin` reads.
RUSTFLAGS="-C target-feature=+simd128" cargo build --release --target wasm32-wasip1
mkdir -p "$here/target"
"$javy" init-plugin target/wasm32-wasip1/release/escouade_js.wasm -o "$here/target/escouade-js.wasm"
ls -l "$here/target/escouade-js.wasm"
