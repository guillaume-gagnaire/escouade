# Publier l'app mobile

La publication de l'app mobile d'Escouade (MO-8, ESC-576 ; conception : [`MOBILE.md`](MOBILE.md) § 8). Ce que l'utilisateur prépare une fois sur ses comptes Google Play et Apple, puis dans GitHub ; ce que fait le workflow `Mobile release` ; les fiches, les captures, la politique de confidentialité et les réponses à donner aux stores. English: [`MOBILE-PUBLISH.en.md`](MOBILE-PUBLISH.en.md). Ce fichier est la source du guide publié (claude.ai/artifact/KWdauPHqRs6wxFVe8Qk9Md) : un nom ou un choix qui change ici change là-bas dans la même revue.

| | |
|---|---|
| Identifiant (Android et iOS) | `dev.escouade.mobile` |
| Environnement GitHub | `mobile-release` |
| Canal de Google Play | `alpha` (le test fermé) |
| Profil d'Apple | `Escouade App Store` (App Store Connect) |
| Appareils | iPhone et téléphones Android ; pas d'iPad en 2.3 (M10) |

> **Les clés restent chez toi.** Tu poses chaque secret toi-même dans GitHub. Ne les colle jamais dans une conversation, une issue, un ticket ni « À toi » : aucun agent ne les lit ni ne les voit. Garde une copie hors ligne de la clé de téléversement (`.jks`), du certificat (`.p12`) et de la clé d'API (`.p8`) : Apple et Google ne te les redonneront pas.

Les comptes sont ceux de la société, vérifiés par son numéro D-U-N-S chez Google comme chez Apple : le vendeur affiché sur les deux stores est son nom légal, que les fiches ne répètent nulle part.

## 1. Google Play

1. **Créer l'app** dans la Play Console : nom `Escouade`, langue par défaut français (France), application gratuite ; accepter les déclarations du programme et des lois sur l'export.
2. **La clé de téléversement.** La signature de Google Play (Play App Signing) garde la clé de l'app chez Google ; la CI ne signe qu'avec la clé de téléversement. `keytool` vient avec le JDK :

   ```sh
   keytool -genkeypair -v -storetype PKCS12 -keystore escouade-upload.jks \
     -alias escouade-upload -keyalg RSA -keysize 4096 -validity 10000
   ```

   En PKCS12, le mot de passe de la clé est celui du magasin : le même dans les deux secrets.
3. **Un compte de service pour l'API de publication** (console.cloud.google.com, un projet à toi, par exemple `escouade-play`) : activer *Google Play Android Developer API* ; IAM › Comptes de service › Créer `play-ci`, sans rôle dans Google Cloud ; Clés › Ajouter une clé › JSON.
4. **Ses droits dans la Play Console** (Utilisateurs et autorisations › Inviter) : l'adresse du compte de service, sur l'app Escouade seule, « Publier des applications dans des canaux de test » et « Gérer les canaux de test et modifier les listes de testeurs ». Rien sur la production.
5. **Le test fermé** (Tester et publier › Tests › Test fermé) : garder le canal `alpha`, celui que vise la CI ; y ajouter des testeurs (des adresses ou un groupe Google) et partager le lien d'inscription. C'est le premier canal, pour essayer l'app sur de vrais téléphones avant la production. Un compte d'entreprise n'a pas de nombre de testeurs ni de durée imposés avant de demander la production.
6. **Le contenu de l'app** (Règles et programmes › Contenu de l'application) : la politique de confidentialité (§ 6), la sécurité des données (§ 7.1), la classification du contenu, le public cible (adultes, outil de développeur), aucune publicité.
7. **Le premier AAB, à la main, une fois.** Google n'accepte d'envoi par l'API qu'après un premier envoi depuis la console : lancer le workflow en « construire seulement » (§ 4), télécharger l'artefact `mobile-release-android`, déposer l'AAB dans le test fermé. La CI fait les suivants.

## 2. Apple

Le certificat, le profil et la clé d'API se créent avec un compte **Titulaire** (Account Holder) ou **Administrateur** de l'équipe ; la clé d'API elle-même a le rôle **Gestionnaire d'app** (App Manager).

