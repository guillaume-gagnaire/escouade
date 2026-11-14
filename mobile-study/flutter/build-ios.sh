#!/usr/bin/env bash
# La maquette Flutter pour iOS (macOS) : le simulateur n'accepte que le mode debug (JIT : son démarrage ne se compare
# pas) ; l'appareil, en release (AOT), non signé, donne le poids.
#   MS_SERVER=http://127.0.0.1:<port> MS_KEY=<clé> bash mobile-study/flutter/build-ios.sh
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p assets/texts
cp ../shared/texts/escouade-texts.json ../shared/texts/mobile.json assets/texts/
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
flutter pub get
flutter build ios --simulator --debug --dart-define=MS_SERVER="${MS_SERVER:?}" --dart-define=MS_KEY="${MS_KEY:?}"
flutter build ios --release --no-codesign --dart-define=MS_SERVER="${MS_SERVER:?}" --dart-define=MS_KEY="${MS_KEY:?}"
find build/ios -name "Runner.app" -print
