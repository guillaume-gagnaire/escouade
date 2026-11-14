// Audit 2026-11, black box: what the app listens on, and what its MCP server answers to whom. The app is the test
// instance of the fixture (its own data folder, the fake harness); « Les agents peuvent piloter Escouade » is on from
// its start, on a port of the test's.
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { addProject, expect, test, type App } from '../../../../../tests/e2e/fixture';
import { filesHolding, instanceProcesses, invoke, keep, listeners, mcpRequest, memoryMb, PORTS, raw, redact, tool } from './helpers';

test.use({ preStart: { run: () => ({ mcpEnabled: true, mcpPort: PORTS.mcp }) } });

/** The server as the fake `claude mcp add` was told it: in the test's own `.claude.json`, never the user's. */
async function declared(app: App): Promise<{ port: number; token: string }> {
  let entry: { url: string; headers?: { Authorization?: string } } | undefined;
  await expect
    .poll(
      () => {
        if (fs.existsSync(app.claudeJson)) entry = JSON.parse(fs.readFileSync(app.claudeJson, 'utf8')).mcpServers?.escouade;
        return !!entry;
      },
      { timeout: 60_000 },
    )
    .toBe(true);
  return { port: Number(new URL(entry!.url).port), token: String(entry!.headers?.Authorization ?? '').replace(/^Bearer /, '') };
}

/** The token the agent `id` was started with: the file of its process, in the test's data folder. */
function agentToken(app: App, id: string): string {
  const file = path.join(app.data, 'mcp', `${id}.json`);
  const text = fs.readFileSync(file, 'utf8');
  const found = /Bearer ([A-Za-z0-9_-]{20,})/.exec(text);
  if (!found) throw new Error(`no token in ${file}`);
  return found[1];
}

const init = {
  jsonrpc: '2.0',
  id: 1,
  method: 'initialize',
  params: { protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 'audit', version: '1' } },
};

test('the app listens on the loopback alone', async ({ app }) => {
  const server = await declared(app);
  const ours = instanceProcesses(app.root);
  const apps = ours.filter((p) => p.name.toLowerCase() === 'escouade.exe');
  expect(apps.length).toBeGreaterThan(0);
  const heard = listeners(ours.map((p) => p.pid));
  const named = heard.map((l) => ({ ...l, process: ours.find((p) => p.pid === l.pid)?.name }));
  keep('ports', { mcpPort: server.port, processes: ours.map((p) => p.name).sort(), listeners: named.map(({ pid: _pid, ...l }) => l) });
  // The app itself: its MCP server, on 127.0.0.1.
  const ofApp = named.filter((l) => l.process?.toLowerCase() === 'escouade.exe' && l.protocol === 'tcp');
  expect(ofApp.map((l) => l.port)).toContain(server.port);
  for (const l of named.filter((l) => l.protocol === 'tcp')) {
    expect(['127.0.0.1', '::1'], `${l.process} listens on ${l.address}:${l.port}`).toContain(l.address);
  }
  // From another address of the machine, nothing answers on that port.
  const other = Object.values((await import('node:os')).networkInterfaces())
    .flat()
    .find((i) => i && i.family === 'IPv4' && !i.internal)?.address;
  if (other) {
    const reached = await new Promise<boolean>((resolve) => {
      const s = net.connect({ host: other, port: server.port });
      s.setTimeout(3000, () => (s.destroy(), resolve(false)));
      s.on('connect', () => (s.destroy(), resolve(true)));
      s.on('error', () => resolve(false));
    });
    expect(reached, 'the MCP server answers on a network address of the machine').toBe(false);
  }
});

