#!/usr/bin/env bash
# La maquette Capacitor pour iOS (macOS ; `npm ci` à la racine du dépôt d'abord) : le Rust commun en XCFramework et
# ses liaisons Swift (UniFFI), le plugin local plugins/rust-core (paquet Swift), puis xcodebuild pour le simulateur
# et pour l'appareil (non signé, pour le poids).
#   VITE_MS_SERVER=http://127.0.0.1:<port> VITE_MS_KEY=<clé> bash mobile-study/capacitor/build-ios.sh
set -euo pipefail
cd "$(dirname "$0")"
npm ci
npx vite build
bash ../shared/core/build-ios-xcframework.sh "$PWD/plugins/rust-core/ios" "$PWD/plugins/rust-core/ios/Sources/EscouadeMobileCore"
[ -d ios ] || npx cap add ios --packagemanager SPM
npx cap sync ios
PL=ios/App/App/Info.plist
/usr/libexec/PlistBuddy -c "Add :NSCameraUsageDescription string Lire le QR code de l’appairage" "$PL" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :NSFaceIDUsageDescription string Confirmer une autorisation" "$PL" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :NSAppTransportSecurity dict" -c "Add :NSAppTransportSecurity:NSAllowsArbitraryLoads bool true" "$PL" 2>/dev/null || true
for sdk in iphonesimulator iphoneos; do
  dest="generic/platform=iOS Simulator"; [ $sdk = iphoneos ] && dest="generic/platform=iOS"
  xcodebuild -project ios/App/App.xcodeproj -scheme App -configuration Release -sdk $sdk -destination "$dest" \
    -derivedDataPath build/ios-$sdk ARCHS=arm64 ONLY_ACTIVE_ARCH=NO CODE_SIGNING_ALLOWED=NO build -quiet
done
find build -name "App.app" -path "*Release-*" -print
