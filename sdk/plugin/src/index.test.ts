// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { createPlugin, ERRORS, FACT_NAMES, PROTOCOL_VERSION, RpcError, type Fact } from './index';
import { connect, decisionRequest, fact, refusedPath } from './testing';

// The SDK's client (ESC-181, PLG-9), driven line by line as Escouade drives it: the handshake, batches, facts by name
// and by pattern, decisions, secrets, logs, errors, shutdown.

/** A plugin listening on lines kept in `out`. */
function wired() {
  const out: unknown[] = [];
  const traces: string[] = [];
  let ended = false;
  const plugin = createPlugin().listen({
    write: (l) => out.push(JSON.parse(l)),
    trace: (t) => traces.push(t),
    end: () => (ended = true),
  });
  return { plugin, out, traces, ended: () => ended };
}

describe('its folders and its programs', () => {
  it('fs and spawn go by their methods, and a program says what it wrote and how it ended', async () => {
    const plugin = createPlugin();
    const asked: unknown[] = [];
    const fake = connect(plugin, {
      host: {
        'fs/read': (params) => (asked.push(params), { text: 'dans son dossier' }),
        'proc/spawn': () => ({ proc: 7 }),
        'proc/write': (params) => (asked.push(params), null),
      },
    });
    await fake.initialize();
    expect(await plugin.fs.text({ agent: 'a', root: 'agent', path: 'n.txt' })).toBe('dans son dossier');
    const said: string[] = [];
    // What comes before the answer of `proc/spawn` is not lost.
    const started = plugin.runProgram({ command: 'git', args: ['--version'] }, (data, stream) => said.push(`${stream}:${data}`));
    await fake.notify('proc/out', { proc: 7, stream: 'stdout', data: 'git version' });
    const program = await started;
    await fake.notify('proc/out', { proc: 7, stream: 'stderr', data: 'warn' });
    await program.write('x', true);
    await fake.notify('proc/exit', { proc: 7, code: 0 });
    expect(await program.exited).toBe(0);
    expect(said).toEqual(['stdout:git version', 'stderr:warn']);
    expect(asked).toEqual([
      { agent: 'a', root: 'agent', path: 'n.txt' },
      { proc: 7, data: 'x', close: true },
    ]);
  });

  it('a file that is no text and a refusal reject', async () => {
    const plugin = createPlugin();
    const fake = connect(plugin, {
      host: {
        'fs/read': (p: { path: string }) => {
          if (p.path === 'out') throw new RpcError(ERRORS.refused, 'leads out');
          return { dataBase64: '//4=' };
        },
      },
    });
    await fake.initialize();
    await expect(plugin.fs.text({ root: 'temp', path: 'bin' })).rejects.toMatchObject({ code: ERRORS.failed });
    await expect(plugin.fs.read({ root: 'temp', path: 'out' })).rejects.toMatchObject({ code: ERRORS.refused });
  });
});

