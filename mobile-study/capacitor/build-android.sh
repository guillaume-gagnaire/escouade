#!/usr/bin/env bash
# Construit l'APK de la maquette Capacitor (release, x86_64 et arm64) : l'interface, le Rust commun par cargo-ndk, ses
# liaisons Kotlin par UniFFI, puis Gradle.
#
#   VITE_MS_SERVER=http://10.0.2.2:<port> VITE_MS_KEY=<clé> bash mobile-study/capacitor/build-android.sh
set -euo pipefail
cd "$(dirname "$0")"
HERE=$PWD
CORE=$HERE/../shared/core
npx vite build
npx cap sync android
(cd "$CORE" && cargo ndk -t x86_64 -t arm64-v8a -o "$HERE/android/app/src/main/jniLibs" build --release --features uniffi)
# Les liaisons se lisent dans une bibliothèque non dépouillée (le profil release retire les symboles qu'UniFFI lit) :
# celle de la machine, en debug, a les mêmes.
(cd "$CORE" && cargo build --quiet --features uniffi --lib)
LIB=$(ls "$CORE"/target/debug/{libescouade_mobile_core.so,libescouade_mobile_core.dylib,escouade_mobile_core.dll} 2>/dev/null | head -1 || true)
(cd "$CORE" && cargo run --quiet --features uniffi-cli --bin uniffi-bindgen -- generate \
  --library "$LIB" --language kotlin --out-dir "$HERE/android/app/src/main/java" --no-format)
test -f "$HERE/android/app/src/main/java/uniffi/escouade_mobile_core/escouade_mobile_core.kt"
node ../shared/push/fake-google-services.mjs dev.escouade.study.capacitor android/app/google-services.json
cd android
if [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* ]]; then ./gradlew.bat assembleRelease --console=plain; else ./gradlew assembleRelease --console=plain; fi
ls -la app/build/outputs/apk/release/
