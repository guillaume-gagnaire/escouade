# Publishing the mobile app

The publication of Escouade's mobile app (MO-8, ESC-576; design: [`MOBILE.md`](MOBILE.md) § 8). What the user prepares once on their Google Play and Apple accounts, then in GitHub; what the `Mobile release` workflow does; the listings, the screenshots, the privacy policy and the answers the stores ask for. Français : [`MOBILE-PUBLISH.md`](MOBILE-PUBLISH.md), the source of the published guide (claude.ai/artifact/KWdauPHqRs6wxFVe8Qk9Md): a name or a choice that changes here changes there in the same review.

| | |
|---|---|
| Identifier (Android and iOS) | `dev.escouade.mobile` |
| GitHub environment | `mobile-release` |
| Google Play track | `alpha` (the closed test) |
| Apple profile | `Escouade App Store` (App Store Connect) |
| Devices | iPhone and Android phones; no iPad in 2.3 (M10) |

> **The keys stay with you.** You set every secret yourself in GitHub. Never paste one in a conversation, an issue, a ticket or “Your turn”: no agent reads or sees them. Keep an offline copy of the upload key (`.jks`), the certificate (`.p12`) and the API key (`.p8`): Apple and Google will not give them back.

The accounts are the company's, verified by its D-U-N-S number at Google and at Apple: the seller shown on both stores is its legal name, which the listings never repeat.

## 1. Google Play

1. **Create the app** in the Play Console: name `Escouade`, default language French (France), free app; accept the program and export law declarations.
2. **The upload key.** Play App Signing keeps the app's key at Google; the CI signs with the upload key alone:

   ```sh
   keytool -genkeypair -v -storetype PKCS12 -keystore escouade-upload.jks \
     -alias escouade-upload -keyalg RSA -keysize 4096 -validity 10000
   ```

   In PKCS12, the key's password is the store's: the same in both secrets.
3. **A service account for the publishing API** (console.cloud.google.com, a project of yours such as `escouade-play`): enable *Google Play Android Developer API*; IAM › Service accounts › Create `play-ci`, with no Google Cloud role; Keys › Add key › JSON.
4. **Its rights in the Play Console** (Users and permissions › Invite): the service account's address, on the Escouade app alone, “Release apps to testing tracks” and “Manage testing tracks and edit tester lists”. Nothing on production.
5. **The closed test** (Test and release › Testing › Closed testing): keep the `alpha` track, the one the CI targets; add testers (addresses or a Google group) and share the opt-in link. It is the first track, to try the app on real phones before production. An organization account has no required number of testers nor duration before applying for production.
6. **App content** (Policy and programs › App content): the privacy policy (§ 6), data safety (§ 7.1), content rating, target audience (adults, a developer tool), no ads.
7. **The first AAB, by hand, once.** Google accepts API uploads only after a first upload from the console: run the workflow in “build only” (§ 4), download the `mobile-release-android` artifact, drop the AAB in the closed test. The CI does the next ones.

## 2. Apple

The certificate, the profile and the API key are created with an **Account Holder** or **Admin** account of the team; the API key itself has the **App Manager** role.

1. **The identifier** (developer.apple.com › Certificates, Identifiers & Profiles › Identifiers › +): explicit App ID `dev.escouade.mobile`, description “Escouade”, *Push Notifications* capability (MO-6).
2. **The app in App Store Connect** (Apps › + › New App): iOS, name “Escouade” (a variant if taken), primary language French, bundle ID `dev.escouade.mobile`, SKU `escouade-mobile`, iPhone only.
3. **The distribution certificate, without a Mac** (OpenSSL, in Git Bash):

   ```sh
   openssl req -new -newkey rsa:2048 -nodes -keyout apple-distribution.key \
     -out apple-distribution.csr -subj "/CN=Escouade Distribution/C=FR"
   ```

   The request sent in Certificates › + › *Apple Distribution*; then `distribution.cer` into a password-protected `.p12`:

   ```sh
   openssl x509 -inform der -in distribution.cer -out apple-distribution.pem
   openssl pkcs12 -export -legacy -inkey apple-distribution.key \
     -in apple-distribution.pem -out apple-distribution.p12
   ```

   `-legacy` keeps an encryption the CI Mac's keychain can open. The certificate lasts a year.
4. **The profile** (Profiles › + › Distribution › *App Store Connect*): App ID `dev.escouade.mobile`, the certificate above, name `Escouade App Store` (the CI looks for it by that name).
5. **The API key** (App Store Connect › Users and Access › Integrations › App Store Connect API › Team Keys): App Manager role; note its ID and the Issuer ID; the `.p8` downloads once.
6. **TestFlight**: a group of internal testers (no review). External testers go through beta review.
7. **Export compliance**: the answer to settle is in § 7.3.

## 3. The secrets in GitHub

The environment: the repository › Settings › Environments › New environment `mobile-release`; *Required reviewers*: you; *Deployment branches*: `main` and `release/*`. The secrets go in that environment, never in the repository's: a PR or a fork never sees them.

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

