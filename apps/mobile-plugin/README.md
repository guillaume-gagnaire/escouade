# @escouade/capacitor-mobile

The Capacitor plugin of Escouade's mobile app (MO-3, [`docs/MOBILE.md`](../../docs/MOBILE.md) § 3.13): the phone's Rust
([`crates/escouade-mobile`](../../crates/escouade-mobile)) under Android (Kotlin) and iOS (Swift), through the bindings
UniFFI generates.

**The API** is typed and described in [`src/definitions.ts`](src/definitions.ts):

| Method | What it does |
| --- | --- |
| `startTunnel({ space, privateKey, peerPublicKey, presharedKey?, endpoint, address, api, keepaliveSecs? })` | Starts the tunnel of a space and its local proxy; resolves at once with its status (`connecting`) |
| `tunnelStatus({ space })` | `state` (`connecting`, `up`, `stopped`, `failed`), `proxyUrl`, `websocketUrl`, the last handshake, the bytes, the connections |
| `stopTunnel({ space })` | Stops it |
| `openNotification({ pairingKey, sealed, guardState? })` | Opens a sealed notification; resolves with what it says and the replay guard's new state |
| `version()` | The crate's version |

A refusal rejects with a `code`: `Forged`, `Replayed`, `TooOld`, `Malformed`, `UnknownVersion`, `TooLarge`, `BadKey`,
`BadConfig`, `BadGuardState`, `Io`, `NoTunnel`. `test/api.test.ts` (run by the root's Vitest) keeps TypeScript, Kotlin,
Swift and the crate in agreement.

**Building** (the libraries and the bindings are never committed):

```sh
ANDROID_NDK_HOME=<the NDK> bash scripts/mobile-android.sh --aar   # from the repository's root; Java 21, Gradle 8.14
bash scripts/mobile-ios.sh --build                                # under macOS, with Xcode
```

The CI's `mobile-android` and `mobile-ios` jobs do the same when the crate or the plugin changes. Nothing is signed nor
published.
