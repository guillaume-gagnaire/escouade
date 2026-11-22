// A local space for the phone's page (MO-1), shared by its e2e (e2e/harness.ts) and `npm run dev:server`: the data
// folder of an `escouade-server` with the fake `claude`, the forward that stands for the phone's tunnel (MO-2, MO-3:
// a browser page's Origin taken off), a client of the API, and an agent stopped on a permission.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = fileURLToPath(new URL('../../..', import.meta.url));
/** The version the phone says in its handshake: the app's. */
export const VERSION = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8')).version;
const FAKE = path.join(ROOT, 'tests', 'fixtures', process.platform === 'win32' ? 'fake-claude.cmd' : 'fake-claude');
const NO_STORE = 'http://127.0.0.1:9';

/** The data folder of a server under `root`, written (its settings: the fake `claude`), and the server's environment. */
export function prepareServer(root) {
  const data = path.join(root, 'data');
  fs.mkdirSync(data, { recursive: true });
  fs.writeFileSync(
    path.join(data, 'settings.json'),
    JSON.stringify({
      claudePath: FAKE,
      sound: false,
      osNotifications: false,
      idleStopMinutes: 0,
      language: 'fr',
      telemetry: false,
      nameAsked: true,
      accountsVersion: 1,
      autoUpdate: false,
      midTurnDelaySecs: 0,
      accounts: [{ id: 'principal', name: 'Principal', engine: 'claude', configDir: '', claudePath: '', active: true }],
    }),
  );
  // Never the machine's own Claude, store, updates or harnesses.
  const env = {
    ...process.env,
    ESCOUADE_DATA_DIR: data,
    CLAUDE_CONFIG_DIR: path.join(root, 'claude'),
    FAKE_CLAUDE_HOME: path.join(root, 'home'),
    FAKE_CLAUDE_LOG: path.join(root, 'fake-claude.jsonl'),
    ESCOUADE_STORE_URL: NO_STORE,
    ESCOUADE_STORE_MIRROR_URL: NO_STORE,
    ESCOUADE_TEST_OPEN_VSX: NO_STORE,
    ESCOUADE_CODEX: 'none',
    ESCOUADE_OPENCODE: 'none',
    ESCOUADE_COPILOT: 'none',
    ESCOUADE_CURSOR: 'none',
    ESCOUADE_VIBE: 'none',
    ESCOUADE_AGY: 'none',
    ESCOUADE_GEMINI: 'none',
    CODEX_HOME: path.join(root, 'codex'),
    COPILOT_HOME: path.join(root, 'copilot'),
    XDG_CONFIG_HOME: path.join(root, 'xdg'),
  };
  for (const k of ['CLAUDE_SECURESTORAGE_CONFIG_DIR', 'CLAUDE_CODE_ENABLE_TODO_TOOLS', 'DISABLE_AUTOUPDATER', 'ESCOUADE_UPDATE_ENDPOINT'])
    delete env[k];
  return { data, env };
}

/**
 * A forward of `listen` (0: a free port) to 127.0.0.1:`to`, standing for the phone's tunnel: the bytes as they come,
 * but the `Origin` of the request that opens the WebSocket taken off. Through the real tunnel, the core takes the
 * WebView's Origin (`proto::PHONE_ORIGINS`, MO-2); on the loopback, where the test reaches it, it refuses every page's
 * Origin but its own window's, and takes a client without one as a program of its own. The token is asked all the same.
 * @returns {Promise<{ port: number, close: () => void }>}
 */
export function phoneForward(listen, to) {
  const sockets = new Set();
  const server = net.createServer((client) => {
    const upstream = net.connect({ host: '127.0.0.1', port: to });
    for (const s of [client, upstream]) {
      sockets.add(s);
      s.on('close', () => sockets.delete(s));
      s.on('error', () => {
        client.destroy();
        upstream.destroy();
      });
    }
    // The request's head, until its blank line, then the rest as it comes.
    let head = Buffer.alloc(0);
    const onHead = (chunk) => {
      head = Buffer.concat([head, chunk]);
      const end = head.indexOf('\r\n\r\n');
      if (end < 0) return;
      client.off('data', onHead);
      const text = head
        .subarray(0, end)
        .toString('latin1')
        .replace(/\r\nOrigin: [^\r\n]*/i, '');
      upstream.write(Buffer.concat([Buffer.from(text, 'latin1'), head.subarray(end)]));
      client.pipe(upstream);
    };
    client.on('data', onHead);
    upstream.pipe(client);
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(listen, '127.0.0.1', () => {
      let open = true;
      resolve({
        port: server.address().port,
        close: () => {
          for (const s of sockets) s.destroy();
          if (open) server.close();
          open = false;
        },
      });
    });
  });
}

