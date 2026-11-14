#!/usr/bin/env bash
# La maquette Tauri pour le simulateur iOS (macOS ; `npm ci` à la racine du dépôt d'abord, pour Svelte et vite).
#   VITE_MS_SERVER=http://127.0.0.1:<port> VITE_MS_KEY=<clé> bash mobile-study/tauri/build-ios.sh
set -euo pipefail
cd "$(dirname "$0")"
npm ci
command -v xcodegen >/dev/null || brew install xcodegen
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
[ -d src-tauri/gen/apple ] || npx tauri ios init --ci
PL=$(ls src-tauri/gen/apple/*_iOS/Info.plist | head -1)
for kv in "NSCameraUsageDescription string Lire le QR code de l’appairage" "NSFaceIDUsageDescription string Confirmer une autorisation"; do
  set -- $kv; k=$1; t=$2; shift 2; /usr/libexec/PlistBuddy -c "Add :$k $t $*" "$PL" 2>/dev/null || true
done
/usr/libexec/PlistBuddy -c "Add :NSAppTransportSecurity dict" -c "Add :NSAppTransportSecurity:NSAllowsArbitraryLoads bool true" "$PL" 2>/dev/null || true
npx tauri ios build --target aarch64-sim --ci
find src-tauri/gen/apple/build -name "*.app" -maxdepth 4 -print
