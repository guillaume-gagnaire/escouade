#!/usr/bin/env node
// A fake `opencode serve` (OpenCode 1.19.3's HTTP API and its event stream `/global/event`), as the real streams
// of tests/fixtures/opencode/ show it: what the conformance bench (src-tauri/src/engine/opencode/bench.rs) and the
// end-to-end tests drive instead of OpenCode.
//
// `opencode --version` says its version (`FAKE_OPENCODE_VERSION`: another one, like `opencode v2.0.32`); `opencode models
// --verbose` lists its models;
// `opencode serve --port 0` listens on 127.0.0.1 and says where (« opencode server listening on http://… »), behind
// `OPENCODE_SERVER_PASSWORD` (Basic `opencode:<password>`, 401 otherwise). Its sessions are kept by folder in
// `<FAKE_OPENCODE_HOME | XDG_DATA_HOME | tmp/fake-opencode>/<hash of the folder>.json`, so that a server started again resumes
// them. It answers `/config/providers`, `/config`, `/session` (create, get, patch, messages, fork, prompt_async,
// abort, summarize), `/session/status`, `/permission/:id/reply`, `/question/:id/reply`, `/instance/dispose`. What a prompt plays is picked by its
// message (French words, as the scenarios of the fake `claude`):
// - "raisonnement": a reasoning (by deltas), then a reply;
// - "commande": `rm -rf build` (bash), which asks its permission under a rule `ask`: run once allowed, refused
//   otherwise (the message given said back);
// - "fichier": an edit of src/app.ts (written for real when it is there: `const a = 2;`), under the rules of `edit`;
// - "question": a question (`question.asked`), its answer said back;
// - "propose un plan", or the agent `plan`: a plan written by the plan agent; "Mets le plan en œuvre" /
//   "Implement the plan": a plain reply;
// - "étapes": a task list of three (`todowrite`, `todo.updated`), done one after the other;
// - "sous-agent": a subagent (`task`, a session of its own with its events), its report;
// - "lent": a reply, then `sleep 30` (bash, allowed), unless aborted (`MessageAbortedError`);
// - "deux appels": two calls to the model (a `read`, then the reply), each its `step-finish`;
// - "outil escouade <outil>": that tool of the MCP server the session's folder was given (`POST /mcp`), called;
// - anything else: « Bonjour, tu as dit : <message> ».
// Each call to the model costs what its tokens cost at the prices of the prompt's model (`cost` of its provider's model,
// per million tokens: none for the built-in local ones). `POST /mcp?directory=` connects a remote MCP server for that folder
// alone (initialize, tools/list, with its headers), `GET /mcp` and `POST /mcp/:name/disconnect` as OpenCode has them.
// `opencode run` and `opencode session delete` are played too (see below). FAKE_OPENCODE_PROVIDERS: a file of the providers
// of every account without its own `providers.json`.
// FAKE_OPENCODE_LOG: a file each launch appends to: {"launch": {argv, cwd, pid, config, password}} (config: its
// OPENCODE_CONFIG_CONTENT; password: whether it has one), then {"req": {method, path, directory, body}} for each
// request.

import crypto from 'node:crypto';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';

// Asked to name a task (<tache>, <task>, ESC-156): its first two words and "-opencode".
const nameOf = (asked) => {
  const task = asked.match(/<(tache|task)>\s*([\s\S]*?)\s*<\/\1>/)?.[2];
  return task === undefined ? null : `${(task.toLowerCase().match(/[a-z]+/g) ?? ['tache']).slice(0, 2).join('-')}-opencode`;
};

const VERSION = '1.19.3';
const argv = process.argv.slice(2);
if (argv.includes('--version') || argv[0] === '-v') {
  process.stdout.write(`${process.env.FAKE_OPENCODE_VERSION || VERSION}\n`);
  process.exit(0);
}
if (!['serve', 'models', 'attach', 'run', 'session'].includes(argv[0])) {
  process.stderr.write(`fake opencode: only serve, models, attach, run, session and --version are played (${argv.join(' ')})\n`);
  process.exit(2);
}

const LOG = process.env.FAKE_OPENCODE_LOG;
const log = (entry) => LOG && fs.appendFileSync(LOG, JSON.stringify(entry) + '\n');
let config = null;
try {
  config = JSON.parse(process.env.OPENCODE_CONFIG_CONTENT || 'null');
} catch {
  config = 'unreadable';
}
const PASSWORD = process.env.OPENCODE_SERVER_PASSWORD || '';
if (argv[0] === 'serve') log({ launch: { argv, cwd: process.cwd(), pid: process.pid, config, password: !!PASSWORD } });