| Secret | Content | Build only | Upload |
|---|---|---|---|
| `ANDROID_UPLOAD_KEYSTORE_BASE64` | The upload key's `.jks`, in base64 | required | required |
| `ANDROID_UPLOAD_KEYSTORE_PASSWORD` | Its password | required | required |
| `ANDROID_UPLOAD_KEY_ALIAS` | `escouade-upload` | required | required |
| `ANDROID_UPLOAD_KEY_PASSWORD` | The same password (PKCS12) | required | required |
| `GOOGLE_PLAY_SERVICE_ACCOUNT_JSON` | The JSON key of the `play-ci` service account | — | required |
| `GOOGLE_SERVICES_JSON` | `google-services.json` of the Firebase project reduced to FCM (MO-6) | optional | optional |
| `APPLE_TEAM_ID` | The team ID | required | required |
| `IOS_DISTRIBUTION_CERT_P12_BASE64` | The distribution `.p12`, in base64 | required | required |
| `IOS_DISTRIBUTION_CERT_PASSWORD` | Its password | required | required |
| `IOS_PROVISIONING_PROFILE_BASE64` | The `Escouade App Store` profile, in base64 | required | required |
| `APP_STORE_CONNECT_API_KEY_ID` | The API key's ID | — | required |
| `APP_STORE_CONNECT_API_ISSUER_ID` | The issuer's ID | — | required |
| `APP_STORE_CONNECT_API_KEY_P8` | The `.p8`'s content | — | required |

`GOOGLE_SERVICES_JSON` may be missing until MO-6 lands: the job says so (a notice) and builds without it.

## 4. The `Mobile release` workflow

`.github/workflows/mobile-release.yml`. **Launched by hand alone** (Actions › Mobile release › Run workflow), never on a push nor at night. Two inputs:

