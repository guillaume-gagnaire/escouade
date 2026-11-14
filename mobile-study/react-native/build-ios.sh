#!/usr/bin/env bash
# La maquette React Native (Expo) pour iOS (macOS) : le turbo-module du Rust commun (uniffi-bindgen-react-native, un
# XCFramework), le projet natif par `expo prebuild`, les pods, puis xcodebuild en Release (Hermes) pour le simulateur
# et pour l'appareil (non signé, pour le poids).
#   EXPO_PUBLIC_MS_SERVER=http://127.0.0.1:<port> EXPO_PUBLIC_MS_KEY=<clé> bash mobile-study/react-native/build-ios.sh
set -euo pipefail
cd "$(dirname "$0")"
npm ci
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
(cd modules/escouade-core && node ../../node_modules/uniffi-bindgen-react-native/bin/cli.cjs build ios --and-generate --config ubrn.config.yaml --release)
npx expo prebuild --platform ios --no-install
(cd ios && pod install)
WS=$(ls -d ios/*.xcworkspace | head -1)
SCHEME=$(basename "$WS" .xcworkspace)
for sdk in iphonesimulator iphoneos; do
  dest="generic/platform=iOS Simulator"; [ $sdk = iphoneos ] && dest="generic/platform=iOS"
  xcodebuild -workspace "$WS" -scheme "$SCHEME" -configuration Release -sdk $sdk -destination "$dest" \
    -derivedDataPath build/ios-$sdk ARCHS=arm64 ONLY_ACTIVE_ARCH=NO CODE_SIGNING_ALLOWED=NO build -quiet
done
find build -name "*.app" -path "*Release-*" -maxdepth 6 -print