/** The first message of the server to `message`, sent first on a new WebSocket to `url`. */
function firstAnswer(url, message) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    const timer = setTimeout(() => {
      ws.close();
      reject(new Error(`no answer from ${url} in 20 s`));
    }, 20_000);
    ws.onopen = () => ws.send(JSON.stringify(message));
    ws.onmessage = (e) => {
      clearTimeout(timer);
      ws.close();
      resolve(JSON.parse(String(e.data)));
    };
    ws.onerror = () => {
      clearTimeout(timer);
      reject(new Error(`the WebSocket to ${url} failed`));
    };
  });
}

/** A phone paired with the server of `data` on `port`, as the user does it (`escouade-server pair`, then its code): its token. */
export async function pairPhone(program, data, env, port) {
  const printed = execFileSync(program, ['pair', '--data-dir', data], { env, encoding: 'utf8', timeout: 60_000 });
  const code = /([A-Z2-9]{4}-[A-Z2-9]{4})/.exec(printed)?.[1];
  if (!code) throw new Error(`escouade-server pair printed no code:\n${printed}`);
  const paired = await firstAnswer(`ws://127.0.0.1:${port}/remote`, { t: 'pair', code, name: 'Téléphone de test' });
  if (paired.t !== 'paired' || typeof paired.token !== 'string') throw new Error(`the pairing was refused: ${JSON.stringify(paired)}`);
  return paired.token;
}

/**
 * A client of the API of its own (no Origin, as a program): the handshake, then `call(cmd, args)`.
 * @returns {Promise<{ call: (cmd: string, args?: Record<string, unknown>) => Promise<any>, close: () => void }>}
 */
export async function apiClient(url, token) {
  const ws = new WebSocket(url);
  let next = 1;
  const waiting = new Map();
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`no welcome from ${url} in 20 s`)), 20_000);
    ws.onopen = () => ws.send(JSON.stringify({ t: 'hello', version: VERSION, token }));
    ws.onerror = () => reject(new Error(`the WebSocket to ${url} failed`));
    ws.onmessage = (e) => {
      if (typeof e.data !== 'string') return;
      const m = JSON.parse(e.data);
      if (m.t === 'welcome') {
        clearTimeout(timer);
        resolve(undefined);
      } else if (m.t === 'refused') reject(new Error(`refused: ${e.data}`));
      else if (m.t === 'reply') {
        const p = waiting.get(m.id);
        waiting.delete(m.id);
        if (m.error !== undefined) p?.reject(new Error(String(m.error)));
        else p?.resolve(m.ok ?? null);
      }
    };
  });
  return {
    call: (cmd, args = {}) =>
      new Promise((resolve, reject) => {
        const id = next++;
        waiting.set(id, { resolve, reject });
        ws.send(JSON.stringify({ t: 'call', id, cmd, args }));
      }),
    close: () => ws.close(),
  };
}

const git = (cwd, ...args) => execFileSync('git', args, { cwd, stdio: 'ignore' });

/** A repository under `root`, with one commit. */
export function makeRepo(root, name = 'facturation') {
  const repo = path.join(root, name);
  fs.mkdirSync(path.join(repo, 'src'), { recursive: true });
  git(repo, 'init', '-q', '-b', 'main');
  git(repo, 'config', 'user.email', 'e2e@test');
  git(repo, 'config', 'user.name', 'e2e');
  git(repo, 'config', 'core.autocrlf', 'false');
  fs.writeFileSync(path.join(repo, 'src', 'app.ts'), 'const a = 1;\n');
  git(repo, 'add', '-A');
  git(repo, 'commit', '-qm', 'init');
  return repo;
}

/** A project of the server and its agent, stopped on a permission (the fake `claude` asks one for « permission »). */
export async function stoppedAgent(call, root) {
  const repo = makeRepo(root);
  const project = await call('create_project', {
    path: repo,
    name: 'facturation',
    color: 'oklch(0.72 0.12 250)',
    worktreePerAgent: false,
    firstAgent: null,
    restore: false,
  });
  const agent = await call('create_agent', { projectId: project.id, model: null, engine: null, isolated: null, base: null });
  await call('send_message', { id: agent.id, text: 'Lance les tests, permission', attachments: [], files: [] });
}