1. **L'identifiant** (developer.apple.com › Certificates, Identifiers & Profiles › Identifiers › +) : App ID explicite `dev.escouade.mobile`, description « Escouade », capacité *Push Notifications* (MO-6).
2. **L'app dans App Store Connect** (Apps › + › Nouvelle app) : iOS, nom « Escouade » (une variante si le nom est pris), langue principale français, bundle ID `dev.escouade.mobile`, SKU `escouade-mobile`, iPhone seulement.
3. **Le certificat de distribution, sans Mac** (OpenSSL, dans Git Bash) :

   ```sh
   openssl req -new -newkey rsa:2048 -nodes -keyout apple-distribution.key \
     -out apple-distribution.csr -subj "/CN=Escouade Distribution/C=FR"
   ```

   La demande envoyée dans Certificates › + › *Apple Distribution* ; puis `distribution.cer` en `.p12` protégé par un mot de passe :

   ```sh
   openssl x509 -inform der -in distribution.cer -out apple-distribution.pem
   openssl pkcs12 -export -legacy -inkey apple-distribution.key \
     -in apple-distribution.pem -out apple-distribution.p12
   ```

   `-legacy` garde un chiffrement que le trousseau du Mac de la CI sait ouvrir. Le certificat vaut un an.
4. **Le profil** (Profiles › + › Distribution › *App Store Connect*) : App ID `dev.escouade.mobile`, le certificat d'avant, nom `Escouade App Store` (la CI le cherche par ce nom).
5. **La clé d'API** (App Store Connect › Utilisateurs et accès › Intégrations › App Store Connect API › Clés d'équipe) : rôle Gestionnaire d'app ; noter son ID et l'ID de l'émetteur ; le `.p8` ne se télécharge qu'une fois.
6. **TestFlight** : un groupe de testeurs internes (sans revue). Des testeurs externes passent par la revue bêta.
7. **Le chiffrement à l'export** : la réponse à valider est au § 7.3.

## 3. Les secrets dans GitHub

L'environnement : le dépôt › Settings › Environments › New environment `mobile-release` ; *Required reviewers* : toi ; *Deployment branches* : `main` et `release/*`. Les secrets vont dans cet environnement, jamais dans ceux du dépôt : une PR ou un fork ne les voit pas.

```sh
E=--env=mobile-release
base64 -w0 escouade-upload.jks | gh secret set ANDROID_UPLOAD_KEYSTORE_BASE64 $E
gh secret set ANDROID_UPLOAD_KEYSTORE_PASSWORD $E
gh secret set ANDROID_UPLOAD_KEY_ALIAS $E --body escouade-upload
gh secret set ANDROID_UPLOAD_KEY_PASSWORD $E
gh secret set GOOGLE_PLAY_SERVICE_ACCOUNT_JSON $E < play-ci.json

gh secret set APPLE_TEAM_ID $E
gh secret set APP_STORE_CONNECT_API_KEY_ID $E
gh secret set APP_STORE_CONNECT_API_ISSUER_ID $E
gh secret set APP_STORE_CONNECT_API_KEY_P8 $E < AuthKey_XXXXXXXXXX.p8
base64 -w0 apple-distribution.p12 | gh secret set IOS_DISTRIBUTION_CERT_P12_BASE64 $E
gh secret set IOS_DISTRIBUTION_CERT_PASSWORD $E
base64 -w0 Escouade_App_Store.mobileprovision | gh secret set IOS_PROVISIONING_PROFILE_BASE64 $E
```

| Secret | Contenu | Construire seulement | Envoyer |
|---|---|---|---|
| `ANDROID_UPLOAD_KEYSTORE_BASE64` | Le `.jks` de la clé de téléversement, en base64 | requis | requis |
| `ANDROID_UPLOAD_KEYSTORE_PASSWORD` | Son mot de passe | requis | requis |
| `ANDROID_UPLOAD_KEY_ALIAS` | `escouade-upload` | requis | requis |
| `ANDROID_UPLOAD_KEY_PASSWORD` | Le même mot de passe (PKCS12) | requis | requis |
| `GOOGLE_PLAY_SERVICE_ACCOUNT_JSON` | La clé JSON du compte de service `play-ci` | — | requis |
| `GOOGLE_SERVICES_JSON` | Le `google-services.json` du projet Firebase réduit à FCM (MO-6) | facultatif | facultatif |
| `APPLE_TEAM_ID` | L'identifiant d'équipe | requis | requis |
| `IOS_DISTRIBUTION_CERT_P12_BASE64` | Le `.p12` de distribution, en base64 | requis | requis |
| `IOS_DISTRIBUTION_CERT_PASSWORD` | Son mot de passe | requis | requis |
| `IOS_PROVISIONING_PROFILE_BASE64` | Le profil `Escouade App Store`, en base64 | requis | requis |
| `APP_STORE_CONNECT_API_KEY_ID` | L'ID de la clé d'API | — | requis |
| `APP_STORE_CONNECT_API_ISSUER_ID` | L'ID de l'émetteur | — | requis |
| `APP_STORE_CONNECT_API_KEY_P8` | Le contenu du `.p8` | — | requis |

