/**
 * The way the window speaks to the core (ESC-495, docs/REMOTE.md « La fondation 2b »): Tauri's IPC, or the WebSocket
 * of the remote API (`src-tauri/src/remoteapi`). `ipc.ts` and the other modules that call the core take `invoke` and
 * `Channel` from here, with Tauri's signatures: no component knows which one it runs on.
 *
 * - **Chosen at the start**: the WebSocket when the page was given a link to it (`window.__ESCOUADE_REMOTE__`, which a
 *   build of development puts there when `ESCOUADE_REMOTE_API` and `ESCOUADE_REMOTE_API_TOKEN` say so: the e2e on the
 *   remote API), else the IPC. No setting shows it yet.
 * - **The same calls**: the commands by their names, their arguments, results and errors as the IPC gives them; the
 *   channels, in JSON (the events of `subscribe`) and in bytes (a terminal's output); a raw body (`fs_write_bytes`,
 *   `attach_chunk`) in binary frames, an answer in bytes in parts. The errors the API names (`desk-only`,
 *   `not-served-yet`, `policy`, `token`, `version`…) come as texts of the window's language. The desktop's own
 *   commands (`desk-only`) go to this window's IPC, as the client does them at home.
 * - **After a cut**: the socket is opened again (delays growing, bounded), the handshake gives the session back, each
 *   terminal is resumed from the end of the last bytes it received (no hole, no byte twice), and the state is read
 *   again whole (`onStale`). A call left without its answer is failed, never sent again by itself: a command is not
 *   idempotent. A token refused or another version stop the tries, and say why (`connection`).
 */

import { Channel as TauriChannel, invoke as tauriInvoke } from '@tauri-apps/api/core';
import { connection } from './connection.svelte';
import { fSize } from './format';
import { t } from './i18n';

type Args = Record<string, unknown> | Uint8Array | ArrayBuffer | number[];
type Options = Parameters<typeof tauriInvoke>[2];

let nextChannel = 1;

/** A channel the core sends messages on, given as an argument of a call (`__CHANNEL__:<id>`, as Tauri's). */
export class Channel<T = unknown> {
  readonly id = nextChannel++;
  onmessage: (message: T) => void = () => {};

  toJSON(): string {
    return `__CHANNEL__:${this.id}`;
  }
}

/** What a page is given to speak to the core by its remote API: its address, the device's token, the app's version. */
export interface RemoteLink {
  url: string;
  token: string;
  version: string;
}

/** The socket the transport opens: the browser's `WebSocket`, or a test's. */
export interface Socket {
  binaryType: string;
  onopen: (() => void) | null;
  onmessage: ((e: { data: unknown }) => void) | null;
  onclose: ((e: { code: number; reason: string }) => void) | null;
  onerror: (() => void) | null;
  send(data: string | ArrayBuffer | Uint8Array): void;
  close(code?: number, reason?: string): void;
}

// ---------- Tauri's IPC ----------

/** The arguments for Tauri: each of our channels given as one of Tauri's, which hands its messages over. */
function forTauri(args?: Args): Args | undefined {
  if (!args || args instanceof ArrayBuffer || ArrayBuffer.isView(args) || Array.isArray(args)) return args;
  let out: Record<string, unknown> = args;
  for (const [key, value] of Object.entries(args)) {
    if (value instanceof Channel) {
      if (out === args) out = { ...args };
      const tauri = new TauriChannel<unknown>();
      tauri.onmessage = (m) => value.onmessage(m);
      out[key] = tauri;
    }
  }
  return out;
}

function ipcInvoke<T>(cmd: string, args?: Args, options?: Options): Promise<T> {
  return tauriInvoke<T>(cmd, forTauri(args) as Parameters<typeof tauriInvoke>[1], options);
}

// ---------- the remote API ----------

/** The kinds of the binary frames (`remoteapi::proto::Frame`). */
const FRAME = { channel: 1, reply: 2, term: 3, replyPart: 4, body: 5 } as const;
const HEAD = 9;
/** The largest message the core reads (`proto::MAX_MESSAGE`). */
const MAX_MESSAGE = 16 << 20;
/** The delays between two tries, growing to their bound. */
export const RETRY_DELAYS = [250, 500, 1000, 2000, 4000, 5000];
/** The refusals that stop the tries: nothing changes by trying again. */
const FINAL = new Set(['token', 'version', 'policy']);

