/**
 * The API the mobile app's Svelte interface sees of its Rust (`crates/escouade-mobile`, MO-3, `docs/MOBILE.md`):
 * the tunnel of each space and its local proxy, and the opening of the sealed notifications. The Kotlin
 * (`android/…/EscouadeMobilePlugin.kt`) and Swift (`ios/…/EscouadeMobilePlugin.swift`) sides call the crate through
 * the bindings UniFFI generates; `test/api.test.ts` checks the three sides name the same methods.
 *
 * A failure rejects the promise with `code` one of {@link EscouadeMobileErrorCode} and a message in English.
 */
export interface EscouadeMobilePlugin {
  /**
   * Starts the tunnel of a space (one per space; a tunnel already running for `space` is stopped first) and its
   * local proxy. Resolves at once with `state` `'connecting'`: the first handshake is on its way. Then
   * {@link tunnelStatus} says `'up'`, and the WebView opens `websocketUrl` (the remote API's WebSocket, through the
   * proxy and the tunnel).
   *
   * Rejects with `BadKey` (a key that is not 32 bytes of base64), `BadConfig` (an address, the API or the endpoint),
   * `Io` (the system refused a socket).
   */
  startTunnel(options: StartTunnelOptions): Promise<TunnelStatus>;

  /** Where the tunnel of `space` stands. Rejects with `NoTunnel` when none was started for it. */
  tunnelStatus(options: SpaceOptions): Promise<TunnelStatus>;

  /** Stops the tunnel of `space`: its proxy, its connections, its UDP socket. Resolves even when none runs. */
  stopTunnel(options: SpaceOptions): Promise<void>;

  /**
   * Opens a sealed notification with its pairing key, against the replay guard's state of that pairing (`''` or
   * left out for the first). Resolves with what it says and the guard's new state, which the app keeps for the next
   * one (it is not a secret). Rejects with `Forged` (altered, or sealed under another key), `Replayed` (opened
   * already), `TooOld` (below the guard's window), `Malformed`, `UnknownVersion`, `TooLarge`, `BadKey` or
   * `BadGuardState`; on a refusal, the state given stays the one to keep.
   */
  openNotification(options: OpenNotificationOptions): Promise<OpenedNotification>;

  /** The crate's version (`escouade-mobile 0.1.0`), for the app's « About ». */
  version(): Promise<{ version: string }>;
}

/** What the phone is given at pairing (MO-4's QR code) to reach one space. The keys are base64 (32 bytes). */
export interface TunnelConfig {
  /** The phone's WireGuard private key. */
  privateKey: string;
  /** The core's WireGuard public key. */
  peerPublicKey: string;
  /** WireGuard's optional preshared key. */
  presharedKey?: string;
  /** Where the core's peer listens, `host:port` (UDP). */
  endpoint: string;
  /** The phone's address in the tunnel, `10.77.0.2/24` (a bare address is a /32). */
  address: string;
  /** The remote API in the tunnel, `10.77.0.1:7878`. */
  api: string;
  /** WireGuard's persistent keepalive, in seconds: 25 when left out, 0 for none. */
  keepaliveSecs?: number;
}

export interface SpaceOptions {
  /** The space's id, as the app knows it (one tunnel per space). */
  space: string;
}

export interface StartTunnelOptions extends TunnelConfig, SpaceOptions {}

/**
 * - `connecting`: no handshake yet, or the last one expired (WireGuard tries again every 5 s);
 * - `up`: a handshake under 3 minutes old, the API is reachable;
 * - `stopped`: by {@link EscouadeMobilePlugin.stopTunnel};
 * - `failed`: the engine stopped on an error of the system (`error`).
 */
export type TunnelState = 'connecting' | 'up' | 'stopped' | 'failed';

export interface TunnelStatus {
  space: string;
  state: TunnelState;
  /**
   * The local proxy, its secret included: `http://127.0.0.1:<port>/<secret>`. The proxy forwards only a request
   * whose path starts with the secret, and takes it out (another app of the phone does not know it).
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
}

export interface OpenNotificationOptions {
  /** The pairing key (32 bytes, base64). */
  pairingKey: string;
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

/** The `code` of a rejected promise: the crate's error (`MobileError`), and `NoTunnel` from the plugin itself. */
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
  | 'NoTunnel';
