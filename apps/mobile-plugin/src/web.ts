import { WebPlugin } from '@capacitor/core';

import {
  QR_VERSION,
  type DictateOptions,
  type Dictated,
  type EscouadeMobilePlugin,
  type FinishPairingOptions,
  type LockState,
  type OpenFileOptions,
  type OpenedNotification,
  type PairingFound,
  type PairingKey,
  type ReadPairingOptions,
  type SpaceOptions,
  type TunnelStatus,
  type UnlockOptions,
} from './definitions';
import { b64, derive, unb64 } from './derive';

/**
 * The plugin in a browser (the e2e, `npm run dev`): no tunnel, no proxy, no Keystore. A space is reached at the
 * address typed (the e2e's forward to a local `escouade-server`); the pairing is the phone's (an X25519 key pair, the
 * token derived as the native side derives it, WebCrypto here), and the token is kept in the page's storage and given in
 * the handshake (`TunnelStatus.token`): what the native side never does. The screen lock is the test's:
 * `window.__ESCOUADE_LOCK__` is `'ok'` (by default), `'cancel'` (the sheet cancelled) or `'none'` (no screen lock).
 * The dictation is the test's too (`window.__ESCOUADE_DICTATE__`: what the system hears; none, no dictation), and a file
 * opened is downloaded by the browser, its name said in `window.__ESCOUADE_OPENED__`.
 */

const STORE = 'escouade.web-spaces';

interface Kept {
  kind: 'tunnel' | 'direct';
  api: string;
  token?: string;
  pairingKey?: string;
}

declare global {
  interface Window {
    __ESCOUADE_LOCK__?: 'ok' | 'cancel' | 'none';
    /** What the lock was asked, for the e2e (the reasons, in order). */
    __ESCOUADE_UNLOCKS__?: string[];
    /** What the system's dictation hears, for the e2e; none: this browser has no dictation. */
    __ESCOUADE_DICTATE__?: string;
    /** The files opened, for the e2e (their names, in order). */
    __ESCOUADE_OPENED__?: string[];
  }
}

function readAll(): Record<string, Kept> {
  try {
    const raw = localStorage.getItem(STORE);
    return raw ? (JSON.parse(raw) as Record<string, Kept>) : {};
  } catch {
    return {};
  }
}

function writeAll(all: Record<string, Kept>): void {
  try {
    localStorage.setItem(STORE, JSON.stringify(all));
  } catch {
    // A private window: the space lasts as long as the page.
  }
}

export class EscouadeMobileWeb extends WebPlugin implements EscouadeMobilePlugin {
  private pending = new Map<string, Kept>();
  private halves = new Map<string, CryptoKey>();

  private lock(): 'ok' | 'cancel' | 'none' {
    return (typeof window !== 'undefined' && window.__ESCOUADE_LOCK__) || 'ok';
  }

  async lockState(): Promise<LockState> {
    const none = this.lock() === 'none';
    return { screenLock: !none, biometry: none ? 'none' : 'face' };
  }

  async unlock(options: UnlockOptions): Promise<void> {
    (window.__ESCOUADE_UNLOCKS__ ??= []).push(options.reason);
    const lock = this.lock();
    if (lock === 'none') throw this.fail('no screen lock', 'NoLock');
    if (lock === 'cancel') throw this.fail('cancelled', 'Cancelled');
  }

  async readPairing(options: ReadPairingOptions): Promise<PairingFound> {
    if (options.qr !== undefined) {
      let read: Record<string, any>;
      try {
        read = JSON.parse(options.qr);
      } catch {
        throw this.fail('not a QR code of Escouade', 'BadPairing');
      }
      if (read?.escouade !== QR_VERSION || !read.tunnel) throw this.fail("not a QR code of Escouade's pairing", 'BadPairing');
      // A browser has no WireGuard: what the QR says is shown, its tunnel is not started.
      this.pending.set(options.space, { kind: 'tunnel', api: String(read.tunnel.api ?? '') });
      return {
        kind: 'tunnel',
        name: read.name,
        here: read.here === true,
        version: read.version,
        code: read.code,
        expiresAt: read.expiresAt,
        endpoint: read.tunnel.endpoint,
      };
    }
    if (!options.api?.trim()) throw this.fail('a QR code or an address', 'BadConfig');
    this.pending.set(options.space, { kind: 'direct', api: options.api.trim() });
    return { kind: 'direct', endpoint: options.api.trim() };
  }

