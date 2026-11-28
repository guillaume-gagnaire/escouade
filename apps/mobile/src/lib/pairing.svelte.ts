/**
 * The pairing of a space (MO-4, ESC-570, `docs/MOBILE.md` § 3.1, § 3.9 and § 3.10), and its end.
 *
 * - **Begin**: the QR code read by the camera (`@capacitor/barcode-scanner`), its text handed over unread to the native
 *   side (`readPairing`); or the 8-character code typed, with the address of a space reached without the tunnel (its
 *   address on Tailscale). The app says what it found (« Ce poste · DESKTOP-7QK2 · Escouade 2.3.0 ») and proposes a
 *   name for this phone.
 * - **No pairing without a screen lock** (M3): refused before anything, with its screen.
 * - **Pair**: the native side makes a key pair; the app opens the space's way (its tunnel, or a proxy to its address)
 *   and gives the code with the public half (`{ t: 'pair', code, name, key }`); the core answers its own public half
 *   (`paired`), and the native side derives the device's token and keeps it with the keys. The token never comes here.
 * - **Each failure its words**: the code expired, used already or never asked, wrong, too many tries; the tunnel that
 *   does not come up; another version; the policy of the machine.
 * - **The end of a space**: « Retirer » asks the core to revoke the phone (`unpair`), then forgets its keys and token;
 *   a space that revoked the phone has its own screen (« Ce téléphone n'est plus appairé à Travail »), with « Appairer
 *   de nouveau » and « Retirer Travail de ce téléphone ».
 */

import { EscouadeMobile, type EscouadeMobilePlugin, type PairingFound } from '@escouade/capacitor-mobile';
import { t } from '$shared/lib/i18n';
import type { Socket } from '$shared/lib/transport';
import { forgetKept } from './kept';
import { lockState } from './lock';
import { platformOf } from './native';
import { dropSpace, keepSpace, type KeptSpace } from './paired';

export type Step = 'start' | 'nolock' | 'found' | 'pairing' | 'failed';

export const pairing = $state<{
  /** The pairing screen is shown above the shell (a space added); always with no space. */
  open: boolean;
  step: Step;
  found: PairingFound | null;
  /** The name this phone takes on the space. */
  name: string;
  /** What went wrong, in the user's words. */
  error: string;
  /** What to do then: scan again, or try again. */
  again: 'scan' | 'retry' | null;
}>({ open: false, step: 'start', found: null, name: '', error: '', again: null });

declare global {
  interface Window {
    /** A test's camera: the text it reads in place of the scanner (the e2e). */
    __ESCOUADE_SCAN__?: string;
  }
}

/** What the pairing goes through: the native side, the camera, the sockets; a test's in their place. */
export interface PairingDeps {
  plugin: Pick<
    EscouadeMobilePlugin,
    'readPairing' | 'pairingKey' | 'finishPairing' | 'startTunnel' | 'tunnelStatus' | 'stopTunnel' | 'forgetSpace'
  >;
  scan: () => Promise<string>;
  open: (url: string) => Socket;
  /** How long the tunnel may take to come up, and an answer to come. */
  waitMs: number;
}

const defaults: PairingDeps = {
  plugin: EscouadeMobile,
  scan: async () => {
    if (typeof window !== 'undefined' && typeof window.__ESCOUADE_SCAN__ === 'string') return window.__ESCOUADE_SCAN__;
    // Loaded when the camera is asked: its web side (html5-qrcode) weighs on the page otherwise.
    const { CapacitorBarcodeScanner, CapacitorBarcodeScannerTypeHint } = await import('@capacitor/barcode-scanner');
    const r = await CapacitorBarcodeScanner.scanBarcode({ hint: CapacitorBarcodeScannerTypeHint.QR_CODE });
    return r.ScanResult;
  },
  open: (url) => new WebSocket(url) as unknown as Socket,
  waitMs: 20_000,
};

let deps = defaults;

/** A test's in place of the app's (none: the app's again). */
export function usePairingDeps(over: Partial<PairingDeps> | null): void {
  deps = over ? { ...defaults, ...over } : defaults;
}

/** What the app does once a space is paired or removed: its connections opened again. */
let changed: () => void = () => {};
export function onSpacesChanged(f: () => void): void {
  changed = f;
}