/** The text the window shows for an error the remote API named, in its own language (the core's text else). */
export function namedError(code: string, command: string, fallback: unknown): unknown {
  switch (code) {
    case 'desk-only':
      return t('remote.error.deskOnly', { command });
    case 'not-served-yet':
      return t('remote.error.notServedYet', { command });
    case 'policy':
      return t('remote.error.policy');
    case 'token':
      return t('remote.error.token');
    case 'version':
      return typeof fallback === 'string' && fallback ? fallback : t('remote.error.version');
    case 'too-many-connections':
      return t('remote.error.tooMany');
    case 'busy':
      return t('remote.error.busy');
    case 'too-large':
      return t('remote.error.tooLarge');
    default:
      return fallback;
  }
}

interface Pending {
  cmd: string;
  resolve: (v: unknown) => void;
  reject: (e: unknown) => void;
  parts: Uint8Array[];
}

/** A channel of the transport, and where its terminal's output stands. */
interface Open {
  channel: Channel<unknown>;
  /** It carried events of the state (JSON): a new subscription takes its place after a cut, it is not resumed. */
  json: boolean;
  /** The end of the last numbered byte received. */
  last?: number;
  /** A resume asked, its replay not over: what comes past a hole waits for it. */
  resuming: boolean;
}

function bytesOf(data: unknown): Uint8Array | null {
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (ArrayBuffer.isView(data)) return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  return null;
}

/** `bytes` as a buffer of their own (what a channel of bytes hands over, as Tauri's does). */
function own(bytes: Uint8Array): ArrayBuffer {
  return bytes.slice().buffer;
}

export class RemoteTransport {
  private socket: Socket | null = null;
  private next = 1;
  private pending = new Map<number, Pending>();
  private channels = new Map<number, Open>();
  private session: string | null = null;
  private desk = new Set<string>();
  private ready = false;
  private everOpen = false;
  private stopped = false;
  private tries = 0;
  private waiting: (() => void)[] = [];
  private stale: (() => void)[] = [];
  private timer: ReturnType<typeof setTimeout> | null = null;
  private holdUntil = 0;

  constructor(
    private link: RemoteLink,
    private open: (url: string) => Socket = (url) => new WebSocket(url) as unknown as Socket,
    private local: <T>(cmd: string, args?: Args, options?: Options) => Promise<T> = ipcInvoke,
    private delays: number[] = RETRY_DELAYS,
  ) {
    connection.state = 'connecting';
    this.connect();
  }

  /** `f` is called when the state the window holds is to be read again whole: after a cut, or a queue that overflowed. */
  onStale(f: () => void): () => void {
    this.stale.push(f);
    return () => (this.stale = this.stale.filter((g) => g !== f));
  }

  /** The socket closed as a cut would (the e2e of the cut), the next try held off `holdMs` at least. */
  drop(holdMs = 0): void {
    this.holdUntil = Date.now() + holdMs;
    this.socket?.close(4000, 'dropped');
  }

