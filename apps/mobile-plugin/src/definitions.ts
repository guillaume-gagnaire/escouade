/**
 * The API the mobile app's Svelte interface sees of its native side and its Rust (`crates/escouade-mobile`, MO-3 and
 * MO-4, `docs/MOBILE.md`): the pairing of a space, its keys kept by the system (Android Keystore, iOS Keychain), its
 * tunnel and local proxy, the screen lock and the biometric lock, the opening of the sealed notifications. The Kotlin
 * (`android/…/EscouadeMobilePlugin.kt`) and Swift (`ios/…/EscouadeMobilePlugin.swift`) sides call the crate through
 * the bindings UniFFI generates; `test/api.test.ts` checks the three sides name the same methods.
 *
 * **No secret reaches the JavaScript (SR1-15, ESC-570)**: the WireGuard key, the device's token and the pairing key
 * stay in the native side, which keeps them in the Keystore or the Keychain (readable after the first unlock, this
 * device only, never backed up). The page passes a space's id; the proxy writes the token in each request. The one
 * exception is the QR code's text, read by the camera and handed over at once, unread, to {@link
 * EscouadeMobilePlugin.readPairing}, before any content of an agent is shown.
 *
 * A failure rejects the promise with `code` one of {@link EscouadeMobileErrorCode} and a message in English.
 */
export interface EscouadeMobilePlugin {
  /** Whether the phone has a screen lock, and which biometry it offers. */
  lockState(): Promise<LockState>;

  /**
   * The system's lock (M3): the fingerprint or the face, the phone's code as a fallback; `reason` says what is about
   * to go. Rejects with `Cancelled` (cancelled or failed: nothing changes, the gesture is made again) or `NoLock`.
   */
  unlock(options: UnlockOptions): Promise<void>;

  /**
   * A pairing begun for `space`: the QR code's text as the camera read it (`qr`), or the address the user typed
   * (`api`, `host:port`, a space reached without the tunnel). Kept in memory until {@link finishPairing}. Resolves with
   * what the page shows (« Ce poste · DESKTOP-7QK2 · Escouade 2.3.0 ») and the code to give. Rejects with
   * `BadPairing` (no QR of Escouade's, or of another version) or `BadConfig`.
   */
  readPairing(options: ReadPairingOptions): Promise<PairingFound>;

  /**
   * The phone's half of the pairing of `space`: a key pair made here, its public half to give with the code. Rejects
   * with `NoLock` when the phone has no screen lock (no pairing without one), `NoSpace` when no pairing was begun.
   */
  pairingKey(options: SpaceOptions): Promise<PairingKey>;

  /**
   * The pairing of `space` ended: the device's token and the pairing key derived from the core's public half, kept
   * with the space's config in the Keystore or the Keychain. Rejects with `BadKey` or `NoSpace`.
   */
  finishPairing(options: FinishPairingOptions): Promise<void>;

  /** Everything of `space` forgotten: its tunnel stopped, its keys and its token removed. Resolves even when none. */
  forgetSpace(options: SpaceOptions): Promise<void>;

  /**
   * Starts the way to a space (its tunnel, or a proxy to its address; one per space, the one running stopped first),
   * from what is kept of it, or of its pairing under way. Resolves at once: the tunnel's first handshake is on its
   * way; {@link tunnelStatus} says `'up'`, and the WebView opens `websocketUrl`. Rejects with `NoSpace`, `BadKey`,
   * `BadConfig`, `Io`.
   */
  startTunnel(options: SpaceOptions): Promise<TunnelStatus>;

  /** Where the way to `space` stands. Rejects with `NoTunnel` when none was started for it. */
  tunnelStatus(options: SpaceOptions): Promise<TunnelStatus>;

  /** Stops the way to `space`: its proxy, its connections, its UDP socket. Resolves even when none runs. */
  stopTunnel(options: SpaceOptions): Promise<void>;

  /**
   * Opens a sealed notification of `space` with its pairing key, against the replay guard's state of that pairing
   * (`''` or left out for the first). Resolves with what it says and the guard's new state, which the app keeps for
   * the next one (it is not a secret). Rejects with `Forged` (altered, or sealed under another key), `Replayed`,
   * `TooOld`, `Malformed`, `UnknownVersion`, `TooLarge`, `BadKey`, `BadGuardState` or `NoSpace`; on a refusal, the
   * state given stays the one to keep.
   */
  openNotification(options: OpenNotificationOptions): Promise<OpenedNotification>;

  /** The crate's version (`escouade-mobile 0.1.0`), for the app's « About ». */
  version(): Promise<{ version: string }>;
}