/** The space being paired, its code, and the address typed. */
let space = '';
let code = '';
let typed = '';

const newId = (): string => `s-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`;

/** The pairing screen opened again at its start (« Appairer un espace », « Appairer de nouveau »). */
export function openPairing(): void {
  Object.assign(pairing, { open: true, step: 'start', found: null, error: '', again: null });
}

export function closePairing(): void {
  pairing.open = false;
  pairing.step = 'start';
  if (space) void deps.plugin.forgetSpace({ space }).catch(() => {});
  space = '';
}

function fail(error: string, again: 'scan' | 'retry' | null = 'retry'): void {
  Object.assign(pairing, { step: 'failed', error, again });
}

/** A name for this phone as the space will show it. */
const defaultName = (): string => (platformOf() === 'android' ? t('mobile.pair.androidName') : t('mobile.pair.iosName'));

async function guardLock(): Promise<boolean> {
  if ((await lockState()).screenLock) return true;
  Object.assign(pairing, { step: 'nolock', error: '', again: null });
  return false;
}

/** The QR code read by the camera, its text handed over unread. */
export async function scanQr(): Promise<void> {
  if (!(await guardLock())) return;
  let text: string;
  try {
    text = await deps.scan();
  } catch {
    // The camera closed, or refused: the screen stays as it was.
    return;
  }
  space = newId();
  try {
    const found = await deps.plugin.readPairing({ space, qr: text });
    code = found.code ?? '';
    typed = '';
    if (found.expiresAt && found.expiresAt <= Date.now()) return fail(t('mobile.pair.error.expired'), 'scan');
    Object.assign(pairing, { step: 'found', found, name: defaultName(), error: '', again: null });
  } catch {
    fail(t('mobile.pair.error.notQr'), 'scan');
  }
}