- **`platform`**: `android` (the default: Google's closed test opens the way, M11), `ios` or `both`;
- **`build_only`** (checked by default): the signed AAB or IPA as an artifact (`mobile-release-android`, `mobile-release-ios`, 14 days), **nothing sent**.

Each job waits for the approval of the `mobile-release` environment, then:

1. **checks the secrets** (`apps/mobile/release/release.mjs check`): a missing secret stops the job before anything is built, its name in the message (`Missing secret APPLE_TEAM_ID in the environment mobile-release`); never a value;
2. computes the versions: the run number for Google (`versionCode`) and Apple (`CFBundleVersion`), the app's version without its pre-release for Apple (`2.3.0-beta.2` → `2.3.0`);
3. builds the phone's Rust library, the page, the native project;
4. runs fastlane (`apps/mobile/fastlane/Fastfile`, its version locked by `apps/mobile/Gemfile.lock`), which checks the secrets again:
   - **Android**: `android build` (the AAB signed by the upload key, checked by `jarsigner`) or `android closed_test` (then `upload_to_play_store` on `alpha`, without listing nor screenshots);
   - **iOS**: `ios build` (a throwaway keychain with a random password, the certificate and the profile, manual signing of the `App` target, the IPA) or `ios testflight_upload` (then `pilot` with the API key, without waiting for processing, without external distribution);
5. **removes, even on failure**, the files the secrets became, `google-services.json`, the keychain and the profile.

**Never** a submission to Apple's review, external testers, Google's production track, nor an upload of the listings: those are your gestures in the consoles.

Tried without secrets (2026-11-28, fastlane 2.242.0): every lane stops before doing anything, naming each missing secret.

## 5. Listings and screenshots

- **The texts**: `apps/mobile/store/listing/android/<locale>/` (`title.txt` ≤ 30, `short_description.txt` ≤ 80, `full_description.txt` ≤ 4,000, `release_notes.txt` ≤ 500) and `apps/mobile/store/listing/ios/<locale>/` (`name.txt` and `subtitle.txt` ≤ 30, `keywords.txt` ≤ 100, `promotional_text.txt` ≤ 170, `description.txt` ≤ 4,000, `release_notes.txt`, the privacy, support and marketing addresses), in `fr-FR` and `en-US`. A test (`apps/mobile/release/release.test.ts`) holds the limits. No seller name: the store shows the account's. The texts describe the app at its first release, MO-6 (push), MO-10 and MO-11 (creating) included.
- **The screenshots**, taken from the app on fictitious data (the tests' fake `claude`): `apps/mobile/store/screenshots/<size>/<locale>/`, by `apps/mobile/e2e/store-shots.ts`:

  ```sh
  cargo build --bin escouade-server        # in src-tauri/
  ESCOUADE_PORT_BASE=<base> npx playwright test -c apps/mobile/e2e/store-shots.config.ts
  ```

  | Folder | Size | For |
  |---|---|---|
  | `play` | 1,080 × 2,160 | Google Play, phone |
  | `appstore-6.9` | 1,290 × 2,796 | App Store, 6.9-inch iPhone |
  | `appstore-6.5` | 1,242 × 2,688 | App Store, 6.5-inch iPhone |

  Six screens per locale: the “Your turn” thread (dark and light), an authorization unfolded, a choice and its renders, the conversation, the agents, the Kanban. The agents' texts come from the fake `claude`, in French; the app's own follow the locale. The icon (512 × 512) and Google's feature graphic (1,024 × 500) remain to be made with the design.
- The lanes send no text nor screenshot: you paste them in the consoles (or a `supply`/`deliver` lane comes later, with your agreement).

## 6. The privacy policy

On the site: `https://escouade.dev/mobile/privacy/` and `https://escouade.dev/en/mobile/privacy/` (`website/content/mobile-privacy/`). What it says: the app talks only to the computers and servers you paired, through an encrypted tunnel; the publisher receives nothing but the sealed push, and only if you accept it; of Firebase, nothing but FCM on Android; no Firebase SDK on iOS; no statistics, no ads, no tracking.

## 7. The answers to prepare

### 7.1 Data safety (Google Play)

| Question | Proposed answer |
|---|---|
| Does the app collect or share any of the required user data types? | Yes |
| Is the data encrypted in transit? | Yes |
| Can users request that data be deleted? | Yes: revoking the phone from the computer, or removing the space in the app, erases its push registration |
| Types collected | **Device or other IDs** (the push token and Firebase's installation ID), for **app functionality** (notifications), optional (accepted at pairing), not shared, processed by a service provider (Google, on the publisher's behalf) |
| Messages, photos, files, audio | **Not collected**: they go from the phone to your own computer or server, through the end-to-end encrypted tunnel; the publisher never receives them. Dictation is the system's. |
| Location, contacts, health, financial, browsing, app activity, crash logs | Not collected |
| Ads | None |

To confirm on your side: Google defines collection as sending off the device. Check on its “Data safety” help page whether what only goes to the user's own server, end-to-end encrypted, is excluded. If in doubt, also declare “Messages” and “Photos”, for app functionality, not shared: more cautious, at no cost to the user.

### 7.2 The privacy label (App Store)

- **Data collected**: *Identifiers › Device ID* (the APNs token kept by the push relay, with a random pairing number), for *App Functionality*; **not linked to the user's identity**, **not used for tracking**.
- Everything else: not collected (the app talks only to your machines; speech recognition is iOS's; no analytics, no crash reports).
- Before MO-6 (no push): “Data Not Collected”.

### 7.3 Export compliance (App Store Connect)

`ITSAppUsesNonExemptEncryption` is **left out of `Info.plist`** until you decide: App Store Connect asks at each build (“Missing Compliance” in TestFlight until answered).

**What the app does**: TLS (by the system) and, **in the app itself** (the phone's Rust), WireGuard (Curve25519, ChaCha20-Poly1305, BLAKE2s, by boringtun) and the sealing of notifications: standard algorithms, for the confidentiality of the exchanges with the user's machines. So it is not only “the system's encryption”; in Apple's questionnaire it is “standard encryption algorithms in addition to those of the operating system”.

**What follows, to validate** (this is not legal advice):

- **United States (EAR)**: the code is free and published (GPL-3.0, public on GitHub); standard encryption software made publicly available is not subject to the EAR (§ 734.3(b)(3), § 742.15(b)); failing that, it is mass market (5D992.c). No CCATS to request.
- **France**: supplying a cryptology means that ensures confidentiality falls under a **declaration to ANSSI** (law 2004-575, art. 30; decree 2007-663), unless exempted. Apple asks, for an app distributed in France, to attach that declaration. This is the point to settle: file the declaration with ANSSI in the company's name, or check an exemption.

**Two ways**:

- **A (recommended)**: answer “standard algorithms”, file the ANSSI declaration and attach it in App Store Connect; once the compliance code is received, set `ITSAppUsesNonExemptEncryption` to `true` and `ITSEncryptionExportComplianceCode` to that code.
- **B**: leave France out of the distribution while the declaration is pending.

### 7.4 Apple's review: the note for guideline 4.2

To paste in App Store Connect › the version › *App Review Information* › Notes, at the first submission. Guideline 4.2 (“minimum functionality”) rejects an app that is only a repackaged website; the note shows what is native and gives the reviewer a way to try it.

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

The note assumes MO-6 merged (push and its extension); without it, remove the *Push notifications* line. The demo server for the review (an `escouade-server` on the trial VPS, with fictitious agents and a pairing code valid during the review) is set up at submission time; without it, the review risks a rejection under guideline 2.1 (the app cannot be tried).

## 8. What remains your gesture

1. Prepare the accounts and set the secrets (§ 1 to 3).
2. Run the workflow in “build only”, approve the environment, drop the first AAB in the closed test by hand.
3. Then run the workflow to upload (unchecked), and approve the environment, at each version.
4. Have the closed test's testers try the app, then apply for production at Google.
5. Settle export compliance (§ 7.3), fill in data safety and the privacy label (§ 7), paste the listings and the screenshots.
6. Set up the demo server and submit to Apple's review, with the note of § 7.4.

Timings: Google reviews the app before each first release on a track (from a few hours to a few days); at Apple, internal TestFlight is available once the build is processed, and the App Store release goes through review (usually one to two days).
