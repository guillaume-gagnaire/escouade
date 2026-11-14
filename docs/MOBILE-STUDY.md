# L'app mobile : Tauri 2, Capacitor, React Native ou Flutter — l'étude (ESC-445)

Étude du 2026-11-14, pour décider de la technique de l'app mobile (décision 5 de `docs/REMOTE.md`). **Aucun code de l'app ne change** : le livrable est ce document, les quatre maquettes de `mobile-study/` (sur la branche `study/esc-445-ios` du dépôt, jamais fusionnée) et leurs mesures (`mobile-study/results/`). La décision revient à l'utilisateur ; la recommandation et son effort sont au §8.

**Le contexte.** L'app mobile est un client de l'API distante d'Escouade (le PC, un serveur auto-hébergé, plus tard une machine hébergée) : ni cœur ni agent sur le téléphone. Elle doit montrer les agents qui attendent et leur conversation, répondre à une autorisation sous verrou biométrique, recevoir un push chiffré de bout en bout quand elle est fermée, appeler le Rust d'Escouade (le pair WireGuard, le chiffrement) et s'appairer par QR code (`docs/REMOTE.md`, « L'app mobile »).

**Comment lire les sources.** Chaque affirmation dit d'où elle vient :

- **[mesuré]** : obtenu ici, sur les maquettes, le 2026-11-14 (le banc et ses résultats bruts sont dans `mobile-study/`) ;
- **[vérifié]** : lu dans le code des paquets (crates.io, npm, pub.dev) ou obtenu d'une API (GitHub, registres), le 2026-11-14 ;
- **[lu]** : vient d'une documentation ou d'un résumé de recherche, non vérifié dans le code ni par une mesure.

## En bref

**La recommandation : Capacitor**, qui reprend l'interface Svelte d'Escouade (91 % du code de la maquette repris ou partagé), avec des plugins officiels pour le push et le QR code. Sur l'émulateur Android bridé, sa WebView fait défiler la conversation de 300 messages à 60 images par seconde, comme les trois autres ; elle coûte deux fois la mémoire de React Native ou de Flutter. **Tauri 2 est écarté pour le téléphone** : son push passe par un plugin communautaire en 0.x qui perd le toucher d'une notification dans le cas courant, et son lecteur de QR dépend des services Google Play. **L'effort** : de l'ordre de 6 à 9 semaines d'un agent, en 5 tickets (§8), après la fondation de `docs/REMOTE.md`. Avant de lancer, deux essais : un vrai Android d'entrée de gamme, et un vrai projet Firebase avec une vraie clé APNs.

