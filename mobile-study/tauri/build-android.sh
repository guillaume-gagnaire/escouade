#!/usr/bin/env bash
# Construit l'APK de la maquette Tauri (release, x86_64 pour l'émulateur et arm64 pour un téléphone).
#
# Sous Windows sans le mode développeur, `tauri android build` compile le Rust puis échoue en liant la bibliothèque dans
# jniLibs (un lien symbolique, que Windows refuse sans ce mode) : on la copie à sa place, puis on lance Gradle sans ses
# tâches Rust. Ailleurs (macOS, Linux), `tauri android build` suffit.
#
#   VITE_MS_SERVER=http://10.0.2.2:<port> VITE_MS_KEY=<clé> bash mobile-study/tauri/build-android.sh
set -euo pipefail
cd "$(dirname "$0")"
ANDROID=src-tauri/gen/android
node ../shared/push/fake-google-services.mjs dev.escouade.study.tauri "$ANDROID/app/google-services.json"

if [[ "$(uname -s)" != MINGW* && "$(uname -s)" != MSYS* ]]; then
  npx tauri android build --apk --target x86_64 --target aarch64 --ci
  exit 0
fi

declare -A ABI=([x86_64]=x86_64 [aarch64]=arm64-v8a)
declare -A TRIPLE=([x86_64]=x86_64-linux-android [aarch64]=aarch64-linux-android)
for t in x86_64 aarch64; do
  # Échoue au lien symbolique, après la compilation du Rust : c'est attendu ici.
  npx tauri android build --apk --target "$t" --ci > "build-$t.log" 2>&1 || true
  so="src-tauri/target/${TRIPLE[$t]}/release/libescouade_mobile_tauri_lib.so"
  [[ -f "$so" ]] || { tail -30 "build-$t.log"; exit 1; }
  mkdir -p "$ANDROID/app/src/main/jniLibs/${ABI[$t]}"
  rm -f "$ANDROID/app/src/main/jniLibs/${ABI[$t]}/libescouade_mobile_tauri_lib.so"
  cp "$so" "$ANDROID/app/src/main/jniLibs/${ABI[$t]}/"
  rm -f "build-$t.log"
done
cd "$ANDROID"
./gradlew.bat assembleUniversalRelease -PabiList=x86_64,arm64-v8a -ParchList=x86_64,arm64 -PtargetList=x86_64,aarch64 \
  -x rustBuildUniversalRelease -x rustBuildX86_64Release -x rustBuildArm64Release --console=plain
ls -la app/build/outputs/apk/universal/release/
