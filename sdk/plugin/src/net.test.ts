// The streams and the sockets of the SDK (ESC-332), on a fake Escouade: what it writes, and what it does of what comes.
import { describe, expect, it } from 'vitest';
import { createPlugin, openSocket, openStream, type Plugin } from './index';

/** A plugin that speaks to `lines`, and what answers its calls. */
function wired() {
  const lines: Array<Record<string, unknown>> = [];
  const answers: Record<string, unknown> = {};
  let plugin: Plugin;
  const receive = (m: unknown) => plugin.receive(JSON.stringify({ jsonrpc: '2.0', ...(m as object) }));
  plugin = createPlugin().listen({
    write(line) {
      const m = JSON.parse(line);
      lines.push(m);
      if (m.id !== undefined && answers[m.method] !== undefined) void receive({ id: m.id, result: answers[m.method] });
    },
  });
  return { plugin, lines, answers, receive };
}

describe('openStream', () => {
  it('hands the pieces in order, gives their bytes back and ends with net/end', async () => {
    const { plugin, lines, answers, receive } = wired();
    answers['net/stream'] = { stream: 7, status: 200, headers: { 'content-type': 'text/event-stream' } };
    const stream = await openStream(plugin, 'https://api.example.dev/events', { sse: true, headers: { accept: 'text/event-stream' } });
    expect(stream.status).toBe(200);
    expect(lines[0]).toMatchObject({
      method: 'net/stream',
      params: { url: 'https://api.example.dev/events', sse: true, headers: { accept: 'text/event-stream' } },
    });
    await receive({ method: 'net/chunk', params: { stream: 7, data: 'héllo' } });
    await receive({ method: 'net/chunk', params: { stream: 7, dataBase64: 'AAEC/w==' } });
    await receive({ method: 'net/end', params: { stream: 7 } });
    const got = [];
    for await (const piece of stream) got.push(piece);
    expect(got).toEqual([{ data: 'héllo' }, { dataBase64: 'AAEC/w==' }]);
    expect(lines.filter((l) => l.method === 'net/credit').map((l) => l.params)).toEqual([
      { stream: 7, bytes: 6 },
      { stream: 7, bytes: 4 },
    ]);
  });

  it('keeps a piece that came before the answer, says why a stream broke, and cancels', async () => {
    const { plugin, lines, answers, receive } = wired();
    await receive({ method: 'net/end', params: { stream: 0 } }).catch(() => {});
    const early = openStream(plugin, 'https://a.example.dev/');
    await receive({ method: 'net/chunk', params: { stream: 3, data: 'first' } });
    await receive({ id: 'p1', result: { stream: 3, status: 200, headers: {} } });
    const stream = await early;
    await receive({ method: 'net/end', params: { stream: 3, error: 'a.example.dev: no byte for 120 s' } });
    const got: unknown[] = [];
    await expect(
      (async () => {
        for await (const piece of stream) got.push(piece);
      })(),
    ).rejects.toThrow('no byte for 120 s');
    expect(got).toEqual([{ data: 'first' }]);

    answers['net/stream'] = { stream: 4, status: 200, headers: {} };
    const other = await openStream(plugin, 'https://a.example.dev/');
    other.cancel();
    expect(lines.at(-1)).toMatchObject({ method: 'net/cancel', params: { stream: 4 } });
    for await (const piece of other) throw new Error(`nothing more: ${JSON.stringify(piece)}`);
  });
});

describe('openSocket', () => {
  it('sends, hands the messages and says how it ended', async () => {
    const { plugin, lines, answers, receive } = wired();
    answers['ws/open'] = { socket: 2 };
    answers['ws/send'] = null;
    const socket = await openSocket(plugin, 'wss://live.example.dev/', { authorization: 'Token k' });
    expect(lines[0]).toMatchObject({
      method: 'ws/open',
      params: { url: 'wss://live.example.dev/', headers: { authorization: 'Token k' } },
    });
    await receive({ method: 'ws/message', params: { socket: 2, text: 'hello' } });
    await receive({ method: 'ws/closed', params: { socket: 2, code: 1000, reason: 'bye' } });
    await expect(socket.closed).resolves.toEqual({ code: 1000, reason: 'bye' });
    const got = [];
    for await (const m of socket) got.push(m);
    expect(got).toEqual([{ text: 'hello' }]);
  });
});
