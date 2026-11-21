#!/usr/bin/env bash
# MO-3 (docs/MOBILE.md): the phone's Rust for iOS, into the Capacitor plugin (under macOS, with Xcode). cargo builds
# crates/escouade-mobile for the device (aarch64-apple-ios) and the simulator (aarch64-apple-ios-sim); xcodebuild
# gathers both in apps/mobile-plugin/ios/escouade_mobileFFI.xcframework; uniffi-bindgen writes the Swift bindings into
# apps/mobile-plugin/ios/Sources/EscouadeMobileCore. With --build, xcodebuild then builds the plugin's Swift package for
# the simulator (arm64, the slice the XCFramework has), unsigned.
#
#   bash scripts/mobile-ios.sh [--build]
#
# Nothing here is signed nor published.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
CRATE=$ROOT/crates/escouade-mobile
PLUGIN=$ROOT/apps/mobile-plugin
IOS=$PLUGIN/ios

cd "$CRATE"
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo build --release --locked --features uniffi --lib --target aarch64-apple-ios
cargo build --release --locked --features uniffi --lib --target aarch64-apple-ios-sim

# The bindings are read from a library with its symbols: the machine's, in debug.
cargo build --quiet --locked --features uniffi --lib
GEN=$CRATE/target/uniffi-swift
rm -rf "$GEN" && mkdir -p "$GEN/headers"
cargo run --quiet --locked --features uniffi-cli --bin uniffi-bindgen -- generate \
  --library target/debug/libescouade_mobile.dylib --language swift --out-dir "$GEN" --no-format
cp "$GEN/escouade_mobileFFI.h" "$GEN/headers/"
cp "$GEN/escouade_mobileFFI.modulemap" "$GEN/headers/module.modulemap"

rm -rf "$IOS/escouade_mobileFFI.xcframework"
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libescouade_mobile.a -headers "$GEN/headers" \
  -library target/aarch64-apple-ios-sim/release/libescouade_mobile.a -headers "$GEN/headers" \
  -output "$IOS/escouade_mobileFFI.xcframework"
mkdir -p "$IOS/Sources/EscouadeMobileCore"
cp "$GEN/escouade_mobile.swift" "$IOS/Sources/EscouadeMobileCore/"
ls -l "$IOS/escouade_mobileFFI.xcframework"

if [ "${1:-}" = "--build" ]; then
  cd "$PLUGIN"
  xcodebuild -scheme EscouadeCapacitorMobile -destination 'generic/platform=iOS Simulator' \
    -derivedDataPath "$CRATE/target/xcode" ARCHS=arm64 CODE_SIGNING_ALLOWED=NO build
fi