  async pairingKey(options: SpaceOptions): Promise<PairingKey> {
    if (this.lock() === 'none') throw this.fail('no screen lock', 'NoLock');
    if (!this.pending.has(options.space)) throw this.fail('no pairing under way', 'NoSpace');
    const pair = (await crypto.subtle.generateKey({ name: 'X25519' }, false, ['deriveBits'])) as CryptoKeyPair;
    this.halves.set(options.space, pair.privateKey);
    return { publicKey: b64(await crypto.subtle.exportKey('raw', pair.publicKey)) };
  }

  async finishPairing(options: FinishPairingOptions): Promise<void> {
    const kept = this.pending.get(options.space);
    const half = this.halves.get(options.space);
    if (!kept || !half) throw this.fail('no pairing under way', 'NoSpace');
    let shared: ArrayBuffer;
    try {
      const core = await crypto.subtle.importKey('raw', unb64(options.coreKey), { name: 'X25519' }, false, []);
      shared = await crypto.subtle.deriveBits({ name: 'X25519', public: core }, half, 256);
    } catch {
      throw this.fail("the core's key is not one", 'BadKey');
    }
    const { token, key } = await derive(shared);
    const all = readAll();
    all[options.space] = { ...kept, token, pairingKey: key };
    writeAll(all);
    this.pending.delete(options.space);
    this.halves.delete(options.space);
  }

  async forgetSpace(options: SpaceOptions): Promise<void> {
    this.pending.delete(options.space);
    this.halves.delete(options.space);
    const all = readAll();
    delete all[options.space];
    writeAll(all);
  }

  async startTunnel(options: SpaceOptions): Promise<TunnelStatus> {
    return this.tunnelStatus(options);
  }

  async tunnelStatus(options: SpaceOptions): Promise<TunnelStatus> {
    const kept = readAll()[options.space] ?? this.pending.get(options.space);
    if (!kept) throw this.fail('nothing kept of this space', 'NoSpace');
    if (kept.kind !== 'direct') throw this.fail('no tunnel in a browser', 'NoTunnel');
    return {
      space: options.space,
      state: 'up',
      proxyUrl: `http://${kept.api}`,
      websocketUrl: `ws://${kept.api}/remote`,
      txBytes: 0,
      rxBytes: 0,
      connections: 0,
      token: kept.token,
    };
  }

  async stopTunnel(): Promise<void> {}

  async openNotification(): Promise<OpenedNotification> {
    throw this.unimplemented('the notifications are the phone’s');
  }

  async version(): Promise<{ version: string }> {
    return { version: 'escouade-mobile (web)' };
  }

  async dictate(_options: DictateOptions): Promise<Dictated> {
    const heard = typeof window !== 'undefined' ? window.__ESCOUADE_DICTATE__ : undefined;
    if (heard === undefined) throw this.fail('no dictation in this browser', 'NoDictation');
    if (!heard) throw this.fail('nothing heard', 'Cancelled');
    return { text: heard };
  }

  async stopDictation(): Promise<void> {}

  async openFile(options: OpenFileOptions): Promise<void> {
    (window.__ESCOUADE_OPENED__ ??= []).push(options.name);
    const bytes = unb64(options.data);
    const url = URL.createObjectURL(new Blob([bytes], { type: options.mediaType }));
    const a = Object.assign(document.createElement('a'), { href: url, download: options.name });
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 10_000);
  }

  private fail(message: string, code: string): Error {
    return Object.assign(new Error(message), { code });
  }
}