describe('@escouade-dev/plugin', () => {
  it('answers initialize with its version whatever is offered, and keeps what Escouade said', async () => {
    const { plugin, out } = wired();
    await plugin.receive(
      JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'initialize',
        params: { protocolVersion: 999, escouade: { version: '2.1.1' }, plugin: { id: 'a.b', version: '1.0.0' }, capabilities: {} },
      }),
    );
    expect(out).toEqual([{ jsonrpc: '2.0', id: 1, result: { protocolVersion: PROTOCOL_VERSION, capabilities: { batch: true } } }]);
    expect(plugin.escouade?.escouade.version).toBe('2.1.1');
  });

  it('answers a batch with a batch, without the notifications', async () => {
    const { plugin, out } = wired();
    await plugin.receive(
      JSON.stringify([
        { jsonrpc: '2.0', id: 'a', method: 'nope' },
        { jsonrpc: '2.0', method: 'events/fact', params: fact('app.started') },
        { jsonrpc: '2.0', id: 'b', method: 'permissions/decide', params: decisionRequest() },
      ]),
    );
    expect(out).toEqual([
      [
        { jsonrpc: '2.0', id: 'a', error: { code: ERRORS.methodNotFound, message: 'unknown method nope' } },
        { jsonrpc: '2.0', id: 'b', result: { decision: 'ask' } },
      ],
    ]);
  });

  it('says a line that is not JSON, or not JSON-RPC', async () => {
    const { plugin, out } = wired();
    await plugin.receive('{nope');
    await plugin.receive('{"id":1,"method":"x"}');
    expect(out).toEqual([
      { jsonrpc: '2.0', id: null, error: { code: ERRORS.parse, message: 'not JSON' } },
      { jsonrpc: '2.0', id: null, error: { code: ERRORS.invalidRequest, message: 'not JSON-RPC 2.0' } },
    ]);
  });

  it('routes facts by name and by pattern, and a failing handler only to its traces', async () => {
    const plugin = createPlugin();
    const seen: string[] = [];
    plugin.onEvent('ticket.column', (f) => void seen.push(`column ${f.data.to}`));
    plugin.onEvent(['ticket.*', 'agent.status'], (f) => void seen.push(`any ${f.name}`));
    plugin.onEvent('ticket.merged', () => {
      throw new Error('boom');
    });
    const escouade = connect(plugin);
    await escouade.initialize();
    await escouade.fact('ticket.column');
    await escouade.fact('ticket.merged');
    await escouade.fact('agent.status', { data: { status: 'waiting', was: 'running' } });
    await escouade.fact('app.started');
    expect(seen).toEqual(['column review', 'any ticket.column', 'any ticket.merged', 'any agent.status']);
    expect(escouade.traces.join('\n')).toContain('events/fact ticket.merged: Error: boom');
    // A notification is never answered: everything it wrote answers the one call, initialize.
    expect(escouade.received).toHaveLength(1);
  });

  it('decides permissions, a thrown RpcError answered with its code', async () => {
    const plugin = createPlugin().decidePermission(async ({ request }) => {
      if (request.input?.command === 'boom') throw new RpcError(-32010, 'no rule');
      return request.input?.command === 'rm -rf /' ? { decision: 'deny', message: 'never' } : { decision: 'allow' };
    });
    const escouade = connect(plugin);
    await escouade.initialize();
    expect(await escouade.decide()).toEqual({ decision: 'allow' });
    expect(await escouade.decide({ request: { input: { command: 'rm -rf /' } } })).toEqual({ decision: 'deny', message: 'never' });
    await expect(escouade.decide({ request: { input: { command: 'boom' } } })).rejects.toMatchObject({ code: -32010 });
  });

  it('asks for its secrets, and a refused one rejects with -32001', async () => {
    const plugin = createPlugin();
    const got: Array<string | null> = [];
    plugin.onEvent('ticket.column', async (_f, p) => {
      got.push(await p.secret('webhook'));
      got.push(await p.secret('empty'));
      await p.secret('other').catch((e: RpcError) => got.push(`refused ${e.code}`));
    });
    const escouade = connect(plugin, { secrets: { webhook: 'https://hooks.example', empty: null } });
    await escouade.initialize();
    await escouade.fact('ticket.column');
    expect(got).toEqual(['https://hooks.example', null, `refused ${ERRORS.notYours}`]);
    expect(escouade.secretsAsked).toEqual(['webhook', 'empty', 'other']);
  });

  it('asks for its settings, those of the fact’s project over the user’s', async () => {
    const plugin = createPlugin();
    const got: unknown[] = [];
    plugin.onEvent('ticket.column', async (f, p) => {
      got.push(await p.settings());
      got.push(await p.settings(f.project?.id));
    });
    const escouade = connect(plugin, {
      settings: { channel: '#escouade', mentions: false },
      projectSettings: { 'bench-project': { channel: '#equipe' } },
    });
    await escouade.initialize();
    await escouade.fact('ticket.column');
    expect(got).toEqual([
      { channel: '#escouade', mentions: false },
      { channel: '#equipe', mentions: false },
    ]);
  });

  it('logs to the journal, serves its own methods, and ends after shutdown once its handlers ran', async () => {
    const { plugin, out, ended } = wired();
    let closed = false;
    plugin.handle('tickets/list', (params) => ({ echo: params }));
    plugin.onShutdown(() => void (closed = true));
    plugin.log.warn('careful');
    await plugin.receive('{"jsonrpc":"2.0","id":2,"method":"tickets/list","params":{"q":1}}');
    expect(ended()).toBe(false);
    await plugin.receive('{"jsonrpc":"2.0","id":3,"method":"shutdown"}');
    expect(out).toEqual([
      { jsonrpc: '2.0', method: 'log', params: { level: 'warn', message: 'careful' } },
      { jsonrpc: '2.0', id: 2, result: { echo: { q: 1 } } },
      { jsonrpc: '2.0', id: 3, result: null },
    ]);
    expect(closed).toBe(true);
    expect(ended()).toBe(true);
  });

  it('builds the facts and decisions of the bench, extra fields merged', () => {
    const f: Fact = fact('ticket.column', { ticket: { key: 'APP-1' }, project: { private: true } });
    expect(f).toMatchObject({
      name: 'ticket.column',
      version: 1,
      project: { id: 'bench-project', name: 'Banc', private: true },
      seq: 1,
      ticket: { id: 'bench-ticket', key: 'APP-1', title: 'Un ticket du banc' },
      data: { from: 'doing', to: 'review' },
    });
    expect(decisionRequest().request).toEqual({ id: 'bench-request', kind: 'command', tool: 'Bash', input: { command: 'npm test' } });
    expect(FACT_NAMES).toContain('turn.ended');
  });

  it('reads a file of its folder by files/read, and is refused any path out of it (ESC-321)', async () => {
    const plugin = createPlugin();
    const escouade = connect(plugin, { files: { 'rules.json': '{"a":1}', 'data/words.txt': 'un' } });
    await escouade.initialize();
    await expect(plugin.file('rules.json')).resolves.toBe('{"a":1}');
    await expect(plugin.file('data/words.txt')).resolves.toBe('un');
    for (const path of ['../x.json', '/etc/passwd', 'C:/x', 'data/../../x', '.env', '.git/config', 'escouade-plugin.json', '']) {
      await expect(plugin.file(path), path).rejects.toMatchObject({ code: ERRORS.refused });
    }
    await expect(plugin.file('nope.json')).rejects.toMatchObject({ code: ERRORS.notFound });
    expect(refusedPath('data/words.txt')).toBeNull();
    expect(escouade.received.filter((m) => (m as { method?: string }).method === 'files/read')).toHaveLength(11);
  });

  it("speaks through Escouade's WASM host when it is there, without a timer for shutdown (ESC-321)", async () => {
    const g = globalThis as Record<string, unknown>;
    const sent: string[] = [];
    const traces: string[] = [];
    let listener: ((line: string) => void) | undefined;
    let ended = false;
    const timer = g.setTimeout;
    g.Javy = { IO: {} };
    g.__esc = {
      listen: (cb: (line: string) => void) => (listener = cb),
      write: (line: string) => sent.push(line),
      trace: (line: string) => traces.push(line),
      end: () => (ended = true),
    };
    try {
      const plugin = createPlugin();
      let closed = false;
      plugin.onShutdown(() => void (closed = true));
      plugin.onEvent('app.started', () => {
        throw new Error('boom');
      });
      let resolved = false;
      void plugin.start().then(() => (resolved = true));
      expect(listener).toBeDefined();
      listener!(
        '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"escouade":{"version":"2.2.0"},"plugin":{"id":"a.b","version":"1.0.0"},"capabilities":{}}}',
      );
      listener!(JSON.stringify({ jsonrpc: '2.0', method: 'events/fact', params: fact('app.started') }));
      const asked = plugin.file('rules.json');
      await plugin.idle();
      listener!('{"jsonrpc":"2.0","id":"p1","result":"{}"}');
      await expect(asked).resolves.toBe('{}');
      // No setTimeout in WASM: shutdown is answered once its handlers are done.
      g.setTimeout = undefined;
      listener!('{"jsonrpc":"2.0","id":2,"method":"shutdown"}');
      await plugin.idle();
      expect(sent.map((l) => JSON.parse(l))).toEqual(
        expect.arrayContaining([
          { jsonrpc: '2.0', id: 1, result: { protocolVersion: PROTOCOL_VERSION, capabilities: { batch: true } } },
          { jsonrpc: '2.0', id: 'p1', method: 'files/read', params: { path: 'rules.json' } },
          { jsonrpc: '2.0', id: 2, result: null },
        ]),
      );
      expect(sent).toHaveLength(3);
      expect(traces.join('\n')).toContain('events/fact app.started: Error: boom');
      expect(closed).toBe(true);
      expect(ended).toBe(true);
      expect(resolved).toBe(false);
    } finally {
      g.setTimeout = timer;
      delete g.Javy;
      delete g.__esc;
    }
  });
});
