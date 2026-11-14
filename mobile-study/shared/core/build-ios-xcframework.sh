#!/usr/bin/env bash
# Le Rust commun pour iOS par UniFFI (sous macOS) : la bibliothèque statique pour l'appareil et le simulateur, réunies
# en un XCFramework (module escouade_mobile_coreFFI), et les liaisons Swift.
#
#   bash mobile-study/shared/core/build-ios-xcframework.sh <dossier du XCFramework> <dossier des liaisons Swift>
set -euo pipefail
cd "$(dirname "$0")"
XC_OUT=$1
SWIFT_OUT=$2
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo build --release --features uniffi --lib --target aarch64-apple-ios
cargo build --release --features uniffi --lib --target aarch64-apple-ios-sim
# Les liaisons se lisent dans une bibliothèque non dépouillée : celle de la machine, en debug.
cargo build --features uniffi --lib
GEN=target/uniffi-swift
rm -rf "$GEN" && mkdir -p "$GEN/headers"
cargo run --quiet --features uniffi-cli --bin uniffi-bindgen -- generate --library target/debug/libescouade_mobile_core.dylib \
  --language swift --out-dir "$GEN" --no-format
cp "$GEN/escouade_mobile_coreFFI.h" "$GEN/headers/"
cp "$GEN/escouade_mobile_coreFFI.modulemap" "$GEN/headers/module.modulemap"
rm -rf "$XC_OUT/escouade_mobile_coreFFI.xcframework"
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libescouade_mobile_core.a -headers "$GEN/headers" \
  -library target/aarch64-apple-ios-sim/release/libescouade_mobile_core.a -headers "$GEN/headers" \
  -output "$XC_OUT/escouade_mobile_coreFFI.xcframework"
mkdir -p "$SWIFT_OUT"
cp "$GEN/escouade_mobile_core.swift" "$SWIFT_OUT/"
echo "XCFramework : $XC_OUT/escouade_mobile_coreFFI.xcframework ; liaisons : $SWIFT_OUT/escouade_mobile_core.swift"