  invoke<T>(cmd: string, args?: Args, options?: Options): Promise<T> {
    if (this.ready && this.desk.has(cmd)) return this.local<T>(cmd, args, options);
    if (!this.ready) {
      // Before the first connection, the call waits for it; after a cut, it fails at once (the band says why).
      if (this.everOpen || this.stopped) {
        return Promise.reject(this.stopped ? connection.reason : t('remote.error.offline', { command: cmd }));
      }
      return new Promise<void>((go) => this.waiting.push(go)).then(() => this.invoke<T>(cmd, args, options));
    }
    const id = this.next++;
    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, { cmd, resolve: resolve as (v: unknown) => void, reject, parts: [] });
      const call: Record<string, unknown> = { t: 'call', id, cmd };
      const body = bytesOf(args) ?? (Array.isArray(args) ? new Uint8Array(args) : null);
      if (body) {
        // The raw body first, in parts, then the call that says its length and its headers.
        const piece = MAX_MESSAGE - HEAD - 1;
        for (let at = 0; at < body.length; at += piece) this.send(frame(FRAME.body, id, body.subarray(at, at + piece)));
        call.args = {};
        call.body = body.length;
        call.headers = headersOf(options?.headers);
      } else {
        call.args = this.withChannels((args as Record<string, unknown>) ?? {});
      }
      this.send(JSON.stringify(call));
    });
  }

  /** The arguments, our channels given by their ids (`toJSON`), each one noted to hand its messages over. */
  private withChannels(args: Record<string, unknown>): Record<string, unknown> {
    for (const value of Object.values(args)) {
      if (value instanceof Channel && !this.channels.has(value.id)) {
        this.channels.set(value.id, { channel: value, json: false, resuming: false });
      }
    }
    return args;
  }

  private send(data: string | Uint8Array): void {
    this.socket?.send(data);
  }

  private connect(): void {
    this.timer = null;
    const socket = this.open(this.link.url);
    this.socket = socket;
    socket.binaryType = 'arraybuffer';
    let welcomed = false;
    let refusal: string | null = null;
    socket.onopen = () => {
      socket.send(JSON.stringify({ t: 'hello', version: this.link.version, token: this.link.token, session: this.session ?? undefined }));
    };
    socket.onmessage = (e) => {
      if (this.socket !== socket) return;
      const bytes = bytesOf(e.data);
      if (bytes) return this.binary(bytes);
      let m: Record<string, any>;
      try {
        m = JSON.parse(String(e.data));
      } catch {
        return;
      }
      if (!welcomed) {
        if (m.t === 'welcome') {
          welcomed = true;
          this.welcome(m);
        } else if (m.t === 'refused') {
          refusal = String(m.code ?? '');
          connection.reason = String(namedError(refusal, '', m.error ?? refusal));
        }
        return;
      }
      this.message(m);
    };
    socket.onerror = () => {};
    socket.onclose = () => {
      if (this.socket !== socket) return;
      this.socket = null;
      this.ready = false;
      // Every call left without its answer fails: never sent again (a command is not idempotent).
      for (const [, p] of this.pending) p.reject(t('remote.error.lost', { command: p.cmd }));
      this.pending.clear();
      for (const open of this.channels.values()) open.resuming = false;
      if (refusal && FINAL.has(refusal)) {
        this.stopped = true;
        connection.state = 'refused';
        for (const go of this.waiting.splice(0)) go();
        return;
      }
      connection.state = this.everOpen ? 'lost' : 'connecting';
      const delay = Math.max(this.delays[Math.min(this.tries, this.delays.length - 1)], this.holdUntil - Date.now());
      this.tries++;
      this.timer = setTimeout(() => this.connect(), delay);
    };
  }

  private welcome(m: Record<string, any>): void {
    const again = this.everOpen;
    this.session = typeof m.session === 'string' ? m.session : null;
    this.desk = new Set(Array.isArray(m.desk) ? m.desk : []);
    this.ready = true;
    this.everOpen = true;
    this.tries = 0;
    connection.state = 'open';
    connection.reason = '';
    // A server's secrets are in a file of its data, not in a keychain (ESC-514).
    connection.secretsInFile = m.secretsInFile === true;
    if (again) {
      // The terminals from where they stopped (the channels of the state are not resumed: it is read again).
      const channels = [...this.channels.entries()]
        .filter(([, open]) => !open.json)
        .map(([id, open]) => {
          open.resuming = true;
          return { channel: id, after: open.last ?? null };
        });
      if (channels.length) this.send(JSON.stringify({ t: 'resume', channels }));
      this.tellStale();
    }
    for (const go of this.waiting.splice(0)) go();
  }

  private tellStale(): void {
    for (const f of this.stale) {
      try {
        f();
      } catch (e) {
        console.error(e);
      }
    }
  }

  private message(m: Record<string, any>): void {
    switch (m.t) {
      case 'reply': {
        const p = this.pending.get(m.id);
        if (!p) return;
        this.pending.delete(m.id);
        if (m.error !== undefined) p.reject(m.code ? namedError(m.code, p.cmd, m.error) : m.error);
        else p.resolve(m.ok ?? null);
        return;
      }
      case 'channel': {
        const open = this.channels.get(m.channel);
        if (!open) return;
        open.json = true;
        open.channel.onmessage(m.data);
        return;
      }
      case 'mark': {
        const open = this.channels.get(m.channel);
        if (open) open.last = Number(m.end);
        return;
      }
      case 'lost': {
        const open = this.channels.get(m.channel);
        if (!open) return;
        const lost = Number(m.bytes);
        if (open.last !== undefined) open.last += lost;
        const text = `\r\n\x1b[2m${t('remote.term.lost', { size: fSize(lost) })}\x1b[0m\r\n`;
        open.channel.onmessage(own(new TextEncoder().encode(text)));
        return;
      }
      case 'resumed': {
        const open = this.channels.get(m.channel);
        if (!open) return;
        open.resuming = false;
        if (m.gone) this.channels.delete(m.channel);
        return;
      }
      case 'gap':
        return this.resume(m.channel);
      case 'resync':
        return this.tellStale();
      default:
        return;
    }
  }

  /** The terminal of `channel` asked again from the end of the last bytes received. */
  private resume(channel: number): void {
    const open = this.channels.get(channel);
    if (!open || open.resuming) return;
    open.resuming = true;
    this.send(JSON.stringify({ t: 'resume', channels: [{ channel, after: open.last ?? null }] }));
  }

  private binary(bytes: Uint8Array): void {
    if (bytes.length < HEAD) return;
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const kind = bytes[0];
    const id = Number(view.getBigUint64(1));
    const rest = bytes.subarray(HEAD);
    if (kind === FRAME.reply || kind === FRAME.replyPart) {
      const p = this.pending.get(id);
      if (!p) return;
      p.parts.push(rest.slice());
      if (kind === FRAME.replyPart) return;
      this.pending.delete(id);
      const size = p.parts.reduce((n, b) => n + b.length, 0);
      const all = new Uint8Array(size);
      let at = 0;
      for (const b of p.parts) {
        all.set(b, at);
        at += b.length;
      }
      p.resolve(all.buffer);
      return;
    }
    const open = this.channels.get(id);
    if (!open) return;
    if (kind === FRAME.channel) {
      open.channel.onmessage(own(rest));
      return;
    }
    if (kind !== FRAME.term || rest.length < 8) return;
    const end = Number(new DataView(rest.buffer, rest.byteOffset, 8).getBigUint64(0));
    const data = rest.subarray(8);
    const start = end - data.length;
    const last = open.last ?? start;
    if (end <= last) {
      // Had already: a resume overlapping what came.
      open.last = last;
      return;
    }
    if (start > last) {
      // A hole (the core dropped some for a slow reading): resumed from where it stands; nothing shown out of order.
      if (!open.resuming) this.resume(id);
      return;
    }
    open.last = end;
    open.channel.onmessage(own(data.subarray(last - start)));
  }
}

