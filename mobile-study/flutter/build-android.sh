#!/usr/bin/env bash
# Construit l'APK de la maquette Flutter (release, x86_64 et arm64). cargokit (posé par flutter_rust_bridge) compile le
# Rust commun pendant le build Gradle, pour chaque ABI.
#
#   MS_SERVER=http://10.0.2.2:<port> MS_KEY=<clé> bash mobile-study/flutter/build-android.sh
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p assets/texts
cp ../shared/texts/escouade-texts.json ../shared/texts/mobile.json assets/texts/
node ../shared/push/fake-google-services.mjs dev.escouade.study.flutter android/app/google-services.json
F=flutter
command -v flutter >/dev/null 2>&1 || F=flutter.bat
$F build apk --release --target-platform android-x64,android-arm64 \
  --dart-define=MS_SERVER="${MS_SERVER:?}" --dart-define=MS_KEY="${MS_KEY:?}"
ls -la build/app/outputs/flutter-apk/
