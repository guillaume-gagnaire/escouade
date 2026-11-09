#!/bin/sh
# Builds escouade-js, Escouade's Javy plugin (src-tauri/escouade-js/: the QuickJS of Javy 9.1 and the `escouade-turn`
# export, docs/PLUGINS.md §5.2), for wasm32-wasip1, then initializes it with the pinned Javy:
# src-tauri/escouade-js/target/escouade-js.wasm and its SHA-256 beside it (escouade-js.wasm.sha256). The app's build
# script embeds it (src-tauri/build.rs): without it, the app runs no `run.js` plugin and says why. CI builds it once
# (the `escouade-js` job) and hands it to every job that builds the app.
#
# The pinned Javy is downloaded once into src-tauri/escouade-js/target/javy/ and checked by its SHA-256 (the digests
# of the v9.1.0 release, the same the app checks, crates/escouade-plugin-cli/src/wasm.rs); ESCOUADE_JAVY names another.
# Needs rustup's wasm32-wasip1 target for the pinned toolchain (rustup target add wasm32-wasip1).
set -e
here=$(cd "$(dirname "$0")/../src-tauri/escouade-js" && pwd)
out="$here/target"
mkdir -p "$out"

if [ -n "$ESCOUADE_JAVY" ]; then
  javy=$ESCOUADE_JAVY
else
  case "$(uname -s)-$(uname -m)" in
    MINGW* | MSYS* | CYGWIN*) asset=javy-x86_64-windows-v9.1.0.gz sum=7148baab85d7426e8c18e2cc4deed4e9f03270cf3245dba5617a5be25a1de836 exe=javy.exe ;;
    Linux-x86_64) asset=javy-x86_64-linux-v9.1.0.gz sum=a68b122d48eb3dfc1b801d4e14c39271fde3638243d3272d206e376ac9189e39 exe=javy ;;
    Linux-aarch64) asset=javy-arm-linux-v9.1.0.gz sum=826962f0e82354cf97d6e928ea36777872bbae76c626e326948e0367c4b55cb5 exe=javy ;;
    Darwin-arm64) asset=javy-arm-macos-v9.1.0.gz sum=99e9ec6a8e8c98e119d137c08a921d2443d3b873c675a5571e1800f4451e6294 exe=javy ;;
    Darwin-x86_64) asset=javy-x86_64-macos-v9.1.0.gz sum=6eed2927575dc2b3fb5a1563eee1ce0874e6da91c9cec39a728c9377a8cc7b5a exe=javy ;;
    *) echo "no pinned Javy for $(uname -s)-$(uname -m): set ESCOUADE_JAVY" >&2; exit 1 ;;
  esac
  javy="$out/javy/$exe"
  if [ ! -x "$javy" ]; then
    mkdir -p "$out/javy"
    # GitHub's releases answer 502 to 504 now and then: tried again, further apart.
    curl -fsSL --retry 5 --retry-all-errors --retry-delay 10 -o "$out/javy/$asset" "https://github.com/bytecodealliance/javy/releases/download/v9.1.0/$asset"
    got=$( (sha256sum "$out/javy/$asset" 2>/dev/null || shasum -a 256 "$out/javy/$asset") | cut -d' ' -f1)
    if [ "$got" != "$sum" ]; then
      echo "$asset: SHA-256 $got, not $sum" >&2
      rm -f "$out/javy/$asset"
      exit 1
    fi
    gunzip -c "$out/javy/$asset" > "$javy"
    chmod +x "$javy"
    rm -f "$out/javy/$asset"
  fi
fi

cd "$here"
# The same bytes wherever it is built on Linux or macOS (ESC-328): the store's image and CI must embed the same
# escouade-js, or the local review could not compare what they compile. rustc writes the paths of the sources into the
# module (its panics' locations): a wrapper of rustc takes out those of this machine. Not in RUSTFLAGS: cargo hashes them
# into the symbols, which would differ by machine all the same. Under Windows the paths keep their backslashes whatever
# is done, so a module built there differs: the reference is the Linux one (docs/STORE.md §3.3).
export RUSTFLAGS="-C target-feature=+simd128" # Javy's own flags (its .cargo/config.toml).
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) ;;
  *)
    {
      echo '#!/bin/sh'
      echo 'rustc=$1; shift'
      echo "exec \"\$rustc\" --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo --remap-path-prefix=$(rustc --print sysroot)=/rust --remap-path-prefix=$here=/escouade-js \"\$@\""
    } > "$out/rustc-wrapper"
    chmod +x "$out/rustc-wrapper"
    export RUSTC_WRAPPER="$out/rustc-wrapper"
    ;;
esac
# rquickjs-sys runs bindgen with the clang of the WASI SDK it downloads, whose own headers it misses on Windows: a
# first build fetches it, then its include folder is given to bindgen.
if ! cargo build --release --target wasm32-wasip1 2>/dev/null; then
  inc=$(ls -d target/wasm32-wasip1/release/build/rquickjs-sys-*/out/wasi-sdk/lib/clang/*/include 2>/dev/null | head -1)
  [ -n "$inc" ] && export BINDGEN_EXTRA_CLANG_ARGS="-I$(cd "$inc" && (pwd -W 2>/dev/null || pwd))"
fi
# The `target_features` section kept (strip = false in Cargo.toml), which `javy init-plugin` (binaryen) reads.
# rquickjs-sys downloads its WASI SDK from GitHub's releases, which answer 502 to 504 now and then (2026-11-08): its
# curl keeps the error page as the archive, which it would read again. A build that failed drops an archive that is not
# gzip and tries again, further apart; the SDK stays where rquickjs-sys puts it, so the module's bytes do not change.
tries=1
wait=10
until cargo build --release --target wasm32-wasip1; do
  [ "$tries" -ge 4 ] && exit 1
  for a in target/wasm32-wasip1/release/build/rquickjs-sys-*/out/wasi-sdk/wasi-sdk-*.tar.gz; do
    if [ -f "$a" ] && ! gzip -t "$a" 2>/dev/null; then
      rm -f "$a"
      echo "$a: not gzip, downloaded again" >&2
    fi
  done
  sleep "$wait"
  wait=$((wait * 3))
  tries=$((tries + 1))
done
# --deterministic: fixed clocks and a zero-filled RNG during the pre-initialization, without which each run differs.
"$javy" init-plugin --deterministic target/wasm32-wasip1/release/escouade_js.wasm -o "$out/escouade-js.wasm"
sum=$( (sha256sum "$out/escouade-js.wasm" 2>/dev/null || shasum -a 256 "$out/escouade-js.wasm") | cut -d' ' -f1)
echo "$sum" > "$out/escouade-js.wasm.sha256"
echo "escouade-js.wasm: $(wc -c < "$out/escouade-js.wasm" | tr -d ' ') bytes, SHA-256 $sum"