function frame(kind: number, id: number, bytes: Uint8Array): Uint8Array {
  const out = new Uint8Array(HEAD + bytes.length);
  out[0] = kind;
  new DataView(out.buffer).setBigUint64(1, BigInt(id));
  out.set(bytes, HEAD);
  return out;
}

function headersOf(headers?: HeadersInit): Record<string, string> {
  if (!headers) return {};
  if (typeof Headers !== 'undefined' && headers instanceof Headers) return Object.fromEntries(headers.entries());
  if (Array.isArray(headers)) return Object.fromEntries(headers);
  return { ...(headers as Record<string, string>) };
}

// ---------- the one this window runs on ----------

declare global {
  interface Window {
    __ESCOUADE_REMOTE__?: RemoteLink;
  }
}

const link = typeof window !== 'undefined' ? window.__ESCOUADE_REMOTE__ : undefined;
const remote = link ? new RemoteTransport(link) : null;

/** Calls the command `cmd` of the core, as Tauri's `invoke` does, on the transport of this window. */
export function invoke<T>(cmd: string, args?: Args, options?: Options): Promise<T> {
  return remote ? remote.invoke<T>(cmd, args, options) : ipcInvoke<T>(cmd, args, options);
}

/** `f` is called when the state the window holds is to be read again whole (never on the IPC, which loses nothing). */
export function onStale(f: () => void): () => void {
  return remote ? remote.onStale(f) : () => {};
}

/** Whether this window runs on the remote API (the e2e of the cut close its socket: `drop`). */
export const remoteTransport = remote;

// The e2e of the cut close the socket as a cut would.
if (remote) {
  (window as unknown as { __escouadeRemote?: { drop(holdMs?: number): void } }).__escouadeRemote = {
    drop: (holdMs?: number) => remote.drop(holdMs),
  };
}
