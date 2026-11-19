/**
 * What changes machines when the window is on a server (ESC-538, `docs/REMOTE.md` « Ce qui change de machine »,
 * `docs/SERVER-UX.md` § 4, S12, S13). On this computer, nothing here changes anything.
 *
 * - **The files of the agents** (S12): downloaded when the user opens one, never ahead, into
 *   `Téléchargements/Escouade/<espace>` by this computer's core (`received_save`), which reads their kind from their
 *   bytes and never opens a program or a script: it only downloads it, and shows it in its folder.
 * - **The ports of the server** (a test server of an agent, the return of a sign-in on `localhost`, S13): relayed to
 *   this computer's 127.0.0.1 through the SSH tunnel's host (`server_relay`); the address the window opens is the one
 *   of this computer.
 * - **What the server's host publishes** (`host` messages of the remote API): a chime, the taskbar's flash, a
 *   notification named by its space (« Travail · … »), the count of what waits, shown by this computer's core
 *   (`server_alert`); a click on the notification asks the server what it does (`host_notification_click`). Nothing a
 *   server sends is run or opened here without the user's gesture: what it asks to open is not opened.
 * - **The voice**: this computer's microphone, transcription and keys; its settings are this computer's
 *   (`local_voice_get`, `local_voice_set`), only the text goes to the server.
 */

import { connection, onServer } from './connection.svelte';
import { t } from './i18n';
import { Channel, invoke, local } from './transport';
import type { Settings, VoiceSettings } from './types';

/** The name of the space the window is on (« Travail »). */
export const spaceName = (): string => connection.server?.name ?? '';

// ---------- the files received (S12) ----------

/** A file received on this computer (`servers::received::Received`). */
export interface Received {
  path: string;
  folder: string;
  name: string;
  kind: 'program' | 'script' | 'image' | 'pdf' | 'file';
  runnable: boolean;
  opened: boolean;
}

/** The copies made here, by what they are a copy of (`<agent>|<item>|<index>`, `local|<agent>|<src>`). */
export const received = $state<Record<string, Received>>({});

/** What is downloading now, by the same keys. */
export const downloading = $state<Record<string, boolean>>({});

/**
 * The file `name` of the server (its bytes, read by `bytes`) kept on this computer, then opened (`open`: never a
 * program nor a script, shown in its folder instead), shown in its folder (`reveal`), or only kept (`keep`).
 */
export async function receive(
  key: string,
  name: string,
  bytes: () => Promise<ArrayBuffer>,
  mode: 'open' | 'reveal' | 'keep',
): Promise<Received> {
  downloading[key] = true;
  try {
    const body = new Uint8Array(await bytes());
    const headers: Record<string, string> = {
      'x-space': encodeURIComponent(spaceName()),
      'x-name': encodeURIComponent(name),
    };
    if (mode === 'open') headers['x-open'] = '1';
    if (mode === 'reveal') headers['x-reveal'] = '1';
    const got = await local<Received>('received_save', body, { headers });
    received[key] = got;
    return got;
  } finally {
    delete downloading[key];
  }
}

/** A copy made before, opened again (never a program nor a script) or shown in its folder. */
export function showReceived(r: Received, reveal: boolean): Promise<void> {
  return local<void>('received_show', { path: r.path, reveal });
}

/** What the window says of a copy made: where it went, and that a script was only downloaded. */
export function receivedText(r: Received): string {
  return r.runnable
    ? t('moves.files.scriptSaved', { name: r.name, folder: r.folder })
    : t('moves.files.saved', { name: r.name, folder: r.folder });
}

// ---------- the ports relayed ----------

/** A port of the server relayed to this computer (`servers::Relayed`). */
export interface Relayed {
  local: number;
  remote: number;
}

type Outcome<T> = { outcome: 'ok'; value: T } | { outcome: 'problem'; problem: { kind: string; message?: string; output?: string } };

const LOOPBACK = new Set(['localhost', '127.0.0.1', '[::1]', '::1', '0.0.0.0']);

/** The address names the machine itself (its loopback): on a server, the server's. */
export function isLoopback(address: string): boolean {
  try {
    const u = new URL(address);
    return ['http:', 'https:', 'ws:', 'wss:'].includes(u.protocol) && LOOPBACK.has(u.hostname.toLowerCase());
  } catch {
    return false;
  }
}

/** The ports of this computer that relay one of the server's: an address of theirs is this computer's already. */
const relayedHere = new Set<number>();

/** The port `port` of the server relayed here (`same`: only on that port); why not, in the window's words. */
export async function relay(port: number, same = false): Promise<Relayed> {
  const r = await local<Outcome<Relayed>>('server_relay', { port, same });
  if (r.outcome === 'ok') {
    if (r.value.local !== r.value.remote) relayedHere.add(r.value.local);
    return r.value;
  }
  throw r.problem.message ?? r.problem.output ?? r.problem.kind;
}

/**
 * `address` as this computer reaches it: on a server, an address of its loopback (a test server of an agent) relayed
 * to a port of this computer's, the same one when it is free; anything else as it is.
 */
