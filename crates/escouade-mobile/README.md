# escouade-mobile

The Rust of Escouade's mobile app (MO-3, [`docs/MOBILE.md`](../../docs/MOBILE.md) § 3.13):

- `tunnel`: the phone's WireGuard peer in user space (`boringtun`), a TCP/IP stack (`smoltcp`) and a local proxy on
  `127.0.0.1` through which the WebView's WebSocket reaches the core's remote API; the proxy forwards only a request
  whose path starts with the session's secret.
- `notification`: the sealed notifications (XChaCha20-Poly1305 under a key HKDF draws from the pairing key), their
  format (version 1), and the replay guard.
- `ffi` (feature `uniffi`): what the Capacitor plugin ([`apps/mobile-plugin`](../../apps/mobile-plugin)) sees through
  UniFFI's Kotlin and Swift bindings.

A package of its own (its `Cargo.lock`, its `rust-toolchain.toml`): the app's workspace does not hold it.

```sh
cargo nextest run --all-features          # the tests, on the machine (a WireGuard peer of test plays the core)
cargo clippy --all-targets --all-features -- -D warnings
ANDROID_NDK_HOME=<the NDK> bash ../../scripts/mobile-android.sh [--aar]   # Android's libraries, the Kotlin bindings
bash ../../scripts/mobile-ios.sh [--build]                                # macOS: the XCFramework, the Swift bindings
```

Never published; nothing it builds is signed.
