// The window on the remote API (ESC-495): the transport against a fake server, its calls, errors, channels, raw bodies,
// and its resume after a cut.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { connection } from './connection.svelte';
import { setLang } from './i18n';
import { Channel, RemoteTransport, namedError, type Socket } from './transport';

/** A socket of the test: what the transport sent, and what the server answers. */
class FakeSocket implements Socket {
  binaryType = 'blob';
  onopen: (() => void) | null = null;
  onmessage: ((e: { data: unknown }) => void) | null = null;
  onclose: ((e: { code: number; reason: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  sent: (string | Uint8Array)[] = [];
  closed = false;
  constructor(public url: string) {}
  send(data: string | ArrayBuffer | Uint8Array) {
    this.sent.push(typeof data === 'string' ? data : new Uint8Array(data as ArrayBuffer).slice());
  }
  close(code = 1000, reason = '') {
    if (this.closed) return;
    this.closed = true;
    this.onclose?.({ code, reason });
  }
  /** The server's message, in JSON. */
  json(m: unknown) {
    this.onmessage?.({ data: JSON.stringify(m) });
  }
  /** The server's binary frame. */
  binary(bytes: Uint8Array) {
    this.onmessage?.({ data: bytes.slice().buffer });
  }
  /** What the transport sent in JSON. */
  texts(): Record<string, any>[] {
    return this.sent.filter((m): m is string => typeof m === 'string').map((m) => JSON.parse(m));
  }
  calls(): Record<string, any>[] {
    return this.texts().filter((m) => m.t === 'call');
  }
}

const enc = new TextEncoder();
const dec = new TextDecoder();

function head(kind: number, id: number, rest: Uint8Array): Uint8Array {
  const out = new Uint8Array(9 + rest.length);
  out[0] = kind;
  new DataView(out.buffer).setBigUint64(1, BigInt(id));
  out.set(rest, 9);
  return out;
}

/** A terminal's output on `channel`, its bytes ending at `end` (`proto::term_frame`). */
function term(channel: number, end: number, text: string): Uint8Array {
  const data = enc.encode(text);
  const rest = new Uint8Array(8 + data.length);
  new DataView(rest.buffer).setBigUint64(0, BigInt(end));
  rest.set(data, 8);
  return head(3, channel, rest);
}

const tick = () => new Promise((r) => setTimeout(r, 0));

describe('the transport on the remote API', () => {
  let sockets: FakeSocket[];
  let local: ReturnType<typeof vi.fn>;
  let transport: RemoteTransport;

  const socket = () => sockets.at(-1)!;
  /** The handshake of the last socket passed. */
  const welcome = (session = 's1') => {
    socket().onopen?.();
    socket().json({ t: 'welcome', version: '2.3.0', client: 'c1', device: 'test', session, resumed: false, desk: ['open_url'] });
  };

  beforeEach(() => {
    setLang('fr');
    sockets = [];
    local = vi.fn(async () => 'local');
    transport = new RemoteTransport(
      { url: 'ws://127.0.0.1:4210/remote', token: 'secret', version: '2.3.0' },
      (url) => {
        const s = new FakeSocket(url);
        sockets.push(s);
        return s;
      },
      local as never,
      [5, 10],
    );
  });
  afterEach(() => {
    for (const s of sockets) s.onclose = null;
  });

  it('says its version, its token and its session first, and holds the calls until the handshake passed', async () => {
    const answer = transport.invoke<number>('closed_projects');
    socket().onopen?.();
    expect(socket().texts()).toEqual([{ t: 'hello', version: '2.3.0', token: 'secret' }]);
    expect(socket().calls()).toEqual([]);
    expect(connection.state).toBe('connecting');
    socket().json({ t: 'welcome', session: 's1', desk: [] });
    await tick();
    const [call] = socket().calls();
    expect(call).toMatchObject({ cmd: 'closed_projects', args: {} });
    socket().json({ t: 'reply', id: call.id, ok: 42 });
    await expect(answer).resolves.toBe(42);
    expect(connection.state).toBe('open');
  });

  it('reads in the handshake whether the core keeps its secrets in a file (a server, ESC-514)', async () => {
    socket().onopen?.();
    socket().json({ t: 'welcome', session: 's1', desk: [], secretsInFile: true });
    await tick();
    expect(connection.secretsInFile).toBe(true);
    socket().close(1006, 'cut');
    await new Promise((r) => setTimeout(r, 20));
    socket().onopen?.();
    socket().json({ t: 'welcome', session: 's1', desk: [] });
    await tick();
    expect(connection.secretsInFile).toBe(false);
  });

  it('gives the results and the errors as the IPC does, and the named ones in the window’s language', async () => {
    welcome();
    const ok = transport.invoke('set_ui', { ui: {} });
    const failed = transport.invoke('create_agent', { projectId: 'p' });
    const desk = transport.invoke('voice_start');
    const notYet = transport.invoke('lsp_start');
    const [a, b, c, d] = socket().calls();
    socket().json({ t: 'reply', id: a.id, ok: null });
    socket().json({ t: 'reply', id: b.id, error: 'Projet introuvable' });
    socket().json({ t: 'reply', id: c.id, error: '“voice_start” is a command of the desktop', code: 'desk-only' });
    socket().json({ t: 'reply', id: d.id, error: 'not served', code: 'not-served-yet' });
    await expect(ok).resolves.toBeNull();
    await expect(failed).rejects.toBe('Projet introuvable');
    await expect(desk).rejects.toBe('« voice_start » se fait sur ce poste, pas à distance.');
    await expect(notYet).rejects.toBe('« lsp_start » n’est pas encore servie à distance.');
    setLang('en');
    expect(namedError('token', 'x', '')).toBe('The token of this device is missing, unknown or revoked.');
  });

  it('does the desktop’s own commands on this window’s IPC', async () => {
    welcome();
    await expect(transport.invoke('open_url', { url: 'https://escouade.dev' })).resolves.toBe('local');
    expect(local).toHaveBeenCalledWith('open_url', { url: 'https://escouade.dev' }, undefined);
    expect(socket().calls()).toEqual([]);
  });

  it('hands a channel its JSON messages and a terminal its numbered bytes once, in order', async () => {
    welcome();
    const events: unknown[] = [];
    const events$ = new Channel<unknown>();
    events$.onmessage = (e) => events.push(e);
    const out: string[] = [];
    const output = new Channel<ArrayBuffer>();
    output.onmessage = (d) => out.push(dec.decode(d));
    void transport.invoke('subscribe', { channel: events$ });
    void transport.invoke('term_spawn', { projectId: 'p', output });
    expect(socket().calls()[1].args.output).toBe(`__CHANNEL__:${output.id}`);
    socket().json({ t: 'channel', channel: events$.id, data: { type: 'teams' } });
    socket().binary(term(output.id, 3, 'abc'));
    socket().binary(term(output.id, 6, 'def'));
    // A replay that overlaps: nothing twice.
    socket().binary(term(output.id, 6, 'cdef'));
    socket().binary(term(output.id, 8, 'efgh'));
    expect(events).toEqual([{ type: 'teams' }]);
    expect(out.join('')).toBe('abcdefgh');
    // A hole (the core dropped some for a slow reading): resumed from where it stands, nothing shown past it.
    socket().binary(term(output.id, 20, 'xyz'));
    expect(out.join('')).toBe('abcdefgh');
    expect(socket().texts().at(-1)).toEqual({ t: 'resume', channels: [{ channel: output.id, after: 8 }] });
    socket().binary(term(output.id, 17, 'ijklmnopq'));
    socket().json({ t: 'resumed', channel: output.id });
    socket().binary(term(output.id, 20, 'xyz'));
    expect(out.join('')).toBe('abcdefghijklmnopqxyz');
    // The core says a gap of its own: the same resume.
    socket().json({ t: 'gap', channel: output.id });
    expect(socket().texts().at(-1)).toEqual({ t: 'resume', channels: [{ channel: output.id, after: 20 }] });
  });

  it('shows a replay of a screen, then numbers from its mark, and says what a cut lost', async () => {
    welcome();
    const out: string[] = [];
    const output = new Channel<ArrayBuffer>();
    output.onmessage = (d) => out.push(dec.decode(d));
    void transport.invoke('agent_terminal', { id: 'a1', output });
    socket().binary(head(1, output.id, enc.encode('screen')));
    socket().json({ t: 'mark', channel: output.id, end: 1000 });
    socket().binary(term(output.id, 1000, 'old'));
    socket().binary(term(output.id, 1004, 'next'));
    expect(out.join('')).toBe('screennext');
    socket().json({ t: 'lost', channel: output.id, bytes: 2048 });
    expect(out.at(-1)).toContain('perdus pendant la coupure');
    socket().binary(term(output.id, 3057, 'later'));
    expect(out.at(-1)).toBe('later');
  });

  it('after a cut: fails the call without its answer, never sends it again, comes back in its session and resumes', async () => {
    vi.useFakeTimers();
    try {
      welcome('s-42');
      const out: string[] = [];
      const output = new Channel<ArrayBuffer>();
      output.onmessage = (d) => out.push(dec.decode(d));
      transport.invoke('term_spawn', { projectId: 'p', output }).catch(() => {});
      socket().binary(term(output.id, 5, 'hello'));
      const stale = vi.fn();
      transport.onStale(stale);
      const merge = transport.invoke('merge_agent', { id: 'a1' });
      const first = socket();
      const callsBefore = first.calls().length;
      first.close(1006, '');
      await expect(merge).rejects.toBe('Connexion au cœur perdue avant la réponse : « merge_agent » n’a peut-être pas été faite.');
      expect(connection.state).toBe('lost');
      // A call meanwhile fails at once, said as not sent.
      await expect(transport.invoke('closed_projects')).rejects.toContain('n’a pas été envoyée');
      // A new try after the first delay.
      expect(sockets).toHaveLength(1);
      await vi.advanceTimersByTimeAsync(5);
      expect(sockets).toHaveLength(2);
      socket().onopen?.();
      expect(socket().texts()[0]).toEqual({ t: 'hello', version: '2.3.0', token: 'secret', session: 's-42' });
      socket().json({ t: 'welcome', session: 's-42', resumed: true, desk: [] });
      // The terminal from the end of what it had; the state read again; the merge never sent again.
      expect(socket().texts()[1]).toEqual({ t: 'resume', channels: [{ channel: output.id, after: 5 }] });
      expect(stale).toHaveBeenCalledTimes(1);
      expect(socket().calls()).toEqual([]);
      expect(first.calls()).toHaveLength(callsBefore);
      // What the terminal printed during the cut, then what it prints now: no hole, no byte twice.
      socket().binary(term(output.id, 11, ' world'));
      socket().json({ t: 'resumed', channel: output.id });
      socket().binary(term(output.id, 11, ' world'));
      socket().binary(term(output.id, 14, '!!!'));
      expect(out.join('')).toBe('hello world!!!');
      expect(connection.state).toBe('open');
    } finally {
      vi.useRealTimers();
    }
  });

  it('tries again after growing delays, up to their bound', async () => {
    vi.useFakeTimers();
    try {
      welcome();
      for (const delay of [5, 10, 10, 10]) {
        const before = sockets.length;
        socket().close(1006, '');
        await vi.advanceTimersByTimeAsync(delay - 1);
        expect(sockets).toHaveLength(before);
        await vi.advanceTimersByTimeAsync(1);
        expect(sockets).toHaveLength(before + 1);
      }
    } finally {
      vi.useRealTimers();
    }
  });

  it('stops trying, and says why, once its token is refused', async () => {
    vi.useFakeTimers();
    try {
      welcome();
      socket().close(1006, '');
      await vi.advanceTimersByTimeAsync(5);
      socket().onopen?.();
      socket().json({ t: 'refused', code: 'token', error: 'Device token missing, unknown or revoked.' });
      socket().close(1008, 'token');
      expect(connection.state).toBe('refused');
      expect(connection.reason).toBe('Le jeton de cet appareil est absent, inconnu ou révoqué.');
      await vi.advanceTimersByTimeAsync(1000);
      expect(sockets).toHaveLength(2);
      await expect(transport.invoke('closed_projects')).rejects.toBe(connection.reason);
    } finally {
      vi.useRealTimers();
    }
  });

  it('sends a raw body in binary before its call, and puts an answer in parts together', async () => {
    welcome();
    const bytes = new Uint8Array([1, 2, 3, 4]);
    const written = transport.invoke('fs_write_bytes', bytes, { headers: { 'x-path': 'a.bin', 'x-expect': 'absent' } });
    const [frame] = socket().sent.filter((m): m is Uint8Array => typeof m !== 'string');
    const [call] = socket().calls();
    expect(frame[0]).toBe(5);
    expect(Number(new DataView(frame.buffer).getBigUint64(1))).toBe(call.id);
    expect([...frame.subarray(9)]).toEqual([1, 2, 3, 4]);
    expect(call).toMatchObject({ cmd: 'fs_write_bytes', args: {}, body: 4, headers: { 'x-path': 'a.bin', 'x-expect': 'absent' } });
    socket().json({ t: 'reply', id: call.id, ok: { mtime: 1, size: 4 } });
    await expect(written).resolves.toEqual({ mtime: 1, size: 4 });
    const read = transport.invoke<ArrayBuffer>('fs_read_bytes', { projectId: 'p', path: 'a.bin' });
    const id = socket().calls()[1].id;
    socket().binary(head(4, id, new Uint8Array([9, 8])));
    socket().binary(head(2, id, new Uint8Array([7])));
    expect([...new Uint8Array(await read)]).toEqual([9, 8, 7]);
  });

  it('reads the state again when the core let events go for a slow reading', () => {
    welcome();
    const stale = vi.fn();
    transport.onStale(stale);
    socket().json({ t: 'resync' });
    expect(stale).toHaveBeenCalledTimes(1);
  });
});