export async function onThisComputer(address: string): Promise<string> {
  if (!onServer() || !isLoopback(address)) return address;
  const u = new URL(address);
  const port = Number(u.port) || (u.protocol === 'https:' || u.protocol === 'wss:' ? 443 : 80);
  if (relayedHere.has(port)) return address;
  const r = await relay(port);
  if (u.hostname === '0.0.0.0') u.hostname = '127.0.0.1';
  u.port = String(r.local);
  return u.toString();
}

// ---------- the sign-in of an account (S13) ----------

/** What a sign-in's terminal gave to open: the page, the port of `localhost` its return waits on, a code of a device. */
export interface LoginLink {
  url: string;
  /** The port of `localhost` the harness waits on for the page's return (relayed on the same port), if any. */
  port: number | null;
  /** A code of a device (GitHub's, Microsoft's): typed on the page, nothing comes back to a port. */
  device: boolean;
}

// eslint-disable-next-line no-control-regex
const ANSI = /\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(\x07|\x1b\\)/g;
const URLS = /https?:\/\/[^\s"'<>`\x1b]+/g;

/** The port of `localhost` in `address`, if it names one. */
function loopbackPort(address: string): number | null {
  try {
    const u = new URL(address);
    if (!LOOPBACK.has(u.hostname.toLowerCase())) return null;
    return Number(u.port) || (u.protocol === 'https:' ? 443 : 80);
  } catch {
    return null;
  }
}

/**
 * The page a sign-in's terminal gives to open (its output, `text`), and how its end comes back: to a port of
 * `localhost` (a parameter `redirect_uri` that names one, or the harness's « local login server » said before), a code of
 * a device, or a code to paste (none of these). Null until the terminal has given an address of the web.
 */
export function loginLink(text: string): LoginLink | null {
  const plain = text.replace(ANSI, '');
  const found = [...plain.matchAll(URLS)].map((m) => m[0].replace(/[.,;:)\]]+$/, ''));
  let listening: number | null = null;
  for (const url of found) {
    const port = loopbackPort(url);
    if (port !== null) {
      listening = port;
      continue;
    }
    let u: URL;
    try {
      u = new URL(url);
    } catch {
      continue;
    }
    let back: number | null = null;
    for (const key of ['redirect_uri', 'redirect_url', 'redirect', 'callback', 'callback_url']) {
      const v = u.searchParams.get(key);
      if (v) back = loopbackPort(v) ?? back;
    }
    const device = /\/(login\/)?device(login)?\b/i.test(u.pathname);
    return { url, port: back ?? (device ? null : listening), device };
  }
  return null;
}

// ---------- what the server's host publishes ----------

/** A `HostEvent` of the server (`host::HostEvent`). */
export type HostEvent =
  | { type: 'chime' }
  | { type: 'flash' }
  | { type: 'notify'; id: number; title: string; body: string }
  | { type: 'waiting'; count: number }
  | { type: 'open'; kind: string; target: string };

/** What the server's host published, shown on this computer, named by its space. */
export function showHostEvent(e: HostEvent): void {
  const clicked = new Channel<boolean>();
  switch (e.type) {
    case 'chime':
    case 'flash':
      void local('server_alert', { kind: e.type, clicked }).catch(() => {});
      return;
    case 'waiting':
      void local('server_alert', { kind: 'waiting', count: e.count, clicked }).catch(() => {});
      return;
    case 'notify':
      clicked.onmessage = () => void invoke('host_notification_click', { id: e.id }).catch(() => {});
      void local('server_alert', {
        kind: 'notify',
        title: t('moves.notify.title', { name: spaceName(), title: e.title }),
        body: e.body,
        clicked,
      }).catch(() => {});
      return;
    default:
      // What the server would open (a link, a file of its own) is never opened here without the user's gesture.
      return;
  }
}

// ---------- the voice, this computer's ----------

/** The voice's settings of this computer, read when the window is on a server. */
export const posteVoice = $state<{ value: VoiceSettings | null }>({ value: null });

/** Read once the window knows it is on a server. */
export async function loadPosteVoice(): Promise<void> {
  if (!onServer()) return;
  posteVoice.value = await local<VoiceSettings>('local_voice_get').catch(() => null);
}

/** `settings` with the voice of this computer in place of the server's, when the window is on one. */
export function withPosteVoice<S extends Pick<Settings, 'voice'>>(settings: S): S {
  return onServer() && posteVoice.value ? { ...settings, voice: posteVoice.value } : settings;
}

/** The voice's settings saved on this computer. */
export async function savePosteVoice(voice: VoiceSettings): Promise<void> {
  await local('local_voice_set', { voice });
  posteVoice.value = voice;
}

// ---------- a logo sent ----------

/** The bytes of a `data:` URL (the preview of a logo read on this computer). */
export function bytesOfDataUrl(url: string): Uint8Array {
  const b64 = url.slice(url.indexOf(',') + 1);
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** The last part of a path of this computer. */
export const baseName = (path: string): string => path.split(/[\\/]/).pop() ?? path;