**Les mesures** (Android : l'émulateur bridé en entrée de gamme ; iOS : le simulateur de la CI ; détails aux §2 et §3) :

| | Tauri 2 | Capacitor | React Native (Expo) | Flutter |
| --- | --- | --- | --- | --- |
| Fluidité, 300 messages (Android) | 59,6 i/s, 0,7 % de saccades | 59,9 i/s, 0,2 % | 59,2 i/s, 0,3 % (un à-coup de 134 ms) | 59,8 i/s, 0,4 % |
| Démarrage jusqu'à la liste (Android, médiane) | 1,7 s | 1,5 s | 1,8 s | **0,9 s** |
| Poids téléchargé (Android arm64 ; iOS appareil) | **8,9 Mo** ; ≈ 6 Mo | 20,8 Mo ; 7,2 Mo | 34,4 Mo ; 39,5 Mo | 26,2 Mo ; 21,2 Mo |
| Mémoire, conversation ouverte (Android) | ≈ 213 Mo | ≈ 224 Mo | ≈ 100 Mo | **≈ 91 Mo** |
| Push app fermée affiché (Android ; iOS) | oui ; oui | oui ; oui | oui ; oui | oui ; oui |
| Toucher la notification rouvre l'app sur son contenu (Android) | **non** (cas courant) | oui, 1,2 s | oui, 1,7 s | oui, 0,4 s |
| Un push « données » fait tourner le Rust app fermée (Android) | non | non | **oui** | **oui** |
| Le plugin de push | communautaire, 0.4 | officiel (Ionic) | officiel (Expo) | officiel (Google) |
| QR code lu (Android) | **non** (ML Kit à télécharger par Play Services) | oui | oui | oui |
| Pont vers le Rust (Android, par appel) | 494 µs | 452 µs | 6,6 µs | 6,6 µs |
| Code repris d'Escouade ou partagé | **95 %** | **91 %** | 21 % | 0 % |
| Construire sous Windows | le mode développeur de Windows, ou un contournement | sans écueil | chemins trop longs : copie courte | NDK à fixer |

## 1. Ce qui a été construit

**La même maquette, quatre fois** (`mobile-study/tauri`, `capacitor`, `react-native`, `flutter`) [mesuré] :

1. la liste des agents qui attendent, puis la conversation d'un agent : 300 messages rejoués par un faux serveur (`shared/fake-server`) depuis une conversation enregistrée (`shared/fixture/conversation.json`) : 99 réponses en Markdown (titres, listes, tableaux, citations, blocs de code Rust et TypeScript colorés), 70 diffs, 65 sorties de commandes, 65 messages de l'utilisateur ;
2. la dernière ligne est une autorisation : la commande entière (quatre lignes) affichée telle quelle, « Refuser » et « Autoriser » ; « Autoriser » demande l'empreinte (le verrou biométrique du système) avant d'envoyer la réponse au serveur ;
3. le push : l'app s'inscrit (FCM sous Android, APNs sous iOS) ; un push qui arrive app fermée porte un texte **scellé** (XChaCha20-Poly1305, la clé de l'appairage) que l'app ouvre par le Rust ;
4. le pont vers le Rust : la même crate pour les quatre (`shared/core`, `seal`, `open`, `bench_roundtrips`), appelée sans pont par Tauri, par un plugin natif sur UniFFI par Capacitor (Kotlin, Swift), par uniffi-bindgen-react-native (un turbo-module JSI) par React Native, et par flutter_rust_bridge par Flutter ; ses tests ouvrent un message scellé par le faux serveur en JavaScript (les deux implémentations s'accordent) ;
5. l'appairage par QR code : l'appareil photo lit `escouade://pair?u=…&k=…`, l'app garde l'adresse et la clé, demande au serveur un texte scellé avec cette clé et l'ouvre par le Rust : c'est la preuve que l'appairage et le pont marchent.

Les briques de chaque maquette, les plus courantes de sa technique [vérifié, versions du 2026-11-14] :

| | Tauri 2 | Capacitor | React Native (Expo) | Flutter |
| --- | --- | --- | --- | --- |
| Version | Tauri 2.12 | Capacitor 8.6 | React Native 0.88, Expo 58 | Flutter 3.50 |
| Interface | Svelte d'Escouade (`svelte-ui/`) | la même | réécrite en React | réécrite en Dart |
| Conversation | DOM, `content-visibility` (comme le bureau) | la même | FlashList 2, react-native-marked | `ListView.builder`, flutter_markdown_plus |
| Push | tauri-plugin-notifications 0.4.6 (communautaire, Choochmeque) | @capacitor/push-notifications 8.1 (officiel) | expo-notifications 58 (Expo) | firebase_messaging 17 (FlutterFire, Google) |
| Biométrie | tauri-plugin-biometric 2.5 (officiel) | @capgo/capacitor-native-biometric 8.9 (communautaire : Capacitor n'en a pas d'officiel) | expo-local-authentication 58 | local_auth 3.0 (officiel, flutter.dev) |
| QR code | tauri-plugin-barcode-scanner 2.5 (officiel) | @capacitor/barcode-scanner 3.1 (officiel) | expo-camera 58 | mobile_scanner 7.5 (communautaire) |
| Rust | appelé directement (`invoke`) | plugin à soi : UniFFI 0.31 + JNA (Kotlin), XCFramework (Swift) | uniffi-bindgen-react-native 0.31.0-7 | flutter_rust_bridge 2.13 (cargokit) |

## 2. Les mesures sous Android

**Le banc** (`mobile-study/bench/android.mjs`, résultats bruts dans `mobile-study/results/android-*.json`) [mesuré] : l'émulateur d'Android Studio **bridé en téléphone d'entrée de gamme** (Android 15, image `google_apis` x86_64, **2 cœurs, 3 Go**, écran 720 × 1600 à 280 dpi, rendu **logiciel** SwiftShader) sur le poste Windows (i9-14900KF, WHPX), sans autre charge pendant les mesures. Les quatre APK sont en release, signées de la clé de débogage. L'émulateur ne vaut pas un vrai téléphone : les chiffres se comparent **entre eux**, pas à un appareil (voir §7).

### 2.1 La fluidité de la conversation de 300 messages

Six glissés lents (1,5 s, le doigt qui remonte dans l'historique : des messages à mettre en page et à peindre) puis six lancers ; les images présentées sont relevées par SurfaceFlinger sur la couche de l'app (`dumpsys SurfaceFlinger --latency`, la même mesure pour une WebView, des vues natives ou Flutter). Une saccade est un intervalle de plus d'1,5 période d'écran (25 ms à 60 Hz).

| | Tauri 2 | Capacitor | React Native | Flutter |
| --- | --- | --- | --- | --- |
| Glissés : images par seconde | 59,6 | 59,9 | 59,2 | 59,8 |
| Glissés : saccades | 0,7 % | 0,2 % | 0,3 % | 0,4 % |
| Glissés : pire image | 36 ms | 33 ms | **134 ms** | 34 ms |
| Lancers : images par seconde | 59,5 | 59,7 | 59,3 | 60,0 |
| Lancers : saccades | 0,6 % | 0,3 % | 0,9 % | 0 % |
| Lancers : pire image | 68 ms | 51 ms | 82 ms | 22 ms |
| 95ᵉ centile de l'intervalle | 18,2 ms | 18,0 ms | 18,2 ms | 18,0 ms |
| Ouvrir la conversation (du toucher à l'affichage) | 926 ms | 901 ms | **106 ms** | 672 ms |

**Ce que cela dit** : sur cet émulateur bridé, **les quatre défilent à 60 images par seconde, avec moins de 1 % de saccades**. La crainte d'une WebView poussive ne se vérifie pas ici : l'interface d'Escouade (le DOM entier, `content-visibility: auto`, comme le bureau) défile aussi bien que les listes paresseuses de React Native et de Flutter. Les différences sont dans les pires images : Flutter est le plus régulier (22 ms au pire) ; React Native a des à-coups ponctuels (134 ms) quand FlashList monte des messages lourds (un diff, un bloc coloré) ; les WebView restent sous 70 ms. L'ouverture de la conversation est plus longue dans les WebView (tout le DOM des 300 messages est construit d'un coup, 0,9 s) que dans les listes paresseuses (0,1 s pour React Native, 0,7 s pour Flutter, qui colore les blocs de code à la construction).

### 2.2 Le démarrage à froid

Huit lancements après un arrêt forcé et un vidage des caches du système. « Première image » : `am start -W` (TotalTime). « Liste affichée » : jusqu'au repère que l'app envoie au faux serveur une fois la liste des agents peinte, données comprises, sur l'horloge de l'hôte (l'aller-retour d'adb retranché).

| Médiane (min à max) | Tauri 2 | Capacitor | React Native | Flutter |
| --- | --- | --- | --- | --- |
| Première image | 344 ms (294 à 402) | 647 ms (614 à 733) | 391 ms (338 à 429) | 829 ms (766 à 980) |
| Liste affichée, données comprises | 1 682 ms (1 123 à 1 869) | 1 492 ms (958 à 1 651) | 1 803 ms (1 630 à 1 931) | **877 ms** (812 à 1 124) |

**Flutter affiche la liste deux fois plus tôt** que les trois autres : son moteur et le code Dart compilé d'avance démarrent ensemble, sans WebView ni moteur JavaScript à lancer. Les WebView montrent vite une première image (la couleur de fond), puis chargent l'interface ; React Native charge son paquet JavaScript (Hermes) avant le premier rendu utile. Les trois sont autour de 1,5 à 1,8 s sur ce téléphone bridé.

### 2.3 Le poids

L'APK construite contient x86_64 (pour l'émulateur) et arm64 ; « arm64 » est ce qu'un téléphone téléchargerait (l'APK sans les bibliothèques des autres ABI, ce que fait un App Bundle sur Google Play).

| | Tauri 2 | Capacitor | React Native | Flutter |
| --- | --- | --- | --- | --- |
| **Téléchargé par un téléphone arm64** | **8,9 Mo** | 20,8 Mo | 34,4 Mo | 26,2 Mo |
| Bibliothèques natives arm64 | 6,3 Mo (l'app entière, Rust et interface : une bibliothèque) | 5,3 Mo (dont 4,7 Mo pour le lecteur de QR, ML Kit embarqué ; le Rust 0,4 Mo) | 23,2 Mo (React Native 6,7, ML Kit 4,7, Hermes 2,6, Expo 1,5, le Rust et son turbo-module 1,5…) | 23,0 Mo (le moteur 10,8, le code Dart 6,8, ML Kit 4,7, le Rust 0,4) |
| Code Java/Kotlin (dex) | 1,3 Mo | 10,9 Mo (non réduit par R8 : Firebase, ML Kit, Kotlin, JNA) | 3,2 Mo | 1,5 Mo |

Tauri est le plus léger, parce que son lecteur de QR n'embarque pas ML Kit (il le télécharge par les services Google Play, voir §2.6) et qu'il réduit le code par R8. Capacitor descendrait vers 13 à 15 Mo avec R8 activé (son gabarit le laisse éteint) [estimé, non mesuré]. Tous restent légers pour une app de ce genre.

### 2.4 La mémoire

PSS (la mémoire propre de l'app plus sa part des pages partagées), conversation de 300 messages ouverte, après cinq secondes.

| | Tauri 2 | Capacitor | React Native | Flutter |
| --- | --- | --- | --- | --- |
| Le processus de l'app | 92 Mo | 104 Mo | 100 Mo | 91 Mo |
| Le processus de rendu de la WebView | 137 Mo | 136 Mo | — | — |
| **Ensemble** | **≈ 213 Mo** | **≈ 224 Mo** | **≈ 100 Mo** | **≈ 91 Mo** |

(Les processus isolés relevés comptent 16 Mo même sans WebView, un processus du système présent dans les quatre cas : ils sont retranchés de l'ensemble.) **Une WebView double la mémoire** : 210 à 220 Mo contre 90 à 100 Mo pour React Native et Flutter. Sur un téléphone de 3 à 4 Go, c'est sans gravité au premier plan ; en arrière-plan, Android tue plus tôt l'app la plus grosse, ce que le push compense (l'app se rouvre sur une notification).

### 2.5 Le push, app fermée

**Sans projet Firebase** (le ticket l'interdit), l'inscription auprès de FCM échoue dans les quatre (`FIS_AUTH_ERROR` : le `google-services.json` est factice, `mobile-study/shared/push`). Ce qui a été essayé à la place : le message FCM est remis à l'app **comme les services Google Play le font**, par la diffusion `com.google.android.c2dm.intent.RECEIVE` (adb en root sur l'émulateur, `bench/android-push.mjs`), l'app ayant été utilisée puis tuée en arrière-plan par le système (Accueil, puis `am kill`).

| | Tauri 2 | Capacitor | React Native | Flutter |
| --- | --- | --- | --- | --- |
| Le processus renaît sur le message | oui | oui | oui | oui |
| Un message « notification » est affiché, app fermée | oui (par le SDK de FCM) | oui | oui | oui |
| **Toucher la notification ouvre l'app sur son contenu**, ouvert par le Rust | **non** : l'app s'ouvre, mais le plugin perd les données du toucher quand la tâche de l'app était restée dans les récents (le cas courant) ; il ne les reçoit (1,2 s) que si l'app avait été arrêtée de force | oui, 1,2 s | oui, 1,7 s | oui, **0,4 s** |
| **Un message « données » fait tourner du code de l'app fermée** (ouvrir le texte scellé par le Rust, pour afficher une notification chiffrée de bout en bout) | **non** (le plugin ne fait rien sans la WebView) | **non** (le plugin officiel ne traite pas les messages « données » app fermée) | **oui** : une tâche JavaScript sans interface (expo-notifications, expo-task-manager) ouvre le texte par le Rust | **oui** : un isolat Dart (firebase_messaging) ouvre le texte par le Rust |

La deuxième ligne de différence compte pour Escouade : `docs/REMOTE.md` veut que **le relais ne voie qu'un réveil et que le téléphone déchiffre lui-même le texte de la notification**. Avec React Native et Flutter, c'est du code de l'app (JavaScript ou Dart, et le Rust) ; avec Tauri et Capacitor, il faut écrire ce service soi-même en Kotlin (un `FirebaseMessagingService` qui appelle le Rust par UniFFI et affiche la notification), quelques dizaines de lignes. **Sous iOS, les quatre ont besoin du même code natif** pour cela : une *Notification Service Extension* en Swift, qui lie la bibliothèque Rust (§3).

**Ce qui reste à essayer avec un vrai projet Firebase** (le chef le demandera) : l'inscription et le jeton dans les quatre ; l'envoi par l'API HTTP v1 de FCM depuis un relais ; la remise par les serveurs de Google, la priorité haute qui réveille un téléphone en veille (Doze), et la même chose sur un vrai téléphone.

### 2.6 Le QR code et le verrou biométrique

L'appareil photo de l'émulateur filme une pièce en 3D ; le QR code de l'appairage y est posé en affiche (`-virtualscene-poster`), et la caméra virtuelle est tournée vers lui par une macro (`bench/emu-pose.mjs`). Une empreinte est enregistrée sur l'émulateur, puis posée par `adb emu finger touch`.

| | Tauri 2 | Capacitor | React Native | Flutter |
| --- | --- | --- | --- | --- |
| QR code lu, appairage prouvé par le Rust | **non** : le plugin officiel attend le module de lecture de ML Kit, que les services Google Play doivent télécharger ; l'image de l'émulateur (sans Play Store) ne le reçoit jamais. Sur un téléphone avec Play Services, il arrive au premier usage [lu] ; sur un téléphone sans (dégooglisé, Huawei), jamais. | oui | oui | oui |
| Autorisation sous empreinte, envoyée au serveur | oui (et « aucune empreinte enregistrée » est bien dit) | oui | oui | oui |

### 2.7 Le pont vers le Rust

L'app appelle 1 000 fois `open` (ouvrir un texte scellé de 256 octets) et mesure le temps par appel ; le même travail fait dans le Rust prend 4 µs.

| | Tauri 2 | Capacitor | React Native | Flutter |
| --- | --- | --- | --- | --- |
| Par appel, vu de l'interface | 494 µs (`invoke`, asynchrone, sérialisé en JSON par la WebView) | 452 µs (le pont de Capacitor, asynchrone) | **6,6 µs** (JSI, synchrone) | **6,6 µs** (FFI, synchrone) |

Le pont de Tauri et de Capacitor est 70 fois plus lent, mais 0,5 ms par appel ne gêne pas ce que l'app demande au Rust (ouvrir une notification, chiffrer un message) ; il pèserait sur un flux, par exemple faire passer tout le trafic du tunnel WireGuard par l'interface, ce que l'app ne doit pas faire (le tunnel vit dans le Rust, du côté natif).

## 3. Les mesures sous iOS

**Le banc** (`mobile-study/bench/ios.mjs`), lancé par la CI sur la branche d'étude `study/esc-445-ios` (`.github/workflows/mobile-study-ios.yml`, runner macOS, Xcode 26.6, simulateur iPhone 17) [mesuré]. Le simulateur rend avec le GPU du Mac et ses chiffres de démarrage et de mémoire ne valent pas un iPhone ; ils se comparent entre eux.

| | Tauri 2 | Capacitor | React Native | Flutter (debug) |
| --- | --- | --- | --- | --- |
| Construite pour le simulateur, et pour l'appareil (non signée) | oui ; l'appareil non construit (`tauri ios build` veut une équipe de signature) | oui, les deux | oui, les deux | oui ; le simulateur en debug seulement |
| Poids du .app de l'appareil (arm64) | ≈ 6 Mo (celui du simulateur : 6,3 Mo) | **7,2 Mo** | 39,5 Mo | 21,2 Mo |
| Démarrage jusqu'à la liste affichée : meilleur, médiane (pire) sur 6 | 1,53 s, **1,63 s** (2,9 s) | 2,65 s, 7,1 s (18,2 s) | 2,35 s, 5,4 s (13,3 s) | 1,74 s, 2,1 s (4,2 s), en debug |
| Mémoire, liste affichée (RSS, Mo) | 201 (l'app ; le processus WebContent n'a pas été retrouvé) | 227 + 182 (WebContent) | 296 | 410, en debug |
| Pont vers le Rust, par appel | 387 µs | 318 µs | 8,2 µs | 103 µs, en debug |
| Push par `simctl push`, app fermée : la notification s'affiche | **oui** | **oui** | **oui** | **oui** |
| Push silencieux (content-available), app fermée : l'app se réveille | non | non | non | non (attendu : voir §7) |
| Construire les deux, sur le runner macOS de la CI | 2 min | 3 min | **18 min** (les pods, React Native compilé depuis ses sources C++) | 4 min |

Les captures des notifications sont dans `mobile-study/results/ios-push-*.png`. **Les démarrages sous le simulateur sont trop dispersés pour classer** Capacitor et React Native (2,4 à 18 s : le simulateur d'un runner partagé, à froid) ; Tauri est le plus stable. Sous iOS, les quatre maquettes se construisent et affichent le push app fermée ; le reste se mesurera sur un iPhone.

## 4. La part de code reprise d'Escouade

`mobile-study/bench/code-share.mjs` compte les lignes (ni vides ni de commentaire) de chaque maquette [mesuré] :

| | Tauri 2 | Capacitor | React Native | Flutter |
| --- | --- | --- | --- | --- |
| Écrites pour la technique | 98 | 188 | 481 | 640 |
| Écrites pour l'étude, partagées avec l'autre WebView (`svelte-ui/`) | 501 | 501 | — | — |
| **Reprises d'Escouade telles quelles** | 1 488 : `markdown.ts` (marked, DOMPurify, shiki), `diff.ts`, `DiffView.svelte`, `app.css` | les mêmes | 131 : `diff.ts` | aucune |
| Textes | le catalogue entier (`t()`) | le même | le même, par Metro (deux contournements : la rune `$state`, `navigator.language`) | 8 clés exportées en JSON |
| Types de la conversation | ceux de `src/lib/types.ts` | les mêmes | les mêmes | réécrits en Dart |
| **Part reprise ou partagée** | **95 %** | **91 %** | 21 % | 0 % |

Dans une vraie app, l'écart grandit : Tauri et Capacitor reprendraient aussi les vues de l'app (la conversation, les cartes d'autorisation et de question, le Kanban, la dictée, la vue d'ensemble), là où React Native et Flutter les réécriraient toutes, et les tiendraient à jour à chaque changement du bureau.

## 5. La maturité de ce qui fait le push, la biométrie, le QR code et le pont

[vérifié, API de GitHub, npm et pub.dev, le 2026-11-14 ; téléchargements de la dernière semaine sur npm, des 30 derniers jours sur pub.dev]

| Brique | Qui la tient | Version | Activité | Usage |
| --- | --- | --- | --- | --- |
| **Push, Tauri** : tauri-plugin-notifications | un particulier (Choochmeque), communautaire | 0.4.6 (le paquet npm en 0.5.0-rc) | dernier commit le 2026-11-13, 88 étoiles, 27 problèmes ouverts | 10 000 / semaine |
| Push, Tauri, l'officiel (tauri-plugin-notification 2.8) | Tauri | 2.8.1 | actif | notifications **locales** seulement : pas de push |
| **Push, Capacitor** : @capacitor/push-notifications | Ionic (officiel) | 8.1.3 | dernier commit le 2026-11-11 | 2,8 millions / semaine |
| **Push, Expo** : expo-notifications | Expo (officiel) | 58.1.9 | quotidienne | 9,2 millions / semaine |
| **Push, Flutter** : firebase_messaging | Google (FlutterFire) | 17.0.1 | dernier commit le 2026-11-13 | 4,5 millions / 30 jours |
| Biométrie, Capacitor : @capgo/capacitor-native-biometric | Capgo (communautaire) | 8.9.3 | 0 problème ouvert | 308 000 / semaine |
| QR, Tauri : tauri-plugin-barcode-scanner | Tauri (officiel) | 2.5.3 | actif | 38 000 / semaine ; **ML Kit non embarqué** (voir §2.6) |
| QR, Capacitor : @capacitor/barcode-scanner | Ionic / OutSystems (officiel) | 3.1.3 | actif, 35 problèmes ouverts | 254 000 / semaine |
| QR, Flutter : mobile_scanner | un particulier (communautaire) | 7.5.0 | actif, 1 179 étoiles | 2,4 millions / 30 jours |
| Pont, React Native : uniffi-bindgen-react-native | Mozilla (James Hugman) | **0.31.0-7** (avant 1.0, une préversion par semaine) | 569 étoiles, 69 problèmes ouverts | 342 000 / semaine |
| Pont, Flutter : flutter_rust_bridge | un particulier (fzyzcjy), communautaire | 2.13.0 (2.x stable depuis 2024) | 5 490 étoiles, 30 problèmes ouverts | 618 000 / 30 jours |
| Pont, Capacitor : UniFFI (Kotlin, Swift) | Mozilla | 0.31 (les liaisons Kotlin et Swift sont les plus anciennes d'UniFFI) | 5 136 étoiles | — |

Ce que cela dit :

- **le push est le point faible de Tauri**, comme l'utilisateur l'avait lu : l'officiel ne fait que des notifications locales, et le seul plugin de push actif est tenu par une personne, avant sa 1.0, utilisé 300 fois moins que celui de Capacitor ;
- Capacitor, Expo et FlutterFire tiennent leur push eux-mêmes, avec des millions d'installations par semaine ;
- le pont de React Native est le plus jeune (0.31.0-7, une préversion) ; il a demandé deux contournements (§6) ;
- flutter_rust_bridge et UniFFI sont mûrs.

## 6. L'effort de construire et de publier sur les deux stores

**Ce que la construction a demandé ici** [mesuré, sous Windows, le 2026-11-14] :

| | Tauri 2 | Capacitor | React Native (Expo) | Flutter |
| --- | --- | --- | --- | --- |
| Première APK release (Rust compris) | 5 à 6 min | 2 à 3 min | ≈ 4 min (après les contournements) | ≈ 5 min |
| Reconstruire après un changement d'interface | 4 min (le front est dans la bibliothèque Rust) | 40 s | 20 à 40 s | ≈ 1 min |
| Écueils rencontrés sous Windows | **le build Android échoue sans le mode développeur de Windows** (lien symbolique dans `jniLibs`) : contourné par une copie | aucun ; Capacitor 8 veut le JDK 21 et minSdk 26 (le lecteur de QR officiel) | **chemins de plus de 260 caractères** dans les builds C++ (react-native-svg) depuis le worktree : build depuis une copie courte ; uniffi-bindgen-react-native exige `repository` dans package.json et oublie un `proguard-rules.pro` | réclame son NDK et l'installe mal sous Windows : NDK fixé |
| Code natif à écrire soi-même | aucun (sauf le push chiffré, §8) | le plugin du Rust : 40 lignes de Kotlin, 45 de Swift, un paquet Swift | aucun (le turbo-module est généré) | aucun (cargokit compile le Rust) |
| Projet natif dans le dépôt | `src-tauri/gen/android`, `gen/apple` (générés une fois, puis à soi) | `android/`, `ios/` (à soi, ouverts dans Android Studio et Xcode) | aucun : `expo prebuild` le refait | `android/`, `ios/` (à soi) |

**Publier** [lu, documentations des quatre ; rien n'a été publié] :

- **les deux comptes** : le compte développeur Apple existe (la signature de macOS) ; Google Play demande un compte (25 $ une fois) et, pour un compte personnel récent, un test fermé de 12 testeurs pendant 14 jours avant la production. C'est le même pour les quatre.
- **iOS demande un Mac** pour les quatre, sauf React Native avec **EAS Build** (les builds iOS dans le nuage d'Expo, payant au-delà d'un petit quota) ; Escouade a déjà des Mac dans sa CI (la signature et la notarisation de macOS), qui suffiraient.
- **la signature et l'envoi** : Tauri (`tauri android build --aab`, `tauri ios build`, puis Transporter), Capacitor et Flutter (Gradle et Xcode, ou fastlane) se font à la main ou par fastlane ; Expo a **EAS Submit**, qui envoie aux deux stores. Capacitor a Appflow (Ionic, payant).
- **les mises à jour sans revue** : Capacitor (Capgo, Appflow) et Expo (EAS Update) peuvent pousser le JavaScript et le HTML sans repasser par la revue d'Apple, dans les limites de ses règles (pas de changement de fonction) ; Tauri ne le peut pas (le front est dans la bibliothèque Rust) ; Flutter non plus (Shorebird, un service tiers, le fait).
- **la revue d'Apple** : une app qui n'est qu'une WebView sur un site peut être refusée (règle 4.2, « minimum functionality ») ; ici l'app a du natif (push, biométrie, appareil photo, trousseau, WireGuard), ce qui suffit d'ordinaire [lu].

## 7. Ce qui n'a pas pu être mesuré

- **Un vrai téléphone**, ni Android d'entrée de gamme ni iPhone : l'émulateur bridé (2 cœurs, 3 Go, rendu logiciel) et le simulateur iOS (le GPU du Mac) ne valent pas un appareil. La fluidité surtout : un vrai GPU d'entrée de gamme (Mali, PowerVR) peut peiner sur la composition d'une WebView où SwiftShader, sur un i9, ne peine pas.
- **Le push par de vrais serveurs** : sans projet Firebase ni clé APNs, ni l'inscription (le jeton), ni l'envoi par l'API de FCM ou d'APNs, ni la remise, ni le réveil d'un téléphone en veille (Doze sous Android, le budget de réveil sous iOS). Sous Android, le message a été remis à l'app comme les services Google Play le font (§2.5) ; sous iOS, par `xcrun simctl push`, qui n'emprunte pas APNs.
- **Le push « données » sous iOS** : `simctl terminate` équivaut à fermer l'app depuis le sélecteur, après quoi iOS ne la réveille plus pour un push silencieux ; aucune des quatre ne s'est réveillée, comme attendu. Le déchiffrement d'une notification sous iOS passe de toute façon par une *Notification Service Extension* en Swift (la même pour les quatre), qui n'a pas été construite.
- **Toucher la notification sous iOS**, **la biométrie et le QR code sous iOS** : le simulateur n'a pas d'appareil photo, et toucher l'écran demanderait un outil d'automatisation (XCUITest) que l'étude n'a pas monté ; le verrou biométrique (Face ID) n'est donc vérifié que sous Android.
- **La fluidité sous iOS** (pas d'outil comparable à SurfaceFlinger sur le simulateur, et un rendu par le GPU du Mac).
- **Flutter sous iOS en release** : le simulateur n'accepte que le mode debug (JIT) ; son démarrage, sa mémoire et son pont sous iOS sont donc ceux du debug, plus lents, et ne se comparent pas.
- **La mémoire des WebView sous iOS** : le processus WebContent de WebKit n'a pas été retrouvé pour Tauri au premier run ; voir §3.
- **Le tunnel WireGuard** (`boringtun`, `smoltcp`) : le pont l'appellerait comme le chiffrement, mais il n'a pas été compilé pour le téléphone ici ; c'est l'essai prévu par `docs/REMOTE.md`.
- **La publication** : rien n'a été envoyé à un store ni signé pour un store ; l'effort du §6 vient des documentations.

## 8. La recommandation et son effort

**Capacitor**, en reprenant l'interface Svelte d'Escouade.

- **Il garde l'avantage de Tauri** — l'interface d'Escouade reprise à plus de 90 %, une seule interface à tenir pour le bureau, le web (option A de `docs/REMOTE.md`) et le téléphone — **sans ses faiblesses mobiles mesurées ici** : son push officiel rouvre l'app sur la notification là où le plugin communautaire de Tauri perd le toucher dans le cas courant ; son lecteur de QR officiel embarque ML Kit au lieu de dépendre des services Google Play ; il se construit sous Windows sans le mode développeur et se reconstruit en 40 s au lieu de 4 min ; il peut pousser le front sans revue de store.
- **La WebView tient la longue conversation** : 60 images par seconde, moins de 1 % de saccades, sur l'émulateur bridé, à égalité avec React Native et Flutter.
- **Ce qu'il coûte** : deux fois la mémoire de React Native ou de Flutter (≈ 220 Mo contre ≈ 100 Mo), un démarrage de 1,5 s contre 0,9 s pour Flutter, un pont 70 fois plus lent (sans effet sur ce que l'app demande au Rust), un peu de code natif à soi (le plugin du Rust : 85 lignes de Kotlin et de Swift ; le service de push chiffré sous Android).

**Les autres, et quand les reprendre** :

- **Tauri 2** : à écarter pour le téléphone aujourd'hui. Son push dépend d'un plugin communautaire en 0.x, tenu par une personne, qui perd le toucher d'une notification dans le cas courant ; son QR code dépend des services Google Play ; son build Android veut le mode développeur sous Windows. À reprendre si Tauri publie un push officiel.
- **Flutter** : le meilleur en démarrage (0,9 s), en mémoire (91 Mo), en régularité (22 ms au pire) et pour le push (du Dart et du Rust tournent app fermée). Mais toute l'interface est à réécrire en Dart (0 % repris), un langage de plus, et à tenir à jour à côté du bureau. À reprendre si le téléphone devient une app à part entière, différente du bureau, ou si un vrai téléphone d'entrée de gamme montre une WebView trop lente.
- **React Native (Expo)** : l'outillage le plus complet pour publier (EAS Build, qui construit iOS sans Mac, et EAS Submit), le push qui fait tourner du JavaScript app fermée, l'ouverture de conversation la plus rapide. Mais l'interface est à réécrire en React (21 % repris), le pont Rust est en préversion (0.31.0-7) et a demandé des contournements, l'APK est la plus lourde (34 Mo), et il a les pires à-coups de défilement (134 ms).

**L'effort, avec Capacitor** (de l'ordre de **6 à 9 semaines** d'un agent, en 5 tickets ; il s'ajoute à la fondation de `docs/REMOTE.md`, l'API distante, dont l'app est un client) :

| Ticket | Contenu | Effort |
| --- | --- | --- |
| 1. L'app et sa mise en page | `apps/mobile` en Capacitor 8 ; l'interface Svelte d'Escouade sur l'API distante (`ipc.ts` sur WebSocket, de la fondation) ; une mise en page d'une colonne pour la vue d'ensemble, la conversation, les autorisations et les questions (la commande entière), l'envoi d'un message, le Kanban, la dictée, l'aperçu ; R8 activé | 2 à 3 semaines |
| 2. Le Rust du téléphone | une crate `escouade-mobile` (le chiffrement du push, le pair WireGuard de `docs/REMOTE.md`) ; le plugin Capacitor sur UniFFI (Kotlin, Swift) ; sa construction en CI (cargo-ndk, XCFramework) | 1 à 2 semaines |
| 3. Le push chiffré de bout en bout | le relais (APNs, FCM) ; sous Android, un `FirebaseMessagingService` qui ouvre le texte par le Rust et affiche la notification ; sous iOS, une *Notification Service Extension* qui fait de même ; le retour sur l'agent au toucher | 1 à 2 semaines, plus un projet Firebase et une clé APNs (à créer par l'utilisateur) |
| 4. L'appairage et la sécurité de l'appareil | le QR code, la clé dans le trousseau (Keystore, Keychain), le verrou biométrique avant une réponse, la révocation depuis le PC | 1 semaine |
| 5. La publication | le compte Google Play, les signatures, la CI (l'AAB sous Linux, l'IPA sur les Mac de la CI), fastlane, les fiches des stores, le test fermé de 14 jours de Google Play, la première revue d'Apple | 1 semaine, plus les délais des stores |

Avant le ticket 1, deux essais courts lèveraient les doutes de §7 : la maquette Capacitor sur un vrai Android d'entrée de gamme (la fluidité), et le push par un vrai projet Firebase et une vraie clé APNs.

## 9. Sources

- Les maquettes et le banc : `mobile-study/` (README, `bench/`), les résultats bruts : `mobile-study/results/`.
- Les runs de la CI iOS : 43595906571 (échoué : Homebrew 6 refuse un tap non approuvé), 43596441385 (Tauri et Flutter mesurés ; Capacitor et React Native corrigés ensuite), 43598519056 (Capacitor et React Native mesurés).
- Les versions et l'activité : crates.io, npm (`npm view`, api.npmjs.org), pub.dev, l'API de GitHub, le 2026-11-14.
- Le plugin de push de Tauri : le code de `tauri-plugin-notifications` 0.4.6 (`TauriFirebaseMessagingService.kt`, `NotificationPlugin.kt` : `onIntent`, `extractPushNotificationData`, la mise en attente du clic) [vérifié].
- Le lecteur de QR de Tauri : `tauri-plugin-barcode-scanner` 2.5.3, `android/build.gradle.kts` (`play-services-mlkit-barcode-scanning`) [vérifié].
- La confiance des taps de Homebrew 6 : docs.brew.sh/Tap-Trust [lu].
- Le test fermé de Google Play pour les comptes personnels, EAS Build et Submit, Appflow, Capgo, Shorebird, la règle 4.2 d'Apple : leurs documentations [lu].