/** The biometry the phone offers: `none` leaves its code alone. */
export type Biometry = 'face' | 'fingerprint' | 'none';

export interface LockState {
  /** A screen lock is set (a code, a pattern, a password): without it, no pairing. */
  screenLock: boolean;
  biometry: Biometry;
}

export interface UnlockOptions {
  /** What is about to go, as the system's sheet says it (« Autoriser la commande de tests-e2e ? · Escouade »). */
  reason: string;
}

export interface SpaceOptions {
  /** The space's id, as the app knows it (one tunnel per space). */
  space: string;
}

export interface ReadPairingOptions {
  space: string;
  /** The QR code's text, as the camera read it: handed over unread. */
  qr?: string;
  /** Or the address typed (`host:port`): a space reached without the tunnel. */
  api?: string;
}

/** How a space is reached: its WireGuard tunnel (a QR code), or its address (Tailscale's, a test's). */
export type SpaceKind = 'tunnel' | 'direct';

export interface PairingFound {
  kind: SpaceKind;
  /** The machine (`DESKTOP-7QK2`), for a QR code. */
  name?: string;
  /** This computer, not a server. */
  here?: boolean;
  /** Escouade's version on it. */
  version?: string;
  /** The pairing code a QR code holds (used once, at once). */
  code?: string;
  /** When the QR code ends (ms). */
  expiresAt?: number;
  /** Where the phone reaches it (`host:port`). */
  endpoint?: string;
}

export interface PairingKey {
  /** The phone's public half (X25519, base64): given with the code. */
  publicKey: string;
}

export interface FinishPairingOptions {
  space: string;
  /** The core's public half (`Paired.key`). */
  coreKey: string;
}

/**
 * - `connecting`: no handshake yet, or the last one expired (WireGuard tries again every 5 s);
 * - `up`: a handshake under 3 minutes old (or the address's proxy listens), the API is reachable;
 * - `stopped`: by {@link EscouadeMobilePlugin.stopTunnel};
 * - `failed`: the engine stopped on an error of the system (`error`).
 */
export type TunnelState = 'connecting' | 'up' | 'stopped' | 'failed';

export interface TunnelStatus {
  space: string;
  state: TunnelState;
  /**
   * The local proxy, its secret included: `http://127.0.0.1:<port>/<secret>`. The proxy forwards only a request
   * whose path starts with the secret, takes it out (another app of the phone does not know it) and writes the
   * device's token.
   */
  proxyUrl: string;
  /** The remote API's WebSocket through the proxy: `ws://127.0.0.1:<port>/<secret>/remote`. */
  websocketUrl: string;
  /** Seconds since the last handshake, when there is a session. */
  lastHandshakeSecs?: number;
  /** Bytes of data sent and received in the tunnel. */
  txBytes: number;
  rxBytes: number;
  /** Connections to the proxy open now. */
  connections: number;
  /** The error that stopped the engine, when `state` is `failed`. */
  error?: string;
  /** In a browser alone (the e2e, `npm run dev`), which has no proxy: the token the page gives in its handshake. */
  token?: string;
}

export interface OpenNotificationOptions {
  space: string;
  /** The sealed text, as the push brought it. */
  sealed: string;
  /** The replay guard's state kept from the last notification of this pairing; `''` or left out for the first. */
  guardState?: string;
}

export interface OpenedNotification {
  /** From 1, increasing for each pairing. */
  seq: number;
  /** When the core sealed it, in Unix seconds. */
  at: number;
  /** What it is about (`permission`, `question`, `inbox`…). */
  kind: string;
  title: string;
  body: string;
  /** What touching it reopens, when there is one. */
  target?: string;
  /** The replay guard's new state: keep it for the next notification of this pairing. */
  guardState: string;
}

/**
 * The `code` of a rejected promise: the crate's error (`MobileError`), then the plugin's own: `NoTunnel`, `NoSpace`
 * (nothing kept of that space), `NoLock` (no screen lock), `Cancelled` (the lock cancelled or failed), `BadPairing`
 * (not a QR code of Escouade's pairing).
 */
export type EscouadeMobileErrorCode =
  | 'BadKey'
  | 'BadConfig'
  | 'Io'
  | 'TooLarge'
  | 'Malformed'
  | 'UnknownVersion'
  | 'Forged'
  | 'Replayed'
  | 'TooOld'
  | 'BadGuardState'
  | 'NoTunnel'
  | 'NoSpace'
  | 'NoLock'
  | 'Cancelled'
  | 'BadPairing';

/** The QR code's marker and version (`src-tauri/src/phone/mod.rs`, `QR_VERSION`). */
export const QR_VERSION = 'pair/1';
