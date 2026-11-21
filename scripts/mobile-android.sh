#!/usr/bin/env bash
# MO-3 (docs/MOBILE.md): the phone's Rust for Android, into the Capacitor plugin. cargo-ndk builds
# crates/escouade-mobile for arm64-v8a (phones) and x86_64 (the emulator) into apps/mobile-plugin/android/src/main/jniLibs;
# uniffi-bindgen writes its Kotlin bindings into apps/mobile-plugin/android/src/main/java/uniffi. With --aar, Gradle then
# builds the plugin's AAR (unsigned), which needs the plugin's npm packages (@capacitor/android) and Gradle 8.14.
#
#   ANDROID_NDK_HOME=<the NDK> bash scripts/mobile-android.sh [--aar]
#
# GRADLE names Gradle (`gradle` by default). Nothing here is signed nor published.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
CRATE=$ROOT/crates/escouade-mobile
PLUGIN=$ROOT/apps/mobile-plugin
ANDROID=$PLUGIN/android
: "${ANDROID_NDK_HOME:?ANDROID_NDK_HOME must name the Android NDK}"

cd "$CRATE"
rustup target add aarch64-linux-android x86_64-linux-android
OUT=$CRATE/target/android-libs
rm -rf "$OUT"
# API level 26: the plugin's minSdk.
cargo ndk -P 26 -t arm64-v8a -t x86_64 -o "$OUT" build --release --locked --features uniffi
# Only the crate's library: boringtun builds a library of its own beside it, which the app does not load.
for abi in arm64-v8a x86_64; do
  mkdir -p "$ANDROID/src/main/jniLibs/$abi"
  cp "$OUT/$abi/libescouade_mobile.so" "$ANDROID/src/main/jniLibs/$abi/"
done

# The bindings are read from a library with its symbols: the machine's, in debug.
cargo build --quiet --locked --features uniffi --lib
LIB=
for f in target/debug/libescouade_mobile.so target/debug/libescouade_mobile.dylib target/debug/escouade_mobile.dll; do
  if [ -z "$LIB" ] && [ -f "$f" ]; then LIB=$f; fi
done
rm -rf "$ANDROID/src/main/java/uniffi"
cargo run --quiet --locked --features uniffi-cli --bin uniffi-bindgen -- generate --library "$LIB" --language kotlin \
  --out-dir "$ANDROID/src/main/java" --no-format
test -f "$ANDROID/src/main/java/uniffi/escouade_mobile/escouade_mobile.kt"
ls -l "$ANDROID"/src/main/jniLibs/*/libescouade_mobile.so

if [ "${1:-}" = "--aar" ]; then
  (cd "$PLUGIN" && npm ci --no-audit --no-fund)
  "${GRADLE:-gradle}" -p "$ANDROID" --no-daemon --console=plain assembleRelease
  ls -l "$ANDROID"/build/outputs/aar/
fi
