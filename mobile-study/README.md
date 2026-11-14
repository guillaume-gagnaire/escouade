# L'étude de l'app mobile (ESC-445)

Quatre maquettes de la même app, une par technique, et de quoi les mesurer. L'étude elle-même, ses chiffres et sa
recommandation sont dans [`docs/MOBILE-STUDY.md`](../docs/MOBILE-STUDY.md). Rien ici n'est fusionné dans l'app.

| Dossier | Technique | Le Rust commun |
| --- | --- | --- |
| `tauri/` | Tauri 2 (Svelte de `svelte-ui/`) | appelé directement (`invoke`) |
| `capacitor/` | Capacitor 8 (Svelte de `svelte-ui/`) | plugin natif sur UniFFI (Kotlin, Swift) |
| `react-native/` | React Native 0.88 (Expo 58) | turbo-module par uniffi-bindgen-react-native |
| `flutter/` | Flutter 3.50 | flutter_rust_bridge 2.13 |

Le socle commun :

- `shared/core/` : le Rust appelé par les quatre (XChaCha20-Poly1305 : `seal`, `open`, `bench_roundtrips`), avec ses
  tests (`cargo test`), dont l'ouverture d'un message scellé en JavaScript ;
- `shared/fixture/` : la conversation enregistrée de 300 messages (`node make-conversation.mjs` la réécrit) ;
- `shared/fake-server/` : le faux serveur d'Escouade qui la rejoue, sert le QR code de l'appairage et note les repères
  des maquettes (`PORT=<port> node server.mjs`) ;
- `shared/texts/` : les textes du téléphone, et ceux d'Escouade exportés de son catalogue pour Flutter ;
- `shared/push/` : le `google-services.json` factice (aucun projet Firebase n'existe) ;
- `svelte-ui/` : l'interface Svelte commune à Tauri et Capacitor, qui reprend telles quelles `src/lib/markdown.ts`,
  `src/lib/diff.ts`, `src/components/DiffView.svelte`, `src/app.css` et le catalogue des textes d'Escouade.

## Construire

Sous Android (testé sous Windows) : le JDK 21, le SDK Android (plateforme 36, NDK 29), Rust avec les cibles
`aarch64-linux-android` et `x86_64-linux-android`, `cargo-ndk`, et `npm ci` à la racine du dépôt (Svelte, vite,
marked). Chaque maquette a son `build-android.sh` (APK release, x86_64 et arm64, signée de la clé de débogage) ;
l'adresse du faux serveur et la clé de l'appairage passent par l'environnement :

```sh
export VITE_MS_SERVER=http://10.0.2.2:<port> VITE_MS_KEY=AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8   # Tauri, Capacitor
export MS_SERVER=$VITE_MS_SERVER MS_KEY=$VITE_MS_KEY                                                    # Flutter
export EXPO_PUBLIC_MS_SERVER=$VITE_MS_SERVER EXPO_PUBLIC_MS_KEY=$VITE_MS_KEY                            # React Native
bash mobile-study/tauri/build-android.sh
```

Les écueils de Windows rencontrés (ils sont dans l'étude) :

- Tauri lie sa bibliothèque dans `jniLibs` par un lien symbolique, que Windows refuse sans le mode développeur :
  `tauri/build-android.sh` la copie et lance Gradle sans ses tâches Rust ;
- React Native dépasse la limite de 260 caractères des chemins dans ses builds C++ quand le dépôt est profond : le
  construire depuis une copie dans un dossier court (un lecteur `subst` ne suffit pas, l'autolinking d'Expo résout
  les chemins réels) ;
- Flutter réclame son propre NDK, que sdkmanager installe mal sous Windows : `ndkVersion` est fixé au NDK 29.

Sous iOS, sur un Mac : `build-ios.sh` de chaque maquette (le simulateur, et l'appareil non signé pour le poids). La
CI les lance sur la branche d'étude `study/esc-445-ios` (`.github/workflows/mobile-study-ios.yml`).

## Mesurer

Sous Android, l'émulateur bridé (Android 15, 2 cœurs, 3 Go, 720 × 1600, rendu logiciel SwiftShader) avec une
empreinte enregistrée, et le faux serveur lancé :

```sh
PORT=<port> node mobile-study/bench/android.mjs tauri dev.escouade.study.tauri <apk>
PORT=<port> node mobile-study/bench/android-push.mjs dev.escouade.study.tauri notification
```

`bench/emu-pose.mjs` oriente l'appareil photo virtuel de l'émulateur vers l'affiche du QR code
(`-virtualscene-poster wall=<pair.png?margin=20>`, pose `0 -150 0 -1.907 0.35 3.411`). Sous iOS, `bench/ios.mjs`
(appelé par la CI). Les résultats bruts sont dans `results/`.
