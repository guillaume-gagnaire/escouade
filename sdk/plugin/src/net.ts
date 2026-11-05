// The streams and the sockets of a plugin in WASM (ESC-332, https://escouade.dev/en/docs/plugins/api): `net/stream` read
// as an async iterable, its credit given back as each piece is taken, and `ws/*` the same way. Under Node (`run.command`),
// a plugin has its own network, and Escouade answers `methodNotFound`. `fetch` and `setTimeout` need nothing of this: in
// WASM they are the standard ones.
import type { Plugin } from './index.js';
import type { NetChunkParams, NetEndParams, NetStreamParams, WsClosedParams, WsMessageParams } from './protocol.js';

/** What comes in order, read by one reader. */
class Pipe<T> implements AsyncIterable<T> {
  private readonly pieces: T[] = [];
  private wake?: () => void;
  private done = false;
  private error?: Error;

  constructor(private readonly taken: (piece: T) => void = () => {}) {}

  push(piece: T) {
    this.pieces.push(piece);
    this.wake?.();
  }

  end(error?: Error) {
    this.done = true;
    this.error = error;
    this.wake?.();
  }

  async *[Symbol.asyncIterator](): AsyncGenerator<T> {
    for (;;) {
      if (this.pieces.length) {
        const piece = this.pieces.shift() as T;
        this.taken(piece);
        yield piece;
      } else if (this.error) throw this.error;
      else if (this.done) return;
      else await new Promise<void>((resolve) => (this.wake = resolve));
    }
  }
}

/** A piece of a stream: `data` when it is text, `dataBase64` when it is not. */
export type NetPiece = Pick<NetChunkParams, 'data' | 'dataBase64'>;

/** A response read as it comes. Each piece taken gives its bytes back to Escouade: a slow reader slows the server. */
export interface NetStream extends AsyncIterable<NetPiece> {
  readonly status: number;
  readonly headers: Record<string, string>;
  /** Lets go of it: its connection closes, and nothing more comes. */
  cancel(): void;
}

/** A message of a socket: `text`, or `dataBase64` for bytes. */
export type WsPiece = Pick<WsMessageParams, 'text' | 'dataBase64'>;

/** A WebSocket: its messages as they come, until it closes. */
export interface NetSocket extends AsyncIterable<WsPiece> {
  /** Refused past 1 MiB, or when what waits to be sent would pass 256 KiB. */
  send(message: WsPiece): Promise<void>;
  close(): Promise<void>;
  /** How it ended, whoever closed it. */
  readonly closed: Promise<Ended>;
}

type Ended = Pick<WsClosedParams, 'code' | 'reason'>;

interface Wires {
  streams: Map<number, Pipe<NetPiece>>;
  sockets: Map<number, { pipe: Pipe<WsPiece>; closed: (end: Ended) => void; ended: Promise<Ended> }>;
}

const wires = new WeakMap<Plugin, Wires>();

/** How many bytes a piece held. */
function sizeOf(piece: NetPiece) {
  if (piece.data !== undefined) return new TextEncoder().encode(piece.data).length;
  const b64 = piece.dataBase64 ?? '';
  return Math.floor((b64.length * 3) / 4) - (b64.endsWith('==') ? 2 : b64.endsWith('=') ? 1 : 0);
}

/** A piece may come before the answer that names its stream has been handed on: each side makes the pipe. */
function streamOf(p: Plugin, w: Wires, stream: number) {
  let pipe = w.streams.get(stream);
  if (!pipe) {
    pipe = new Pipe<NetPiece>((piece) => p.notify('net/credit', { stream, bytes: sizeOf(piece) }));
    w.streams.set(stream, pipe);
  }
  return pipe;
}

function socketOf(w: Wires, socket: number) {
  let s = w.sockets.get(socket);
  if (!s) {
    let closed: (end: Ended) => void = () => {};
    const ended = new Promise<Ended>((resolve) => (closed = resolve));
    s = { pipe: new Pipe<WsPiece>(), closed, ended };
    w.sockets.set(socket, s);
  }
  return s;
}

function wired(p: Plugin): Wires {
  let w = wires.get(p);
  if (w) return w;
  const own: Wires = { streams: new Map(), sockets: new Map() };
  wires.set(p, own);
  p.handle('net/chunk', (params) => {
    const { stream, data, dataBase64 } = params as NetChunkParams;
    streamOf(p, own, stream).push({ data, dataBase64 });
  });
  p.handle('net/end', (params) => {
    const { stream, error } = params as NetEndParams;
    streamOf(p, own, stream).end(error ? new Error(error) : undefined);
    own.streams.delete(stream);
  });
  p.handle('ws/message', (params) => {
    const { socket, text, dataBase64 } = params as WsMessageParams;
    socketOf(own, socket).pipe.push({ text, dataBase64 });
  });
  p.handle('ws/closed', (params) => {
    const { socket, code, reason } = params as WsClosedParams;
    const s = socketOf(own, socket);
    s.pipe.end();
    s.closed({ code, reason });
  });
  w = own;
  return w;
}

/** Opens `url` as a stream (`net/stream`): the rules of `fetch` at its opening; `sse: true` for one that may stay silent. */
export async function openStream(p: Plugin, url: string, init: Omit<NetStreamParams, 'url'> = {}): Promise<NetStream> {
  const w = wired(p);
  const { stream, status, headers } = await p.call('net/stream', { url, ...init });
  const pipe = streamOf(p, w, stream);
  return {
    status,
    headers,
    cancel() {
      p.notify('net/cancel', { stream });
      pipe.end();
      w.streams.delete(stream);
    },
    [Symbol.asyncIterator]: () => pipe[Symbol.asyncIterator](),
  };
}

/** Opens a WebSocket (`ws/open`): `wss` to a host of the plugin. Its headers never reach the journal. */
export async function openSocket(p: Plugin, url: string, headers?: Record<string, string>): Promise<NetSocket> {
  const w = wired(p);
  const { socket } = await p.call('ws/open', { url, ...(headers ? { headers } : {}) });
  const s = socketOf(w, socket);
  return {
    send: async (message) => void (await p.call('ws/send', { socket, ...message })),
    close: async () => void (await p.call('ws/close', { socket })),
    closed: s.ended,
    [Symbol.asyncIterator]: () => s.pipe[Symbol.asyncIterator](),
  };
}