// `opencode attach <url> --session <id> --dir <folder>`: a client of the server at <url> (its password in
// OPENCODE_SERVER_PASSWORD), in a terminal: it says the session's title, then what the session does (a text, a
// permission asked or answered, the end of a turn). Enter answers the permission waiting (« Allow once »); else what was
// typed is sent to the session as a message. Each launch is noted in `<OPENCODE_CONFIG_DIR>/attaches.jsonl`: its
// arguments and the password it was given, for the tests to find it never was on its command line (and in
// FAKE_OPENCODE_LOG: {"attach": {argv, password, passwordInArgv}}, whether it had one and saw it among its arguments).
if (argv[0] === 'attach') {
  const url = argv[1];
  const opt = (name) => (argv.indexOf(name) > 0 ? argv[argv.indexOf(name) + 1] : undefined);
  const session = opt('--session');
  const dir = opt('--dir') || process.cwd();
  log({ attach: { argv, password: !!PASSWORD, passwordInArgv: !!PASSWORD && argv.some((a) => a.includes(PASSWORD)) } });
  const dirOf = process.env.OPENCODE_CONFIG_DIR;
  if (dirOf) {
    fs.mkdirSync(dirOf, { recursive: true });
    fs.appendFileSync(path.join(dirOf, 'attaches.jsonl'), JSON.stringify({ argv, password: PASSWORD, pid: process.pid }) + '\n');
  }
  const headers = { authorization: 'Basic ' + Buffer.from(`opencode:${PASSWORD}`).toString('base64'), 'content-type': 'application/json' };
  const q = `?directory=${encodeURIComponent(dir)}`;
  // What it shows goes to the log too ({"shown": line}): the window draws it where a test cannot read it (WebGL).
  const say = (line) => {
    log({ shown: line });
    process.stdout.write(line + '\r\n');
  };
  const got = await fetch(`${url}/session/${session}${q}`, { headers }).catch((e) => ({ ok: false, status: String(e) }));
  if (!got.ok) {
    say(`fake opencode attach: refused (${got.status})`);
    process.exit(1);
  }
  const info = await got.json();
  say(`fake opencode attach: ${info.title} (${session})`);
  let waiting = null;
  let typed = '';
  const events = await fetch(`${url}/global/event`, { headers });
  (async () => {
    const decoder = new TextDecoder();
    let buf = '';
    for await (const chunk of events.body) {
      buf += decoder.decode(chunk, { stream: true });
      let at;
      while ((at = buf.indexOf('\n\n')) >= 0) {
        const frame = buf.slice(0, at);
        buf = buf.slice(at + 2);
        if (!frame.startsWith('data: ')) continue;
        const { type, properties: p = {} } = JSON.parse(frame.slice(6)).payload || {};
        if (p.sessionID !== session) continue;
        if (type === 'permission.asked') {
          waiting = p.id;
          say(`permission ${p.permission}: ${(p.patterns || []).join(' ')} (Enter: allow once)`);
        } else if (type === 'permission.replied') {
          if (waiting === p.requestID) waiting = null;
          say(`permission answered: ${p.reply}`);
        } else if (type === 'message.part.updated' && p.part?.type === 'text' && p.part.time?.end) {
          say(`text: ${p.part.text}`);
        } else if (type === 'session.idle') {
          say('turn over');
        }
      }
    }
    say('fake opencode attach: the server is gone');
    process.exit(1);
  })();
  if (process.stdin.isTTY) process.stdin.setRawMode(true);
  process.stdin.on('data', async (d) => {
    for (const c of d.toString('utf8')) {
      if (c === '\x03') process.exit(0);
      if (c !== '\r') {
        typed += c;
        continue;
      }
      if (waiting) {
        const rid = waiting;
        await fetch(`${url}/permission/${rid}/reply${q}`, { method: 'POST', headers, body: JSON.stringify({ reply: 'once' }) });
      } else if (typed.trim()) {
        // Without what the terminal answered (the cursor's position, on Windows).
        const text = typed.replace(/\x1b\[[0-9;?]*[A-Za-z]/g, '').trim();
        say(`> ${text}`);
        await fetch(`${url}/session/${session}/prompt_async${q}`, {
          method: 'POST',
          headers,
          body: JSON.stringify({ parts: [{ type: 'text', text }] }),
        });
      }
      typed = '';
    }
  });
  await new Promise(() => {});
}

const HOME = process.env.FAKE_OPENCODE_HOME || process.env.XDG_DATA_HOME || path.join(os.tmpdir(), 'fake-opencode');
fs.mkdirSync(HOME, { recursive: true });
// The sessions of each folder, in a file of their own (the tests run many servers at once).
const stores = new Map();
function storeOf(dir) {
  const key = crypto.createHash('sha1').update(path.resolve(dir).toLowerCase()).digest('hex').slice(0, 16);
  if (!stores.has(key)) {
    let all = {};
    try {
      all = JSON.parse(fs.readFileSync(path.join(HOME, `${key}.json`), 'utf8'));
    } catch {}
    stores.set(key, { key, all });
  }
  return stores.get(key);
}
const sessionOf = (dir, sid) => storeOf(dir).all[sid];
function save(dir) {
  const st = storeOf(dir);
  fs.writeFileSync(path.join(HOME, `${st.key}.json`), JSON.stringify(st.all));
}

let serial = 0;
const id = (prefix) => `${prefix}_${Date.now().toString(16)}${String(++serial).padStart(4, '0')}${process.pid}`;
const now = () => Date.now();

const BUILT_IN = {
  providers: [
    {
      id: 'ollama',
      name: 'Ollama (local)',
      source: 'config',
      models: {
        'qwen3-coder:30b-32k': {
          id: 'qwen3-coder:30b-32k',
          providerID: 'ollama',
          name: 'Qwen3 Coder 30b (32k)',
          capabilities: { reasoning: false, attachment: false, toolcall: true, input: { text: true, image: false } },
          limit: { context: 32768, output: 8192 },
          status: 'active',
        },
        'gpt-oss:20b-32k': {
          id: 'gpt-oss:20b-32k',
          providerID: 'ollama',
          name: 'gpt-oss 20b (32k)',
          capabilities: { reasoning: true, attachment: false, toolcall: true, input: { text: true, image: false } },
          limit: { context: 32768, output: 8192 },
          status: 'active',
          variants: { low: { reasoningEffort: 'low' }, medium: { reasoningEffort: 'medium' }, high: { reasoningEffort: 'high' } },
        },
      },
    },
    {
      id: 'lmstudio',
      name: 'LM Studio (local)',
      source: 'config',
      models: {
        'devstral-small': {
          id: 'devstral-small',
          providerID: 'lmstudio',
          name: 'Devstral Small',
          capabilities: { reasoning: false, attachment: true, toolcall: true, input: { text: true, image: true } },
          limit: { context: 65536, output: 8192 },
          status: 'active',
        },
      },
    },
  ],
  default: { ollama: 'qwen3-coder:30b-32k', lmstudio: 'devstral-small' },
};
// An account's own providers (`<OPENCODE_CONFIG_DIR>/providers.json`, as `/config/providers` answers), else the built-in
// ones; its launches noted there too (`launches.jsonl`), for the tests of the core.
const ACCOUNT = process.env.OPENCODE_CONFIG_DIR;
let PROVIDERS = BUILT_IN;
try {
  PROVIDERS = JSON.parse(fs.readFileSync(path.join(ACCOUNT, 'providers.json'), 'utf8'));
} catch {
  // Those of every account (FAKE_OPENCODE_PROVIDERS: a file), Principal's included (the end-to-end tests).
  try {
    PROVIDERS = JSON.parse(fs.readFileSync(process.env.FAKE_OPENCODE_PROVIDERS, 'utf8'));
  } catch {}
}
if (ACCOUNT) {
  fs.mkdirSync(ACCOUNT, { recursive: true });
  fs.appendFileSync(path.join(ACCOUNT, 'launches.jsonl'), JSON.stringify({ argv, config }) + '\n');
}

// `opencode models --verbose`: each model's `<provider>/<model>` line, then its JSON.
if (argv[0] === 'models') {
  log({ models: { argv } });
  {
    for (const p of PROVIDERS.providers) {
      for (const [mid, m] of Object.entries(p.models)) process.stdout.write(`${p.id}/${mid}\n${JSON.stringify(m, null, 2)}\n`);
    }
  }
  process.exit(0);
}

// The sessions `opencode run` kept, which `opencode session delete <id>` deletes (`<HOME>/run-sessions.json`).
const RUNS = path.join(HOME, 'run-sessions.json');
const runSessions = () => {
  try {
    return JSON.parse(fs.readFileSync(RUNS, 'utf8'));
  } catch {
    return [];
  }
};
if (argv[0] === 'session') {
  if (argv[1] !== 'delete' || !argv[2]) process.exit(2);
  const all = runSessions();
  if (!all.includes(argv[2])) {
    process.stderr.write(`Session not found: ${argv[2]}\n`);
    process.exit(1);
  }
  fs.writeFileSync(RUNS, JSON.stringify(all.filter((s) => s !== argv[2])));
  log({ deleted: argv[2] });
  process.stdout.write(`Session ${argv[2]} deleted\n`);
  process.exit(0);
}
// `opencode run --format json --model <provider/model> --agent <agent> …`, its message read from its input: a session kept
// (`run-sessions.json`), then « Réponse d’OpenCode (<model>) : <message> » as a part `text`; a model none of the providers
// has, or the message « échoue », ends on an event `error`. Noted in FAKE_OPENCODE_LOG and the account's `launches.jsonl`
// with its message (`{"run": {argv, cwd, config, prompt}}`).
if (argv[0] === 'run') {
  let prompt = '';
  for await (const chunk of process.stdin) prompt += chunk;
  const opt = (name) => {
    const at = argv.indexOf(name);
    return at >= 0 ? argv[at + 1] : undefined;
  };
  const model = opt('--model') || '';
  log({ run: { argv, cwd: process.cwd(), config, prompt } });
  if (ACCOUNT && fs.existsSync(ACCOUNT))
    fs.appendFileSync(path.join(ACCOUNT, 'launches.jsonl'), JSON.stringify({ run: { argv, config, prompt } }) + '\n');
  const sid = `ses_run${Date.now().toString(16)}${process.pid}`;
  fs.writeFileSync(RUNS, JSON.stringify([...runSessions(), sid]));
  const out = (v) => process.stdout.write(JSON.stringify({ timestamp: Date.now(), sessionID: sid, ...v }) + '\n');
  const [pid, ...rest] = model.split('/');
  const known = PROVIDERS.providers.find((p) => p.id === pid)?.models?.[rest.join('/')];
  if (!known || /échoue/.test(prompt)) {
    out({ type: 'error', error: { name: 'UnknownError', data: { message: 'Unexpected server error. Check server logs for details.' } } });
    process.exit(1);
  }
  const part = { id: 'prt_run', messageID: 'msg_run', sessionID: sid };
  out({ type: 'step_start', part: { ...part, id: 'prt_start', type: 'step-start' } });
  out({ type: 'text', part: { ...part, type: 'text', text: nameOf(prompt) ?? `Réponse d’OpenCode (${model}) : ${prompt.trim()}` } });
  out({ type: 'step_finish', part: { ...part, id: 'prt_end', type: 'step-finish', reason: 'stop', cost: 0 } });
  process.exit(0);
}

// The event stream's clients, and the events sent to them.
const clients = new Set();
function emit(directory, type, properties) {
  const line = `data: ${JSON.stringify({ directory, payload: { id: id('evt'), type, properties } })}\n\n`;
  for (const res of clients) res.write(line);
}

// What waits for an answer: permissions and questions, by id.
const asked = new Map();
// The turn under way of each session: whether it was aborted, and what wakes it.
const turns = new Map();

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** `POST /session/:id/summarize`, as OpenCode 1.19.3 does it (SessionCompaction): a message of the user's holding a part
 * `compaction`, the summary written by the model (`summary: true`, its `step-finish` says what it holds), then
 * `session.compacted` and `idle`. Answers once it is over. */
async function summarize(s, body) {
  const dir = s.directory;
  const sid = s.id;
  emit(dir, 'session.status', { sessionID: sid, status: { type: 'busy' } });
  const user = { id: id('msg'), sessionID: sid, role: 'user', time: { created: now() }, model: body };
  const marker = { id: id('prt'), type: 'compaction', auto: false, messageID: user.id, sessionID: sid };
  s.messages.push({ info: user, parts: [marker] });
  emit(dir, 'message.updated', { sessionID: sid, info: user });
  emit(dir, 'message.part.updated', { sessionID: sid, part: marker, time: now() });
  await sleep(30);
  const msg = {
    id: id('msg'),
    parentID: user.id,
    sessionID: sid,
    role: 'assistant',
    agent: 'compaction',
    mode: 'compaction',
    summary: true,
    modelID: body.modelID,
    providerID: body.providerID,
    time: { created: now() },
    cost: 0,
    tokens: { input: 0, output: 0, reasoning: 0, cache: { read: 0, write: 0 } },
  };
  emit(dir, 'message.updated', { sessionID: sid, info: msg });
  const text = { id: id('prt'), type: 'text', text: 'Résumé de la conversation (summarize).', messageID: msg.id, sessionID: sid };
  text.time = { start: now(), end: now() };
  emit(dir, 'message.part.updated', { sessionID: sid, part: text, time: now() });
  const tokens = { input: 900, output: 40, reasoning: 0, cache: { read: 0, write: 0 } };
  const finish = {
    id: id('prt'),
    type: 'step-finish',
    reason: 'stop',
    tokens: { total: 940, ...tokens },
    cost: 0,
    messageID: msg.id,
    sessionID: sid,
  };
  emit(dir, 'message.part.updated', { sessionID: sid, part: finish, time: now() });
  msg.tokens = tokens;
  msg.finish = 'stop';
  msg.time.completed = now();
  s.messages.push({ info: msg, parts: [text, finish] });
  emit(dir, 'message.updated', { sessionID: sid, info: msg });
  emit(dir, 'session.compacted', { sessionID: sid });
  emit(dir, 'session.status', { sessionID: sid, status: { type: 'idle' } });
  emit(dir, 'session.idle', { sessionID: sid });
  save(dir);
}

/** The action of the rules of `session` for `permission` on `pattern` (the last that matches wins). */
function action(session, permission, pattern) {
  const glob = (g) => new RegExp('^' + g.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*') + '$', 's');
  let found = null;
  for (const r of session.permission || []) {
    if ((r.permission === permission || r.permission === '*') && glob(r.pattern).test(pattern)) found = r.action;
  }
  if ((session.approved || []).some((p) => glob(p).test(pattern))) return 'allow';
  return found || (permission === 'external_directory' ? 'ask' : 'allow');
}

function sessionInfo(s) {
  const { messages, approved, ...info } = s;
  return info;
}

function newSession(directory, body = {}) {
  const sid = id('ses');
  const s = {
    id: sid,
    slug: 'fake',
    version: VERSION,
    directory,
    title: body.title || `New session - ${new Date().toISOString()}`,
    parentID: body.parentID,
    permission: body.permission || [],
    time: { created: now(), updated: now() },
    messages: [],
  };
  storeOf(directory).all[sid] = s;
  save(directory);
  emit(directory, 'session.created', { sessionID: sid, info: sessionInfo(s) });
  return s;
}

/** What `tokens` cost on the prompt's model, at its prices per million tokens (`cost`, as OpenCode's models have them): 0 for a
 * model without a price (a local one). */
function priceOf(model, tokens) {
  const m = PROVIDERS.providers.find((p) => p.id === model?.providerID)?.models?.[model?.modelID];
  const c = m?.cost;
  if (!c) return 0;
  return (
    (tokens.input * (c.input || 0) +
      tokens.output * (c.output || 0) +
      tokens.cache.read * (c.cache_read || 0) +
      tokens.cache.write * (c.cache_write || 0)) /
    1e6
  );
}

// The MCP servers added by `POST /mcp`, by folder (an instance's own): `{name: {status, url, headers, tools, session}}`.
const mcps = new Map();
const mcpOf = (dir) => {
  const key = path.resolve(dir).toLowerCase();
  if (!mcps.has(key)) mcps.set(key, {});
  return mcps.get(key);
};
/** A JSON-RPC request (`id`) or notification to a remote MCP server, as OpenCode's client sends it: its result. */
async function mcpCall(e, method, params, id) {
  const headers = { 'content-type': 'application/json', accept: 'application/json, text/event-stream', ...(e.headers || {}) };
  if (e.session) headers['mcp-session-id'] = e.session;
  const msg = id === undefined ? { jsonrpc: '2.0', method, params } : { jsonrpc: '2.0', id, method, params };
  const r = await fetch(e.url, { method: 'POST', headers, body: JSON.stringify(msg) });
  const session = r.headers.get('mcp-session-id');
  if (session) e.session = session;
  const text = await r.text();
  if (!r.ok) throw new Error(`${r.status} ${text}`);
  if (id === undefined) return null;
  const answers = /^data:/m.test(text)
    ? text
        .split('\n')
        .filter((l) => l.startsWith('data:'))
        .map((l) => JSON.parse(l.slice(5)))
    : [JSON.parse(text)];
  const answer = answers.find((a) => a.id === id);
  if (!answer) throw new Error(`no answer to ${method}`);
  if (answer.error) throw new Error(answer.error.message);
  return answer.result;
}
/** `config` (a remote MCP server) connected for the folder: initialized, its tools listed. */
async function connect(config) {
  const e = { status: 'disabled', url: config.url, headers: config.headers || {}, tools: [] };
  if (config.enabled === false) return e;
  try {
    await mcpCall(
      e,
      'initialize',
      { protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 'opencode', version: VERSION } },
      1,
    );
    await mcpCall(e, 'notifications/initialized', {});
    e.tools = ((await mcpCall(e, 'tools/list', {}, 2)).tools || []).map((t) => t.name);
    e.status = 'connected';
  } catch (err) {
    e.status = 'failed';
    e.error = String(err.message || err);
  }
  return e;
}
const statusOf = (e) => (e.status === 'failed' ? { status: 'failed', error: e.error } : { status: e.status });
const statuses = (dir) => Object.fromEntries(Object.entries(mcpOf(dir)).map(([n, e]) => [n, statusOf(e)]));

/** One turn of `session`, from the prompt `body`. */
async function play(s, body) {
  let m;
  const dir = s.directory;
  const sid = s.id;
  const text = (body.parts || [])
    .filter((p) => p.type === 'text')
    .map((p) => p.text)
    .join('\n');
  const turn = { aborted: false, wake: null };
  // Settled once the turn is aborted, whenever it is (`/session/:id/abort`).
  turn.stopped = new Promise((resolve) => (turn.wake = resolve));
  turns.set(sid, turn);
  const user = {
    id: id('msg'),
    sessionID: sid,
    role: 'user',
    time: { created: now() },
    agent: body.agent,
    model: body.model,
    system: body.system,
  };
  s.messages.push({ info: user, parts: [{ id: id('prt'), type: 'text', text, messageID: user.id, sessionID: sid }] });
  emit(dir, 'message.updated', { sessionID: sid, info: user });
  emit(dir, 'message.part.updated', { sessionID: sid, part: s.messages.at(-1).parts[0], time: now() });
  emit(dir, 'session.status', { sessionID: sid, status: { type: 'busy' } });
  const plan = body.agent === 'plan' || /propose un plan/i.test(text);
  const agent = plan ? 'plan' : 'build';
  const msg = {
    id: id('msg'),
    parentID: user.id,
    sessionID: sid,
    role: 'assistant',
    agent,
    mode: agent,
    modelID: body.model?.modelID,
    providerID: body.model?.providerID,
    time: { created: now() },
    cost: 0,
    tokens: { input: 0, output: 0, reasoning: 0, cache: { read: 0, write: 0 } },
  };
  const parts = [];
  s.messages.push({ info: msg, parts });
  emit(dir, 'message.updated', { sessionID: sid, info: msg });
  const part = (p) => {
    const full = { id: p.id || id('prt'), messageID: msg.id, sessionID: sid, ...p };
    const at = parts.findIndex((x) => x.id === full.id);
    if (at >= 0) parts[at] = full;
    else parts.push(full);
    emit(dir, 'message.part.updated', { sessionID: sid, part: full, time: now() });
    return full;
  };
  const say = async (words, kind = 'text') => {
    const p = part({ type: kind, text: '', time: { start: now() } });
    let all = '';
    for (const w of words.match(/\S+\s*/g) || [words]) {
      if (turn.aborted) break;
      all += w;
      emit(dir, 'message.part.delta', { sessionID: sid, messageID: msg.id, partID: p.id, field: 'text', delta: w });
      await sleep(5);
    }
    part({ ...p, text: all, time: { start: p.time.start, end: now() } });
  };
  const tool = (name, input, callID = id('call')) => {
    const p = part({ type: 'tool', tool: name, callID, state: { status: 'pending', input: {}, raw: '' } });
    return part({ ...p, state: { status: 'running', input, time: { start: now() } } });
  };
  const done = (p, output, metadata = {}) =>
    part({
      ...p,
      state: { status: 'completed', input: p.state.input, output, metadata, title: '', time: { start: p.state.time.start, end: now() } },
    });
  const failed = (p, error) =>
    part({ ...p, state: { status: 'error', input: p.state.input, error, time: { start: p.state.time.start, end: now() } } });
  /** Waits for the answer to the permission `permission` on `pattern` when its rules ask; its reply. */
  const permit = async (p, permission, pattern, metadata, always) => {
    if (turn.aborted) return { reply: 'aborted' };
    const act = action(s, permission, pattern);
    if (act === 'allow') return { reply: 'once' };
    if (act === 'deny') return { reply: 'reject', message: 'denied by a rule' };
    const rid = id('per');
    const answer = new Promise((resolve) => asked.set(rid, { kind: 'permission', resolve, session: s, always }));
    emit(dir, 'permission.asked', {
      id: rid,
      sessionID: sid,
      permission,
      patterns: [pattern],
      metadata,
      always,
      tool: { messageID: msg.id, callID: p.callID },
    });
    const r = await Promise.race([answer, turn.stopped.then(() => ({ reply: 'aborted' }))]);
    asked.delete(rid);
    if (r.reply !== 'aborted') emit(dir, 'permission.replied', { sessionID: sid, requestID: rid, reply: r.reply });
    return r;
  };
  /** One call to the model over: its tokens, and its cost at the model's prices. */
  const step = (reason, tokens) => {
    const cost = priceOf(body.model, tokens);
    part({ type: 'step-finish', reason, tokens: { total: tokens.input + tokens.output + tokens.cache.read, ...tokens }, cost });
    msg.cost += cost;
  };
  const finish = (error) => {
    step('stop', { input: 100, output: 10, reasoning: 0, cache: { read: 20, write: 0 } });
    msg.tokens = { input: 100, output: 10, reasoning: 0, cache: { read: 20, write: 0 } };
    msg.finish = 'stop';
    msg.time.completed = now();
    emit(dir, 'message.updated', { sessionID: sid, info: msg });
    if (error) emit(dir, 'session.error', { sessionID: sid, error });
    emit(dir, 'session.status', { sessionID: sid, status: { type: 'idle' } });
    emit(dir, 'session.idle', { sessionID: sid });
    turns.delete(sid);
    save(dir);
  };
  const aborted = () => finish({ name: 'MessageAbortedError', data: { message: 'Aborted' } });

  part({ type: 'step-start' });
  if (plan) {
    await say('Voici mon plan :\n\n1. Lire le code\n2. Écrire le correctif\n3. Lancer les tests');
  } else if (/raisonnement/i.test(text)) {
    await say('Je réfléchis à la question avant de répondre.', 'reasoning');
    await say('Voici ma réponse réfléchie.');
  } else if (/commande/i.test(text)) {
    const p = tool('bash', { command: 'rm -rf build', description: 'Supprime build' });
    const r = await permit(p, 'bash', 'rm -rf build', { command: 'rm -rf build' }, ['rm *']);
    if (r.reply === 'aborted') return aborted();
    if (r.reply === 'reject') {
      failed(
        p,
        r.message
          ? `The user rejected permission to use this specific tool call with the following feedback: ${r.message}`
          : 'The user rejected permission to use this specific tool call.',
      );
      await say(`Je ne lance pas la commande${r.message ? ` : ${r.message}` : '.'}`);
    } else {
      done(p, 'build supprimé', { exit: 0, output: 'build supprimé' });
      await say('La commande est passée.');
    }
  } else if (/fichier/i.test(text)) {
    const file = path.join(dir, 'src', 'app.ts');
    const p = tool('edit', { filePath: file, oldString: 'const a = 1;', newString: 'const a = 2;' });
    const r = await permit(
      p,
      'edit',
      path.relative(dir, file).split(path.sep).join('/'),
      { filepath: file, diff: '@@ -1 +1 @@\n-const a = 1;\n+const a = 2;\n' },
      ['*'],
    );
    if (r.reply === 'aborted') return aborted();
    if (r.reply === 'reject') {
      failed(p, 'The user rejected permission to use this specific tool call.');
    } else {
      try {
        if (fs.existsSync(file)) fs.writeFileSync(file, 'const a = 2;\n');
      } catch {}
      const patch = `Index: ${file}\n===================================================================\n--- ${file}\n+++ ${file}\n@@ -1,1 +1,1 @@\n-const a = 1;\n+const a = 2;\n`;
      done(p, 'Edit applied successfully.', { diff: patch, filediff: { file, patch, additions: 1, deletions: 1 } });
      part({ type: 'patch', hash: 'fake', files: [file.split(path.sep).join('/')] });
    }
    await say('Le fichier est modifié.');
  } else if (/question/i.test(text)) {
    const p = tool('question', { questions: [] });
    const rid = id('que');
    const questions = [
      {
        question: 'Quel langage préfères-tu ?',
        header: 'Langage',
        options: [
          { label: 'Python', description: 'Simple et lisible' },
          { label: 'Rust', description: 'Rapide et sûr' },
        ],
        multiple: false,
      },
    ];
    const answer = new Promise((resolve) => asked.set(rid, { kind: 'question', resolve, session: s }));
    emit(dir, 'question.asked', { id: rid, sessionID: sid, questions, tool: { messageID: msg.id, callID: p.callID } });
    const r = await Promise.race([answer, turn.stopped.then(() => null)]);
    asked.delete(rid);
    if (!r) return aborted();
    emit(dir, 'question.replied', { sessionID: sid, requestID: rid, answers: r.answers });
    const chosen = (r.answers?.[0] || []).join(', ');
    done(p, `User has answered your questions: "${questions[0].question}"="${chosen}".`, { answers: r.answers });
    await say(`Tu as choisi : ${chosen}`);
  } else if (/étapes/i.test(text)) {
    const steps = ['Lire le code', 'Écrire le correctif', 'Lancer les tests'];
    for (let n = 0; n <= steps.length; n++) {
      const todos = steps.map((content, i) => ({
        content,
        status: i < n ? 'completed' : i === n ? 'in_progress' : 'pending',
        priority: 'high',
      }));
      const p = tool('todowrite', { todos });
      emit(dir, 'todo.updated', { sessionID: sid, todos });
      done(p, JSON.stringify(todos, null, 2), { todos });
      await sleep(Number(process.env.FAKE_OPENCODE_STEP_MS || 50));
    }
    await say('Les trois étapes sont faites.');
  } else if (/sous-agent/i.test(text)) {
    const callID = id('call');
    const p = tool('task', { description: 'Explorer le dépôt', prompt: 'Liste les fichiers', subagent_type: 'explore' }, callID);
    const child = newSession(dir, { parentID: sid, title: 'Explorer le dépôt (@explore subagent)' });
    part({ ...p, state: { ...p.state, metadata: { parentSessionId: sid, sessionId: child.id } } });
    const cmsg = { id: id('msg'), sessionID: child.id, role: 'assistant', agent: 'explore', time: { created: now() } };
    emit(dir, 'session.status', { sessionID: child.id, status: { type: 'busy' } });
    emit(dir, 'message.updated', { sessionID: child.id, info: cmsg });
    const cpart = (q) =>
      emit(dir, 'message.part.updated', {
        sessionID: child.id,
        part: { id: id('prt'), messageID: cmsg.id, sessionID: child.id, ...q },
        time: now(),
      });
    cpart({
      type: 'tool',
      tool: 'glob',
      callID: id('call'),
      state: {
        status: 'completed',
        input: { pattern: '**/*' },
        output: 'README.md',
        metadata: { count: 1 },
        title: '',
        time: { start: now(), end: now() },
      },
    });
    // Slowed down by FAKE_OPENCODE_SUB_MS: the subagent is seen running.
    await sleep(Number(process.env.FAKE_OPENCODE_SUB_MS || 0));
    cpart({ type: 'text', text: 'Un seul fichier : README.md.', time: { start: now(), end: now() } });
    cpart({
      type: 'step-finish',
      reason: 'stop',
      tokens: { total: 60, input: 50, output: 10, reasoning: 0, cache: { read: 0, write: 0 } },
      cost: 0,
    });
    emit(dir, 'session.status', { sessionID: child.id, status: { type: 'idle' } });
    emit(dir, 'session.idle', { sessionID: child.id });
    done(p, `<task id="${child.id}" state="completed">\n<task_result>\nUn seul fichier : README.md.\n</task_result>\n</task>`, {
      parentSessionId: sid,
      sessionId: child.id,
    });
    await say('Le sous-agent a fini.');
  } else if (/lent/i.test(text)) {
    await say('Je commence une commande longue.');
    const p = tool('bash', { command: 'sleep 30', description: 'Attend' });
    const r = await permit(p, 'bash', 'sleep 30', { command: 'sleep 30' }, ['sleep *']);
    if (r.reply === 'aborted') return aborted();
    const woke = await Promise.race([sleep(30_000).then(() => false), turn.stopped.then(() => true)]);
    if (woke) {
      done(p, 'User aborted the command', { exit: null });
      return aborted();
    }
    done(p, '(no output)', { exit: 0 });
    await say('Fini.');
  } else if (/deux appels/i.test(text)) {
    // A first call to the model ends on a tool, the turn's second one replies.
    const p = tool('read', { filePath: path.join(dir, 'README.md') });
    done(p, '<file>\n00001| Fake\n</file>');
    step('tool-calls', { input: 1000, output: 50, reasoning: 0, cache: { read: 0, write: 0 } });
    part({ type: 'step-start' });
    await say('Le fichier est lu.');
  } else if ((m = text.match(/outil escouade (\w+)/i))) {
    // A tool of Escouade's server, through the entry of the session's folder (`POST /mcp`).
    const e = mcpOf(dir).escouade;
    if (e?.status !== 'connected' || !e.tools.includes(m[1])) {
      await say('Escouade n’est pas là.');
    } else {
      const p = tool(`escouade_${m[1]}`, {});
      try {
        const r = await mcpCall(e, 'tools/call', { name: m[1], arguments: {} }, 3);
        done(p, (r.content || []).map((c) => c.text).join('\n'));
      } catch (err) {
        failed(p, String(err.message || err));
      }
      await say('L’outil a répondu.');
    }
  } else if (/Mets le plan en œuvre|Implement the plan/i.test(text)) {
    await say("C'est fait.");
  } else {
    await say(`Bonjour, tu as dit : ${text}${body.variant ? ` (variante ${body.variant})` : ''}`);
  }
  if (turn.aborted) return aborted();
  finish();
}

function readBody(req) {
  return new Promise((resolve) => {
    let data = '';
    req.setEncoding('utf8');
    req.on('data', (d) => (data += d));
    req.on('end', () => {
      try {
        resolve(data ? JSON.parse(data) : null);
      } catch {
        resolve(null);
      }
    });
  });
}

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, 'http://127.0.0.1');
  const auth = req.headers.authorization || '';
  const expected = 'Basic ' + Buffer.from(`opencode:${PASSWORD}`).toString('base64');
  if (PASSWORD && auth !== expected) {
    res.writeHead(401).end('Unauthorized');
    return;
  }
  // What the test of the parity asks, out of OpenCode's API: how many read the stream, and its cut.
  if (url.pathname === '/fake/readers') return res.end(String(clients.size));
  if (url.pathname === '/fake/cut') {
    for (const client of clients) {
      clients.delete(client);
      client.destroy();
    }
    return res.end('cut');
  }
  const body = await readBody(req);
  const dir = url.searchParams.get('directory') || process.cwd();
  const p = url.pathname;
  log({ req: { method: req.method, path: p, directory: url.searchParams.get('directory'), body } });
  // In its account's folder too: the tests of an account read it there.
  if (ACCOUNT && fs.existsSync(ACCOUNT))
    fs.appendFileSync(path.join(ACCOUNT, 'requests.jsonl'), JSON.stringify({ method: req.method, path: p, body }) + '\n');
  const json = (v, status = 200) => {
    res.writeHead(status, { 'content-type': 'application/json' });
    res.end(JSON.stringify(v));
  };
  const notFound = () => json({ name: 'NotFoundError', data: { message: `not found: ${p}` } }, 404);
  let m;
  if (req.method === 'GET' && p === '/global/event') {
    res.writeHead(200, { 'content-type': 'text/event-stream', 'cache-control': 'no-cache' });
    res.write(`data: ${JSON.stringify({ payload: { id: id('evt'), type: 'server.connected', properties: {} } })}\n\n`);
    clients.add(res);
    req.on('close', () => clients.delete(res));
    return;
  }
  if (req.method === 'GET' && p === '/config/providers') return json(PROVIDERS);
  if (req.method === 'GET' && p === '/config') return json({ ...(config && typeof config === 'object' ? config : {}) });
  if (req.method === 'POST' && p === '/instance/dispose') {
    // The folder's instance goes, and the MCP servers added to it.
    mcps.delete(path.resolve(dir).toLowerCase());
    return json(true);
  }
  if (p === '/mcp' && req.method === 'GET') return json(statuses(dir));
  if (p === '/mcp' && req.method === 'POST') {
    if (!body?.name || body.config?.type !== 'remote') return json({ name: 'BadRequest', data: { message: 'a remote server' } }, 400);
    mcpOf(dir)[body.name] = await connect(body.config);
    return json(statuses(dir));
  }
  if ((m = p.match(/^\/mcp\/([^/]+)\/disconnect$/)) && req.method === 'POST') {
    const e = mcpOf(dir)[m[1]];
    if (!e) return notFound();
    e.status = 'disabled';
    return json(true);
  }
  // The sessions that run, as OpenCode answers it: an idle one is not listed.
  if (req.method === 'GET' && p === '/session/status') {
    return json(Object.fromEntries([...turns.keys()].map((sid) => [sid, { type: 'busy' }])));
  }
  // What OpenCode judges an edit against: the folder itself here, as a repository's root.
  if (req.method === 'GET' && p === '/path') return json({ worktree: dir, directory: dir });
  if (req.method === 'POST' && p === '/session') return json(sessionInfo(newSession(dir, body || {})));
  if ((m = p.match(/^\/session\/([^/]+)$/))) {
    const s = sessionOf(dir, m[1]);
    if (!s) return notFound();
    if (req.method === 'GET') return json(sessionInfo(s));
    if (req.method === 'PATCH') {
      if (body?.title !== undefined) s.title = body.title;
      if (body?.permission) s.permission = [...(s.permission || []), ...body.permission];
      save(s.directory);
      emit(s.directory, 'session.updated', { sessionID: s.id, info: sessionInfo(s) });
      return json(sessionInfo(s));
    }
  }
  if ((m = p.match(/^\/session\/([^/]+)\/(message|fork|prompt_async|abort|summarize)$/))) {
    const s = sessionOf(dir, m[1]);
    if (!s) return notFound();
    switch (m[2]) {
      case 'message':
        return json(s.messages);
      case 'fork': {
        const copy = newSession(s.directory, { title: `${s.title} (fork #1)` });
        const at = body?.messageID ? s.messages.findIndex((x) => x.info.id === body.messageID) : -1;
        copy.messages = JSON.parse(JSON.stringify(at < 0 ? s.messages : s.messages.slice(0, at)));
        save(s.directory);
        return json(sessionInfo(copy));
      }
      case 'summarize':
        if (turns.has(s.id)) return json({ name: 'BusyError', data: { message: `Session ${s.id} is busy` } }, 400);
        summarize(s, body || {})
          .then(() => json(true))
          .catch((e) =>
            process.stderr.write(`fake opencode: ${e.stack}
`),
          );
        return;
      case 'prompt_async':
        res.writeHead(204).end();
        play(s, body || {}).catch((e) => process.stderr.write(`fake opencode: ${e.stack}\n`));
        return;
      case 'abort': {
        const t = turns.get(s.id);
        if (t) {
          t.aborted = true;
          t.wake();
        }
        return json(true);
      }
    }
  }
  if ((m = p.match(/^\/(permission|question)\/([^/]+)\/reply$/)) && req.method === 'POST') {
    const a = asked.get(m[2]);
    if (!a) return notFound();
    if (m[1] === 'permission' && body?.reply === 'always') {
      const s = a.session;
      s.approved = [...(s.approved || []), ...(a.always || [])];
    }
    a.resolve(body || {});
    return json(true);
  }
  notFound();
});

server.listen(0, '127.0.0.1', () => {
  const { port } = server.address();
  if (!PASSWORD) process.stdout.write('Warning: OPENCODE_SERVER_PASSWORD is not set; server is unsecured.\n');
  process.stdout.write(`opencode server listening on http://127.0.0.1:${port}\n`);
});