/** The code typed, with the address of a space reached without the tunnel. */
export async function typeCode(typedCode: string, address: string): Promise<void> {
  if (!(await guardLock())) return;
  const letters = typedCode.replace(/[^0-9a-z]/gi, '').toUpperCase();
  if (letters.length !== 8) return fail(t('mobile.pair.error.codeLength'), null);
  const api = address
    .trim()
    .replace(/^wss?:\/\//, '')
    .replace(/\/.*$/, '');
  if (!/^[\w.-]+(:\d+)?$|^\[[0-9a-f:]+\](:\d+)?$/i.test(api)) return fail(t('mobile.pair.error.address'), null);
  space = newId();
  try {
    const found = await deps.plugin.readPairing({ space, api });
    code = `${letters.slice(0, 4)}-${letters.slice(4)}`;
    typed = api;
    Object.assign(pairing, { step: 'found', found, name: defaultName(), error: '', again: null });
  } catch {
    fail(t('mobile.pair.error.address'), null);
  }
}

/** How a space shows itself: « Ce poste », a server's machine, or the address typed. */
export function nameOf(found: PairingFound | null): string {
  if (!found) return '';
  if (found.kind === 'direct') return (found.endpoint ?? '').replace(/:\d+$/, '');
  return found.here ? t('mobile.pair.here') : (found.name ?? '');
}

/** What a refusal says to the user. */
export function refusalText(m: Record<string, any>): string {
  switch (m.code) {
    case 'pairing':
      switch (m.reason) {
        case 'expired':
          return t('mobile.pair.error.expired');
        case 'none':
          return t('mobile.pair.error.used');
        case 'wrong':
          return t('mobile.pair.error.wrong');
        case 'wait':
          return t('mobile.pair.error.wait');
        default:
          return String(m.error ?? t('mobile.pair.error.other'));
      }
    case 'version':
      return m.update === 'server' ? t('mobile.pair.error.updateServer') : t('mobile.pair.error.updateApp');
    case 'policy':
      return t('mobile.pair.error.policy');
    default:
      return String(m.error ?? t('mobile.pair.error.other'));
  }
}

/** The way to `id` open: its tunnel up (or its address's proxy), within `waitMs`. */
async function wayUp(id: string): Promise<{ url: string; token?: string } | null> {
  let s = await deps.plugin.startTunnel({ space: id });
  const until = Date.now() + deps.waitMs;
  while (s.state === 'connecting' && Date.now() < until) {
    await new Promise((r) => setTimeout(r, 250));
    s = await deps.plugin.tunnelStatus({ space: id });
  }
  return s.state === 'up' ? { url: s.websocketUrl, token: s.token } : null;
}

/** One message sent on a new socket to `url`, and its first answer (null: none in time, or closed). */
function exchange(url: string, message: Record<string, unknown>): Promise<Record<string, any> | null> {
  return new Promise((resolve) => {
    let socket: Socket;
    try {
      socket = deps.open(url);
    } catch {
      return resolve(null);
    }
    const timer = setTimeout(() => done(null), deps.waitMs);
    let over = false;
    const done = (m: Record<string, any> | null) => {
      if (over) return;
      over = true;
      clearTimeout(timer);
      resolve(m);
      try {
        socket.close(1000, 'done');
      } catch {
        // Closed already.
      }
    };
    socket.onopen = () => socket.send(JSON.stringify(message));
    socket.onmessage = (e) => {
      try {
        done(JSON.parse(String(e.data)));
      } catch {
        done(null);
      }
    };
    socket.onerror = () => {};
    socket.onclose = () => done(null);
  });
}

/** « Appairer ce téléphone »: the code given with the phone's key, the token derived by the native side. */
export async function pairNow(): Promise<KeptSpace | null> {
  const found = pairing.found;
  if (!found || !space) return null;
  const name = pairing.name.trim() || defaultName();
  pairing.step = 'pairing';
  let key: string;
  try {
    key = (await deps.plugin.pairingKey({ space })).publicKey;
  } catch (e) {
    if ((e as { code?: string })?.code === 'NoLock') {
      Object.assign(pairing, { step: 'nolock' });
      return null;
    }
    fail(t('mobile.pair.error.other'));
    return null;
  }
  let way: { url: string } | null = null;
  try {
    way = await wayUp(space);
  } catch {
    way = null;
  }
  if (!way) {
    fail(
      found.kind === 'tunnel'
        ? t('mobile.pair.error.tunnel', { endpoint: found.endpoint ?? '' })
        : t('mobile.pair.error.unreachable', { address: typed }),
    );
    return null;
  }
  const answer = await exchange(way.url, { t: 'pair', code, name, key });
  if (!answer) {
    fail(t('mobile.pair.error.unreachable', { address: found.endpoint ?? typed }));
    return null;
  }
  if (answer.t === 'refused') {
    fail(refusalText(answer), answer.reason === 'expired' || answer.reason === 'none' ? 'scan' : 'retry');
    return null;
  }
  if (answer.t !== 'paired' || typeof answer.key !== 'string') {
    // A core of before ESC-570 answers the token itself: refused, it would cross the page.
    fail(t('mobile.pair.error.updateServer'));
    return null;
  }
  try {
    await deps.plugin.finishPairing({ space, coreKey: answer.key });
  } catch {
    fail(t('mobile.pair.error.other'));
    return null;
  }
  const kept: KeptSpace = { id: space, name: nameOf(found), here: found.here === true, kind: found.kind };
  keepSpace(kept);
  space = '';
  code = '';
  Object.assign(pairing, { open: false, step: 'start', found: null, error: '', again: null });
  changed();
  return kept;
}

/**
 * The space `id` removed from this phone: when `unpair`, the core asked to revoke it first (« Retirer », S15), as far
 * as it answers; then its keys and token forgotten, whatever the core said, and what the phone kept of it (M13).
 */
export async function removeSpace(id: string, unpair: boolean): Promise<void> {
  if (unpair) {
    try {
      const way = await wayUp(id);
      // The token is the proxy's to write; in a browser alone, the page gives it.
      if (way) await exchange(way.url, { t: 'unpair', token: way.token ?? '' });
    } catch {
      // Unreachable: forgotten here all the same; « Révoquer » on the computer ends it there.
    }
  }
  await deps.plugin.stopTunnel({ space: id }).catch(() => {});
  await deps.plugin.forgetSpace({ space: id }).catch(() => {});
  dropSpace(id);
  forgetKept(id);
  changed();
}
