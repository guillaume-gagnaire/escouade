#!/usr/bin/env node
// Plays the cases of plugins/opencode/src/rules_verdicts.json on the real OpenCode: the rules of the plugin (as its crate
// writes them) on a real `opencode serve`, in a throwaway folder, with a scripted fake model on 127.0.0.1. No account, no
// token: its config folders are throwaway ones (XDG_*), and the only provider is the fake.
//
//   node scripts/opencode-real-rules.mjs            # ESCOUADE_PORT_BASE (4110) and the next one; OPENCODE_EXE to pick it
//
// Each case is one tool call of the model, every card rejected: a card is `ask`, a tool refused by a rule `deny`, a tool that
// ran `allow`. What `allow` runs runs in the throwaway folder. A subagent's edit is played too (the rules of the config).

import { execFileSync, execSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const crate = path.join(here, '..', 'plugins', 'opencode');
const base = Number(process.env.ESCOUADE_PORT_BASE ?? 4110);
const [modelPort, serverPort] = [base, base + 1];
const slashes = (p) => p.replaceAll('\\', '/');
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

function program() {
  if (process.env.OPENCODE_EXE) return process.env.OPENCODE_EXE;
  if (process.platform !== 'win32') return 'opencode';
  const npm = execSync('npm root -g', { encoding: 'utf8' }).trim();
  return path.join(npm, 'opencode-ai', 'bin', 'opencode.exe');
}

/** The model: a prompt `TOOL {"name","args"}` is one call of that tool, whatever follows is a text. */
function fakeModel() {
  const server = http.createServer((req, res) => {
    let body = '';
    req.on('data', (d) => (body += d));
    req.on('end', () => {
      const messages = JSON.parse(body || '{}').messages ?? [];
      const last = messages.at(-1);
      res.writeHead(200, { 'content-type': 'text/event-stream' });
      const send = (o) => res.write(`data: ${JSON.stringify(o)}\n\n`);
      const chunk = { id: 'c', object: 'chat.completion.chunk', created: 1, model: 'm' };
      const delta = (d, finish = null) => send({ ...chunk, choices: [{ index: 0, delta: d, finish_reason: finish }] });
      const text = typeof last?.content === 'string' ? last.content : (last?.content ?? []).map((c) => c.text ?? '').join('');
      const call = last?.role === 'user' ? text.match(/TOOL (\{.*\})/s) : null;
      if (call) {
        const { name, args } = JSON.parse(call[1]);
        const fn = { name, arguments: JSON.stringify(args) };
        delta({ role: 'assistant', tool_calls: [{ index: 0, id: `call_${Date.now()}`, type: 'function', function: fn }] });
        delta({}, 'tool_calls');
      } else {
        delta({ role: 'assistant', content: 'ok' });
        delta({}, 'stop');
      }
      send({ ...chunk, choices: [], usage: { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 } });
      res.end('data: [DONE]\n\n');
    });
  });
  return new Promise((r) => server.listen(modelPort, '127.0.0.1', () => r(server)));
}

async function startServer(root, config) {
  const xdg = path.join(root, 'xdg');
  for (const d of ['config/opencode', 'data', 'state', 'cache']) fs.mkdirSync(path.join(xdg, d), { recursive: true });
  const provider = {
    fake: {
      npm: '@ai-sdk/openai-compatible',
      name: 'Fake',
      options: { baseURL: `http://127.0.0.1:${modelPort}/v1`, apiKey: 'x' },
      models: { m: { name: 'm', tool_call: true } },
    },
  };
  fs.writeFileSync(
    path.join(xdg, 'config', 'opencode', 'opencode.json'),
    JSON.stringify({ provider, enabled_providers: ['fake'], model: 'fake/m', small_model: 'fake/m' }),
  );
  const password = 'throwaway';
  const child = spawn(program(), ['serve', '--hostname', '127.0.0.1', '--port', String(serverPort)], {
    cwd: root,
    stdio: ['ignore', 'pipe', 'pipe'],
    env: {
      ...process.env,
      XDG_CONFIG_HOME: path.join(xdg, 'config'),
      XDG_DATA_HOME: path.join(xdg, 'data'),
      XDG_STATE_HOME: path.join(xdg, 'state'),
      XDG_CACHE_HOME: path.join(xdg, 'cache'),
      OPENCODE_SERVER_PASSWORD: password,
      OPENCODE_DISABLE_AUTOUPDATE: '1',
      OPENCODE_CONFIG_CONTENT: JSON.stringify(config),
    },
  });
  let said = '';
  child.stdout.on('data', (d) => (said += d));
  child.stderr.on('data', (d) => (said += d));
  for (let i = 0; i < 300 && !said.includes('listening on'); i++) await wait(200);
  if (!said.includes('listening on')) throw new Error(`no server: ${said}`);

  const authorization = `Basic ${Buffer.from(`opencode:${password}`).toString('base64')}`;
  const url = `http://127.0.0.1:${serverPort}`;
  const call = async (method, where, dir, body) => {
    const u = new URL(url + where);
    if (dir) u.searchParams.set('directory', dir);
    const r = await fetch(u, {
      method,
      headers: { authorization, 'content-type': 'application/json' },
      body: body ? JSON.stringify(body) : undefined,
    });
    const text = await r.text();
    if (!r.ok) throw new Error(`${method} ${where}: ${r.status} ${text.slice(0, 300)}`);
    try {
      return JSON.parse(text);
    } catch {
      return text;
    }
  };
  const events = [];
  const stop = new AbortController();
  (async () => {
    const r = await fetch(`${url}/global/event`, { headers: { authorization }, signal: stop.signal });
    let buffer = '';
    const decoder = new TextDecoder();
    for await (const piece of r.body) {
      buffer += decoder.decode(piece, { stream: true });
      for (let i; (i = buffer.indexOf('\n\n')) >= 0; buffer = buffer.slice(i + 2)) {
        for (const line of buffer.slice(0, i).split('\n')) {
          if (line.startsWith('data:')) {
            const e = JSON.parse(line.slice(5));
            events.push(e.payload ?? e);
          }
        }
      }
    }
  })().catch(() => {});
  return {
    call,
    events,
    stop() {
      stop.abort();
      if (process.platform === 'win32') {
        try {
          execFileSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' });
        } catch {
          child.kill();
        }
      } else child.kill();
    },
  };
}

/** One call of the model in `session`, every card rejected: what became of it. */
async function play(server, session, dir, name, args, agent) {
  let seen = server.events.length;
  const body = {
    parts: [{ type: 'text', text: `TOOL ${JSON.stringify({ name, args })}` }],
    agent,
    model: { providerID: 'fake', modelID: 'm' },
  };
  await server.call('POST', `/session/${session}/prompt_async`, dir, body);
  const cards = [];
  let tool = null;
  let busy = false;
  for (const end = Date.now() + 60_000; Date.now() < end; await wait(25)) {
    for (; seen < server.events.length; seen++) {
      const { type, properties: p = {} } = server.events[seen];
      if (type === 'permission.asked') {
        cards.push(p);
        await server.call('POST', `/permission/${p.id}/reply`, dir, { reply: 'reject' });
      }
      if (type === 'message.part.updated' && p.part?.type === 'tool' && p.part.sessionID === session) tool = p.part.state;
      if (type === 'session.status' && p.sessionID === session && p.status?.type === 'busy') busy = true;
      if (type === 'session.idle' && p.sessionID === session && busy) {
        if (cards.length) return { got: 'ask', cards };
        if (tool?.status === 'error' && /rule which prevents/.test(tool.error ?? '')) return { got: 'deny' };
        return { got: 'allow', tool };
      }
    }
  }
  const last = server.events
    .slice(-4)
    .map((e) => e.type)
    .join(', ');
  return { got: `no end of turn (${tool?.status ?? 'no tool'}; last events: ${last})`, tool };
}

const root = fs.mkdtempSync(path.join(os.tmpdir(), 'escouade-opencode-rules-'));
const wt = slashes(fs.realpathSync(path.join(root))) + '/proj';
fs.mkdirSync(`${wt}/secret`, { recursive: true });
fs.mkdirSync(`${wt}/sub`);
fs.writeFileSync(`${wt}/sub/.git`, 'gitdir: elsewhere'); // As a worktree or a submodule has it.
fs.mkdirSync(`${root}/prive`);
fs.writeFileSync(`${root}/prive/x`, 'private');
fs.writeFileSync(`${wt}/msg.txt`, 'a message');
fs.writeFileSync(`${wt}/a.txt`, 'a file');
execSync('git init -q && git config user.email a@b.c && git config user.name a && git commit -q --allow-empty -m first', { cwd: wt });

const out = path.join(root, 'rules.json');
execSync('cargo test --quiet --lib dump_the_rules -- --ignored', {
  cwd: crate,
  stdio: ['ignore', 'ignore', 'inherit'],
  env: { ...process.env, ESCOUADE_RULES_OUT: out, ESCOUADE_RULES_WT: wt, ESCOUADE_RULES_UP: slashes(root) },
});
const rules = JSON.parse(fs.readFileSync(out, 'utf8'));
const { cases } = JSON.parse(fs.readFileSync(path.join(crate, 'src', 'rules_verdicts.json'), 'utf8'));

const model = await fakeModel();
const server = await startServer(root, rules.config);
const sessions = new Map();
const sessionOf = async (key) => {
  if (!sessions.has(key)) {
    const s = await server.call('POST', '/session', wt, { title: key, permission: rules[key] });
    sessions.set(key, s.id);
  }
  return sessions.get(key);
};

let wrong = 0;
try {
  const { worktree } = await server.call('GET', '/path', wt);
  console.log(`opencode ${execFileSync(program(), ['--version'], { encoding: 'utf8' }).trim()}, worktree ${worktree}`);
  for (const c of cases) {
    const key = c.fenced ? `${c.mode}+fence` : c.mode;
    const session = await sessionOf(key);
    const agent = c.agent ?? 'build';
    const r = c.edit
      ? await play(server, session, wt, 'write', { filePath: `${wt}/${c.edit}`, content: 'x' }, agent)
      : await play(server, session, wt, 'bash', { command: c.bash, description: 'a case' }, agent);
    const ok = r.got === c.want;
    wrong += ok ? 0 : 1;
    console.log(
      `${ok ? 'ok   ' : 'WRONG'} ${key.padEnd(24)} ${JSON.stringify(c.edit ?? c.bash)} => ${r.got}${ok ? '' : ` (wanted ${c.want})`}`,
    );
  }
  // A subagent takes the rules of the config alone.
  const session = await sessionOf('default');
  const edit = { name: 'write', args: { filePath: `${wt}/.git/hooks/from-a-subagent`, content: 'x' } };
  const task = { description: 'a task', prompt: `TOOL ${JSON.stringify(edit)}`, subagent_type: 'general' };
  const r = await play(server, session, wt, 'task', task, 'build');
  const asked = r.got === 'ask' && r.cards.some((c) => c.permission === 'edit');
  wrong += asked ? 0 : 1;
  console.log(`${asked ? 'ok   ' : 'WRONG'} subagent                 .git/hooks/from-a-subagent => ${r.got}`);
} finally {
  server.stop();
  model.close();
  await wait(2000); // Windows holds the folder a little after the end of its server.
  try {
    fs.rmSync(root, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
  } catch {
    console.log(`left: ${root}`);
  }
}
console.log(wrong ? `${wrong} case(s) wrong of ${cases.length + 1}` : `${cases.length + 1} cases as said`);
process.exit(wrong ? 1 : 0);