test('the MCP server turns away what has no token, a forged Host, a web page, and another path', async ({ app }) => {
  const { port, token } = await declared(app);
  const host = `127.0.0.1:${port}`;
  const good = { host, authorization: `Bearer ${token}` };
  const seen: Record<string, { status: number; cors: string | null; body: string }> = {};
  const note = (name: string, r: { status: number; headers: Record<string, string>; body: string }) => {
    seen[name] = {
      status: r.status,
      cors:
        Object.entries(r.headers)
          .filter(([k]) => k.startsWith('access-control-'))
          .map(([k, v]) => `${k}: ${v}`)
          .join('; ') || null,
      body: redact(r.body.slice(0, 160), token),
    };
    return r.status;
  };

  expect(note('valid token (reference)', await mcpRequest(port, good, init))).toBe(200);
  // No token, a token that is none, a token elsewhere than in its header.
  expect(note('no token', await mcpRequest(port, { host }, init))).toBe(401);
  expect(note('wrong token', await mcpRequest(port, { host, authorization: `Bearer ${'A'.repeat(43)}` }, init))).toBe(401);
  expect(note('token of another length', await mcpRequest(port, { host, authorization: `Bearer ${token.slice(0, -1)}` }, init))).toBe(401);
  expect(note('token with a suffix', await mcpRequest(port, { host, authorization: `Bearer ${token}x` }, init))).toBe(401);
  expect(note('basic scheme', await mcpRequest(port, { host, authorization: `Basic ${token}` }, init))).toBe(401);
  expect(note('token in the query', await mcpRequest(port, { host }, init, `/mcp?token=${token}&access_token=${token}`))).toBe(401);
  expect(note('token in a cookie', await mcpRequest(port, { host, cookie: `token=${token}` }, init))).toBe(401);
  // A page of the web reaching the loopback (DNS rebinding, a form): its Host or its Origin gives it away, token or not.
  for (const forged of [
    'evil.example',
    `evil.example:${port}`,
    `127.0.0.1:${port}.evil.example`,
    `127.0.0.1:${port + 1}`,
    '127.0.0.1',
    `[::1]:${port}`,
    `0.0.0.0:${port}`,
  ]) {
    expect(
      note(
        `Host: ${forged}`.replace(String(port), '<port>').replace(String(port + 1), '<port+1>'),
        await mcpRequest(port, { ...good, host: forged }, init),
      ),
    ).toBe(403);
  }
  for (const origin of ['https://evil.example', 'null', `http://127.0.0.1:${port}`, 'http://localhost', 'tauri://localhost']) {
    expect(note(`Origin: ${origin}`.replace(String(port), '<port>'), await mcpRequest(port, { ...good, origin }, init))).toBe(403);
  }
  expect(
    note(
      'no Host (HTTP/1.0)',
      await raw(
        port,
        `POST /mcp HTTP/1.0\r\nauthorization: Bearer ${token}\r\ncontent-type: application/json\r\ncontent-length: 2\r\n\r\n{}`,
      ),
    ),
  ).not.toBe(200);
  expect(note('absolute URI of another host', await mcpRequest(port, good, init, 'http://evil.example/mcp'))).toBe(403);
  // A browser's preflight: refused, and no header that would let a page read an answer.
  const preflight = await mcpRequest(
    port,
    {
      host,
      origin: 'https://evil.example',
      'access-control-request-method': 'POST',
      'access-control-request-headers': 'authorization, content-type',
    },
    '',
    '/mcp',
    'OPTIONS',
  );
  expect(note('CORS preflight', preflight)).toBe(403);
  for (const r of Object.values(seen)) expect(r.cors, 'an Access-Control header came back').toBeNull();
  // Other paths: nothing is served but /mcp, and without a token nothing tells a path from another.
  for (const target of ['/', '/mcp/', '/mcp/../mcp', '/MCP', '/mcp%2f', '//mcp', '/.env', '/hook']) {
    expect(note(`path ${target} with the token`, await mcpRequest(port, good, init, target))).toBe(404);
    expect(note(`path ${target} without a token`, await mcpRequest(port, { host }, init, target))).toBe(401);
  }
  expect(note('GET /mcp with the token', await mcpRequest(port, good, '', '/mcp', 'GET'))).not.toBe(200);
  // A body that is no JSON-RPC, and one past the limit announced.
  expect(note('a body that is not JSON', await mcpRequest(port, good, '{"jsonrpc":'))).not.toBe(200);
  expect(
    note(
      'a body announced at 64 MiB',
      await raw(
        port,
        `POST /mcp HTTP/1.1\r\nhost: ${host}\r\nauthorization: Bearer ${token}\r\ncontent-type: application/json\r\ncontent-length: ${64 << 20}\r\nconnection: close\r\n\r\n{}`,
        8000,
      ),
    ),
  ).not.toBe(200);
  keep('mcp-guards', seen);
  // The refusals are in the activity log of the server, and none of them holds the token.
  expect(filesHolding(app.data, token).filter((f) => !f.startsWith('mcp'))).toEqual([]);
});

