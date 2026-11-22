# Escouade's mobile app

MO-1 of [`docs/MOBILE.md`](../../docs/MOBILE.md) (§ 3.14): the phone's app in **Capacitor 8** (Android, iOS), around
the app's Svelte interface. Direction B, « le fil » (`design/mobile-2-3/`).

- **Shared, never copied**: the page imports the app's interface where it is, as `$shared` (`src/` of the repository):
  its tokens (`app.css`), its icons, its texts in French and English (the `mobile` zone of `src/lib/i18n`), the plate
  of a space, and its transport to the core (`src/lib/transport.ts`, the WebSocket of the remote API). What exists on
  the phone alone lives here: its shell (`src/App.svelte`, `src/components/`), its spaces (`src/lib/phone-spaces.svelte.ts`,
  one `RemoteTransport` per space), its tabs and the system's back (`src/lib/nav.svelte.ts`), its appearance and
  Capacitor (`src/lib/appearance.ts`, `src/lib/native.ts`), its tokens (`src/mobile.css`).
- **The spaces**: MO-4 pairs them by QR code. Until then, a space is given to the page by the e2e
  (`window.__ESCOUADE_SPACES__`) or by `npm run dev:server`; the app on a phone opens on « Escouade sur ce téléphone ».
- **The native projects** (`android/`, `ios/`) are Capacitor's, made by `cap add`, then: R8 and the shrinking of the
  resources in release; no backup of the app's data; cleartext allowed to `127.0.0.1` alone (the tunnel's proxy of MO-3);
  portrait; iPhone only (M10). Capacitor by SwiftPM under iOS, no CocoaPods.

## Building

`npm ci` at the repository's root (Svelte, Vite and the fonts are its own), then here:

```sh
npm ci
npm run build                  # the page, in dist/
npx cap sync                   # the page into android/ and ios/
cd android && ./gradlew assembleRelease bundleRelease   # Java 21: the APK and the AAB, R8, unsigned
```

The iOS app for the simulator is built under macOS with Xcode (`xcodebuild … -sdk iphonesimulator`,
`CODE_SIGNING_ALLOWED=NO`). The CI's `mobile-app-android` and `mobile-app-ios` jobs do both when the app or `src/`
changes. Nothing is signed nor published: no account, no key (MO-8 does that).

## Trying it

```sh
cargo build --bin escouade-server                       # in src-tauri/
ESCOUADE_PORT_BASE=4170 npm run dev:server -- --demo    # the page on base + 1, on a local escouade-server
```

`--demo` makes a project and its agent stopped on a permission (the fake `claude` of the tests). The server's API
refuses a browser page's Origin on the loopback (it takes the WebView's through the phone's tunnel alone, MO-2): a
forward of the script stands for the tunnel and takes the page's Origin off.

## Tests

- **Vitest**, with the app's: `npx vitest run apps/mobile` from the root.
- **e2e**, at 360 × 780 (an Android) and 430 × 932 (an iPhone), in Chromium, against an `escouade-server` of each test
  (`e2e/harness.ts`): `npx playwright test -c apps/mobile/e2e/playwright.config.ts` from the root, after
  `cargo build --bin escouade-server`. `ESCOUADE_MOBILE_SHOTS=1` writes the captures in `design/mobile-2-3/app/`.
