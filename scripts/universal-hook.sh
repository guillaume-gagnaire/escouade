#!/bin/sh
# The universal macOS build (Release, `--target universal-apple-darwin`): Tauri merges the app's
# own binary for Apple Silicon and Intel (lipo), but not the crate's other binaries (escouade-hook,
# escouade-plugin), which the bundle ships beside it and then looks for in the universal folder.
# Merged here, as the bundling starts (beforeBundleCommand of
# src-tauri/tauri.universal-macos.conf.json), from the two that Tauri's cargo build just made.
set -eu
cd "$(dirname "$0")/.."
target=src-tauri/target
for profile in release debug; do
  out="$target/universal-apple-darwin/$profile"
  [ -f "$out/escouade" ] || continue
  for bin in escouade-hook escouade-plugin; do
    lipo -create -output "$out/$bin" \
      "$target/aarch64-apple-darwin/$profile/$bin" \
      "$target/x86_64-apple-darwin/$profile/$bin"
    lipo -info "$out/$bin"
  done
done