test('connections that say nothing do not hold the server for long', async ({ app }) => {
  const { port, token } = await declared(app);
  // 64 connections at once is what it takes; 80 are opened and left silent.
  const idle = Array.from({ length: 80 }, () => net.connect({ host: '127.0.0.1', port }));
  await new Promise((r) => setTimeout(r, 1500));
  const started = Date.now();
  const r = await mcpRequest(port, { host: `127.0.0.1:${port}`, authorization: `Bearer ${token}` }, init).catch(() => ({ status: 0 }));
  const waited = Date.now() - started;
  for (const s of idle) s.destroy();
  keep('mcp-idle-connections', { silentConnections: idle.length, statusOfTheRequestBehindThem: r.status, waitedMs: waited });
  // Served once the silent ones are closed for saying nothing (10 s), not held for ever.
  expect(r.status).toBe(200);
  expect(waited).toBeLessThan(14_000);
});

test('an agent answers another agent’s permission, never its own, and stays within its folder', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  const { port, token } = await declared(app);
  const external = (name: string, args: object) => tool(port, token, name, args);
  // Two agents of the project, started as Claude outside Escouade starts them: B asks for a permission and waits.
  const a = (await external('create_agent', { project: 'demo-api', message: 'bonjour' })).value as { id: string; name: string };
  const b = (await external('create_agent', { project: 'demo-api', message: 'demande une permission' })).value as {
    id: string;
    name: string;
  };
  expect(a.id && b.id).toBeTruthy();
  type Waiting = { agent: { id: string }; request: string; type: string; tool?: string; arg?: string };
  let waiting: Waiting | undefined;
  await expect
    .poll(
      async () => {
        const all = (await external('list_requests', {})).value as { requests?: Waiting[] } | Waiting[];
        const list = Array.isArray(all) ? all : (all.requests ?? []);
        waiting = list.find((r) => r.agent.id === b.id && r.type === 'permission');
        return !!waiting;
      },
      { timeout: 90_000 },
    )
    .toBe(true);
  const [tokenA, tokenB] = [agentToken(app, a.id), agentToken(app, b.id)];
  const asA = (name: string, args: object) => tool(port, tokenA, name, args);
  const asB = (name: string, args: object) => tool(port, tokenB, name, args);
  const seen: Record<string, { refused: boolean; said: string }> = {};
  const note = (name: string, r: { isError: boolean; text: string }) => {
    seen[name] = { refused: r.isError, said: redact(r.text.slice(0, 260), token, tokenA, tokenB) };
    return r.isError;
  };

  expect(note('A lists the projects (reference)', await asA('list_projects', {}))).toBe(false);
  // B may not answer its own request; nor may anyone make the permission a lasting rule.
  expect(
    note('B answers its own permission', await asB('answer_permission', { agent: b.id, request: waiting!.request, decision: 'allow' })),
  ).toBe(true);
  expect(note('A answers "always"', await asA('answer_permission', { agent: b.id, request: waiting!.request, decision: 'always' }))).toBe(
    true,
  );
  // What an agent may not do through the server.
  const outside = path.join(app.root, 'outside-the-project.txt');
  fs.writeFileSync(outside, 'AUDIT-POC: a file outside the agent’s folder');
  expect(
    note(
      'A attaches a file outside its folder to a ticket',
      await asA('create_ticket', { project: 'demo-api', title: 'Audit', files: [{ path: outside }] }),
    ),
  ).toBe(true);
  expect(note('A sets the Kanban to bypassPermissions', await asA('set_board', { project: 'demo-api', mode: 'bypassPermissions' }))).toBe(
    true,
  );
  expect(note('A stops itself', await asA('stop_agent', { agent: a.id }))).toBe(true);
  expect(note('A writes to itself', await asA('send_message', { agent: a.id, text: 'x' }))).toBe(true);
  // A allows what B asked. Nobody was at the window: it is refused, whatever the way.
  const allowed = await asA('answer_permission', { agent: b.id, request: waiting!.request, decision: 'allow' });
  expect(note('A allows the permission of B', allowed)).toBe(true);
  {
    const answer = { agent: b.id, request: waiting!.request };
    expect(note('A refuses the permission of B in the user’s place', await asA('answer_permission', { ...answer, decision: 'deny' }))).toBe(
      true,
    );
    expect(note('A answers it as a question', await asA('answer_question', { ...answer, answers: [{ text: 'oui' }] }))).toBe(true);
    // « Seulement le chef d’escouade », and no chef chosen for the project: still nobody. The window's setting, as the user sets it.
    const settings = JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8')) as Record<string, unknown>;
    expect(settings.agentsAnswer ?? 'nobody', 'the default of « Les agents peuvent répondre… »').toBe('nobody');
    await invoke(page, 'save_settings', { settings: { ...settings, agentsAnswer: 'chef' } });
    expect(note('setting on "chef", no chef chosen: A allows', await asA('answer_permission', { ...answer, decision: 'allow' }))).toBe(
      true,
    );
    // No tool of the server lets an agent name the chef or change the setting.
    const tools = await mcpRequest(
      port,
      { host: `127.0.0.1:${port}`, authorization: `Bearer ${tokenA}` },
      { jsonrpc: '2.0', id: 9, method: 'tools/list' },
    );
    const names = ((JSON.parse(tools.body).result?.tools ?? []) as { name: string }[]).map((t) => t.name);
    keep('mcp-tools-of-an-agent', { count: names.length, names: names.sort() });
    expect(names.filter((n) => /setting|chef|lead|update_project|set_project/i.test(n))).toEqual([]);
    await invoke(page, 'save_settings', { settings: { ...settings, agentsAnswer: 'nobody' } });
    // The request still waits for the user, whose own Claude (outside Escouade) may answer it.
    const still = (await external('list_requests', {})).value as { requests?: Waiting[] } | Waiting[];
    expect((Array.isArray(still) ? still : (still.requests ?? [])).some((r) => r.request === waiting!.request)).toBe(true);
    expect(
      note('Claude outside Escouade allows it (the user’s own)', await external('answer_permission', { ...answer, decision: 'allow' })),
    ).toBe(false);
  }
  await expect
    .poll(async () => JSON.stringify((await external('get_agent_summary', { agent: b.id })).value), { timeout: 60_000 })
    .toContain('Commande exécutée');
  // And A starts a third agent with words of its own (by design: said in the report).
  expect(note('A creates an agent', await asA('create_agent', { project: 'demo-api', message: 'bonjour' }))).toBe(false);
  keep('mcp-agents', { request: { type: waiting!.type, tool: waiting!.tool, arg: waiting!.arg }, tries: seen });
  // A token of an agent that is no more opens nothing: another instance's, a made-up one.
  expect((await tool(port, `${tokenA.slice(0, -4)}AAAA`, 'list_projects', {})).status).toBe(401);
});