`GOOGLE_SERVICES_JSON` peut manquer tant que MO-6 n'est pas là : le job le dit (un avis) et construit sans.

## 4. Le workflow `Mobile release`

`.github/workflows/mobile-release.yml`. **Lancé à la main seulement** (Actions › Mobile release › Run workflow), jamais au push ni la nuit. Deux entrées :

- **`platform`** : `android` (par défaut : le test fermé de Google ouvre la voie, M11), `ios` ou `both` ;
- **`build_only`** (coché par défaut) : l'AAB ou l'IPA signé rendu en artefact (`mobile-release-android`, `mobile-release-ios`, 14 jours), **rien d'envoyé**.

Chaque job attend l'approbation de l'environnement `mobile-release`, puis :

1. **vérifie les secrets** (`apps/mobile/release/release.mjs check`) : un secret qui manque arrête le job avant toute construction, son nom dans le message (`Missing secret APPLE_TEAM_ID in the environment mobile-release`) ; jamais une valeur ;
2. calcule les versions : le numéro du run pour Google (`versionCode`) et Apple (`CFBundleVersion`), la version de l'app sans sa pré-version pour Apple (`2.3.0-beta.2` → `2.3.0`) ;
3. construit la bibliothèque Rust du téléphone, la page, le projet natif ;
4. lance fastlane (`apps/mobile/fastlane/Fastfile`, version figée par `apps/mobile/Gemfile.lock`), qui vérifie les secrets une seconde fois :
   - **Android** : `android build` (l'AAB signé par la clé de téléversement, vérifié par `jarsigner`) ou `android closed_test` (puis `upload_to_play_store` sur `alpha`, sans fiche ni capture) ;
   - **iOS** : `ios build` (un trousseau jetable au mot de passe tiré au hasard, le certificat et le profil, la signature manuelle de la cible `App`, l'IPA) ou `ios testflight_upload` (puis `pilot` par la clé d'API, sans attendre le traitement, sans distribution externe) ;
5. **retire, même en échec**, les fichiers qu'étaient les secrets, le `google-services.json`, le trousseau et le profil.

**Jamais** de soumission à la revue d'Apple, de testeurs externes, de canal de production chez Google, ni d'envoi des fiches : ce sont tes gestes dans les consoles.

Essayé sans secret (2026-11-28, fastlane 2.242.0) : chaque lane s'arrête avant de rien faire, en nommant chaque secret qui manque.

## 5. Les fiches et les captures

- **Les textes** : `apps/mobile/store/listing/android/<langue>/` (`title.txt` ≤ 30, `short_description.txt` ≤ 80, `full_description.txt` ≤ 4 000, `release_notes.txt` ≤ 500) et `apps/mobile/store/listing/ios/<langue>/` (`name.txt` et `subtitle.txt` ≤ 30, `keywords.txt` ≤ 100, `promotional_text.txt` ≤ 170, `description.txt` ≤ 4 000, `release_notes.txt`, les adresses de confidentialité, d'assistance et du site), en `fr-FR` et `en-US`. Un test (`apps/mobile/release/release.test.ts`) tient les limites. Aucun nom de vendeur : le store affiche celui du compte. Les textes décrivent l'app à sa première publication, MO-6 (le push), MO-10 et MO-11 (la création) compris.
- **Les captures**, tirées de l'app sur des données fictives (le faux `claude` des tests) : `apps/mobile/store/screenshots/<taille>/<langue>/`, par `apps/mobile/e2e/store-shots.ts` :

  ```sh
  cargo build --bin escouade-server        # dans src-tauri/
  ESCOUADE_PORT_BASE=<base> npx playwright test -c apps/mobile/e2e/store-shots.config.ts
  ```

  | Dossier | Taille | Pour |
  |---|---|---|
  | `play` | 1 080 × 2 160 | Google Play, téléphone |
  | `play-tablet-7` | 1 080 × 1 920 | Google Play, tablette 7 pouces |
  | `play-tablet-10` | 1 800 × 3 200 | Google Play, tablette 10 pouces |
  | `appstore-6.9` | 1 290 × 2 796 | App Store, iPhone 6,9 pouces |
  | `appstore-6.5` | 1 242 × 2 688 | App Store, iPhone 6,5 pouces |

  Sur tablette, la page du téléphone est centrée, 600 px au plus (M10). Pas d'iPad : l'app est déclarée pour l'iPhone seul, l'App Store ne demande donc pas de captures d'iPad.

  Six écrans par langue : le fil « À toi » (en sombre et en clair), une autorisation dépliée, un choix et ses rendus, la conversation, les agents, le Kanban. Les textes des agents viennent du faux `claude`, en français ; ceux de l'app suivent la langue.
- **Les icônes et les images de Google Play**, tirées du logo d'Escouade (`website/public/logo.svg`) par `node apps/mobile/scripts/icons.mjs` : les icônes de l'app et ses écrans de lancement (Android et iOS), l'icône de Play en 512 × 512 et son image de présentation en 1 024 × 500 (PNG sans transparence), en `fr-FR` et en `en-US`, dans `apps/mobile/store/graphics/play/`. L'App Store prend l'icône dans le binaire.
- Les lanes n'envoient ni texte ni capture : tu les colles dans les consoles (ou une lane `supply`/`deliver` viendra plus tard, sur ton accord).

## 6. La politique de confidentialité

Sur le site : `https://escouade.dev/mobile/privacy/` et `https://escouade.dev/en/mobile/privacy/` (`website/content/mobile-privacy/`). Ce qu'elle dit : l'app ne parle qu'aux postes et serveurs que tu as appairés, par un tunnel chiffré ; l'éditeur ne reçoit rien d'autre que le push scellé, et seulement si tu l'acceptes ; de Firebase, rien d'autre que FCM sous Android ; aucun SDK Firebase sous iOS ; aucune statistique, aucune publicité, aucun suivi.

## 7. Les réponses à préparer

### 7.1 La sécurité des données (Google Play)

| Question | Réponse proposée |
|---|---|
| L'app collecte-t-elle ou partage-t-elle des données des types requis ? | Oui |
| Les données sont-elles chiffrées en transit ? | Oui |
| Peut-on demander la suppression des données ? | Oui : révoquer le téléphone depuis le poste, ou retirer l'espace dans l'app, efface son inscription au push |
| Types collectés | **Identifiants de l'appareil ou autres** (le jeton de push et l'identifiant d'installation de Firebase), pour le **fonctionnement de l'app** (les notifications), facultatif (accepté à l'appairage), non partagé, traité par un prestataire (Google, pour le compte de l'éditeur) |
| Messages, photos, fichiers, audio | **Non collectés** : ils vont du téléphone à ton propre poste ou serveur, par le tunnel chiffré de bout en bout ; l'éditeur ne les reçoit jamais. La dictée est celle du système. |
| Position, contacts, santé, finances, navigation, données d'usage, plantages | Non collectés |
| Publicité | Aucune |

Le point à confirmer de ton côté : Google définit la collecte comme l'envoi hors de l'appareil. Vérifie sur sa page d'aide « Sécurité des données » si ce qui ne va qu'à un serveur de l'utilisateur lui-même, chiffré de bout en bout, en est exclu. Dans le doute, déclare aussi « Messages » et « Photos », pour le fonctionnement de l'app, non partagés : c'est plus prudent, sans conséquence pour l'utilisateur.

### 7.2 L'étiquette de confidentialité (App Store)

- **Données collectées** : *Identifiants › Identifiant de l'appareil* (le jeton APNs gardé par le relais du push, avec un numéro d'appairage tiré au hasard), pour le *fonctionnement de l'app* ; **non lié à l'identité**, **pas utilisé pour le suivi**.
- Tout le reste : non collecté (l'app ne parle qu'à tes machines ; la reconnaissance vocale est celle d'iOS ; aucune analyse, aucun rapport de plantage).
- Avant MO-6 (pas de push) : « Aucune donnée collectée ».

### 7.3 Le chiffrement à l'export (App Store Connect)

`ITSAppUsesNonExemptEncryption` est **laissé hors de `Info.plist`** tant que tu n'as pas tranché : App Store Connect pose la question à chaque build (« Missing Compliance » dans TestFlight jusqu'à la réponse).

**Ce que fait l'app** : TLS (par le système), et, **dans l'app elle-même** (le Rust du téléphone), WireGuard (Curve25519, ChaCha20-Poly1305, BLAKE2s, par boringtun) et le scellement des notifications : des algorithmes standards, pour la confidentialité des échanges avec les machines de l'utilisateur. Ce n'est donc pas seulement « le chiffrement du système » ; dans le questionnaire d'Apple, c'est « des algorithmes de chiffrement standards en plus de ceux du système ».

**Ce qui en découle, à valider** (ce n'est pas un avis juridique) :

- **États-Unis (EAR)** : le code est libre et publié (GPL-3.0, public sur GitHub) ; un logiciel de chiffrement standard rendu public n'est pas soumis à l'EAR (§ 734.3(b)(3), § 742.15(b)) ; à défaut, il relève du grand public (5D992.c). Pas de CCATS à demander.
- **France** : la fourniture d'un moyen de cryptologie qui assure la confidentialité relève de la **déclaration à l'ANSSI** (loi n° 2004-575, art. 30 ; décret n° 2007-663), sauf dispense. Apple demande, pour une app distribuée en France, de joindre cette déclaration. C'est le point à régler : déposer la déclaration à l'ANSSI au nom de la société, ou vérifier une dispense.

**Deux voies** :

- **A (recommandée)** : répondre « algorithmes standards », déposer la déclaration à l'ANSSI et la joindre dans App Store Connect ; une fois le code de conformité reçu, poser `ITSAppUsesNonExemptEncryption` à `true` et `ITSEncryptionExportComplianceCode` à ce code.
- **B** : exclure la France de la distribution le temps de la déclaration.

### 7.4 La revue d'Apple : la note pour la règle 4.2

À coller dans App Store Connect › la version › *Informations pour la revue de l'app* › Notes, à la première soumission (en anglais, la langue de la revue). La règle 4.2 (« fonctionnalité minimale ») refuse une app qui n'est qu'un site web emballé ; la note montre le natif et donne au relecteur de quoi l'essayer.

```text
Escouade is the mobile companion of Escouade, an open source desktop app and server (GPL-3.0,
https://github.com/guillaume-gagnaire/escouade) that runs coding agents on the user's own machines.
The app is not a website: it has no web content of its own, no account and no server of ours in the
loop. It pairs with the user's computer or server and works through native features:

- Encrypted tunnel: a WireGuard tunnel, implemented natively (Rust, boringtun), straight to the
  user's machine; the app's data never goes through a server of ours.
- Keychain: each pairing's keys and tokens are stored in the iOS Keychain
  (kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly), never on disk in clear.
- Biometrics: every answer that makes an agent act (allowing a command, approving a plan or a
  ticket) is confirmed with Face ID or Touch ID (LocalAuthentication); the app refuses to pair on a
  device without a passcode.
- Camera: pairing reads a QR code shown by the user's computer (AVFoundation scanner).
- Push notifications: end-to-end sealed notifications (APNs), opened on the device by a
  Notification Service Extension that decrypts them with the pairing's key.
- Speech: messages to agents can be dictated with iOS speech recognition.

How to review it: the app needs an Escouade 2.3 computer or server to pair with. We provide a demo
server for the review: [address, pairing code and how long it stays valid: filled in at submission].
Without it, the app opens on its pairing screen, which explains what it is for.
```

La note suppose MO-6 fusionné (le push et son extension) ; sans lui, retire la ligne *Push notifications*. Le serveur de démonstration pour la revue (un `escouade-server` sur le VPS d'essai, avec des agents fictifs et un code d'appairage valable pendant la revue) est à monter au moment de la soumission ; sans lui, la revue risque un refus au titre de la règle 2.1 (l'app ne peut pas être essayée).

## 8. Ce qui reste ton geste

1. Préparer les comptes et poser les secrets (§ 1 à 3).
2. Lancer le workflow en « construire seulement », approuver l'environnement, déposer le premier AAB dans le test fermé à la main.
3. Ensuite, lancer le workflow pour envoyer (décoché), et approuver l'environnement, à chaque version.
4. Faire essayer l'app aux testeurs du test fermé, puis demander la production chez Google.
5. Trancher le chiffrement à l'export (§ 7.3), remplir la sécurité des données et l'étiquette de confidentialité (§ 7), coller les fiches et les captures.
6. Monter le serveur de démonstration et soumettre à la revue d'Apple, avec la note du § 7.4.

Les délais : Google relit l'app avant chaque premier passage d'un canal (de quelques heures à quelques jours) ; chez Apple, TestFlight interne est disponible dès la build traitée, et la sortie sur l'App Store passe par la revue (en général un à deux jours).
