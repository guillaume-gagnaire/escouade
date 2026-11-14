#!/usr/bin/env bash
# Construit l'APK de la maquette React Native (Expo, release, x86_64 et arm64) : le turbo-module du Rust commun par
# uniffi-bindgen-react-native, le projet natif par `expo prebuild`, puis Gradle (Hermes, la nouvelle architecture).
#
#   EXPO_PUBLIC_MS_SERVER=http://10.0.2.2:<port> EXPO_PUBLIC_MS_KEY=<clé> bash mobile-study/react-native/build-android.sh
#
# Sous Windows, depuis un dossier court : les builds C++ (react-native-svg, expo-modules-core) dépassent 260 caractères
# depuis le worktree. Copier src/lib, mobile-study/shared et ce dossier dans la même arborescence relative, par exemple
# C:/r/src/lib, C:/r/m/shared et C:/r/m/rn, et lancer ce script depuis C:/r/m/rn.
set -euo pipefail
cd "$(dirname "$0")"
: "${EXPO_PUBLIC_MS_SERVER:?}" "${EXPO_PUBLIC_MS_KEY:?}"
node ../shared/push/fake-google-services.mjs dev.escouade.study.rn google-services.json
(cd modules/escouade-core && node ../../node_modules/uniffi-bindgen-react-native/bin/cli.cjs build android --and-generate --config ubrn.config.yaml --release)
# Le build.gradle généré cite un proguard-rules.pro que uniffi-bindgen-react-native 0.31.0-7 ne génère pas.
touch modules/escouade-core/android/proguard-rules.pro
npx expo prebuild --platform android --no-install
cd android
if [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* ]]; then G=./gradlew.bat; else G=./gradlew; fi
$G assembleRelease -PreactNativeArchitectures=x86_64,arm64-v8a --console=plain
ls -la app/build/outputs/apk/release/