test('names of tools never seen are turned away before the server keeps any (with a token)', async ({ app }) => {
  const { port, token } = await declared(app);
  const ours = instanceProcesses(app.root);
  const pid = ours.find((p) => p.name.toLowerCase() === 'escouade.exe')!.pid;
  const good = { host: `127.0.0.1:${port}`, authorization: `Bearer ${token}` };
  // A transport that kept each name of a `tools/call` it never saw would grow without end (RUSTSEC-2026-0461 tells
  // such a cache, at the protocol version 2026-07-28). Each name here weighs 256 KiB; a round is 200 of them (50 MiB).
  const NAME = 256 * 1024;
  const round = async (label: string, version: string, from: number) => {
    const statuses: Record<string, number> = {};
    for (let n = from; n < from + 200; n++) {
      const r = await mcpRequest(
        port,
        { ...good, 'mcp-protocol-version': version },
        { jsonrpc: '2.0', id: n, method: 'tools/call', params: { name: `audit-${n}-${'x'.repeat(NAME)}`, arguments: {} } },
      );
      statuses[String(r.status)] = (statuses[String(r.status)] ?? 0) + 1;
    }
    await new Promise((r) => setTimeout(r, 2000));
    return {
      round: label,
      protocolVersion: version,
      mbOfNamesSent: Math.round((200 * NAME) / (1 << 20)),
      statuses,
      privateMemoryMb: memoryMb(pid),
    };
  };
  const start = memoryMb(pid);
  const rounds = [
    await round('1', '2025-06-18', 0),
    await round('2', '2025-06-18', 200),
    await round('3', '2026-07-28', 400),
    await round('4', '2026-07-28', 600),
    await round('5', '2026-07-28', 800),
  ];
  // Without a token, nothing of it: refused before the transport sees the request.
  const without = await mcpRequest(
    port,
    { host: good.host, 'mcp-protocol-version': '2026-07-28' },
    { jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name: 'x'.repeat(NAME), arguments: {} } },
  );
  // Such a name is turned away before the transport: the same names by the ways a gate that reads the body could
  // miss (a batch, a key written twice, bytes after the JSON, an escape), 200 requests of 256 KiB each.
  const around: { shape: string; statuses: Record<string, number>; privateMemoryMb: number }[] = [];
  {
    const big = (n: number) => `audit-${n}-${'x'.repeat(NAME)}`;
    const call = (n: number, name = big(n)) => ({ jsonrpc: '2.0', id: n, method: 'tools/call', params: { name, arguments: {} } });
    const shapes: Record<string, (n: number) => string> = {
      'in a batch, behind a call of a real tool': (n) => JSON.stringify([call(n, 'list_projects'), call(n + 1)]),
      'the method twice, the harmless one last': (n) =>
        `{"jsonrpc":"2.0","id":${n},"method":"tools/call","method":"ping","params":{"name":"${big(n)}","arguments":{}}}`,
      'the method twice, the harmless one first': (n) =>
        `{"jsonrpc":"2.0","id":${n},"method":"ping","method":"tools/call","params":{"name":"${big(n)}","arguments":{}}}`,
      'the name twice, a real tool last': (n) =>
        `{"jsonrpc":"2.0","id":${n},"method":"tools/call","params":{"name":"${big(n)}","name":"list_projects","arguments":{}}}`,
      'the name twice, a real tool first': (n) =>
        `{"jsonrpc":"2.0","id":${n},"method":"tools/call","params":{"name":"list_projects","name":"${big(n)}","arguments":{}}}`,
      'bytes after the JSON': (n) => `${JSON.stringify(call(n))} x`,
      'a byte order mark before the JSON': (n) => `﻿${JSON.stringify(call(n))}`,
      'the method with an escape': (n) => JSON.stringify(call(n)).replace('"tools/call"', '"tools\\u002fcall"'),
      'a real tool’s name in another case': (n) => JSON.stringify(call(n, `LIST_PROJECTS${'X'.repeat(NAME)}${n}`)),
    };
    let n = 10_000;
    for (const [shape, body] of Object.entries(shapes)) {
      const statuses: Record<string, number> = {};
      for (let i = 0; i < 200; i++, n += 2) {
        const r = await mcpRequest(port, { ...good, 'mcp-protocol-version': '2026-07-28' }, body(n));
        statuses[String(r.status)] = (statuses[String(r.status)] ?? 0) + 1;
      }
      await new Promise((r) => setTimeout(r, 1500));
      around.push({ shape, statuses, privateMemoryMb: memoryMb(pid) });
    }
    // A request without image or file past 2 MiB, and one that names a tool that takes files to weigh more.
    const heavy = (name: string, mb: number) =>
      mcpRequest(port, good, { jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name, arguments: { pad: 'x'.repeat(mb << 20) } } });
    around.push({
      shape: 'list_projects with 3 MiB of arguments',
      statuses: { [(await heavy('list_projects', 3)).status]: 1 },
      privateMemoryMb: memoryMb(pid),
    });
    around.push({
      shape: 'create_ticket with 3 MiB of arguments',
      statuses: { [(await heavy('create_ticket', 3)).status]: 1 },
      privateMemoryMb: memoryMb(pid),
    });
  }
  keep('mcp-tool-name-cache', { privateMemoryMbAtStart: start, rounds, statusWithoutToken: without.status, waysAround: around });
  expect(without.status).toBe(401);
  // 250 MiB of names in the rounds, 450 MiB by the ways around: none of it is kept.
  expect(rounds.at(-1)!.privateMemoryMb).toBeGreaterThan(0);
  for (const r of rounds) expect(r.statuses, `round ${r.round}`).toEqual({ '400': 200 });
  expect(around.at(-1)!.privateMemoryMb - start, 'the memory the names left').toBeLessThan(60);
});
