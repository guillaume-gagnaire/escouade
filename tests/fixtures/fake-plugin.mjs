#!/usr/bin/env node
// Test double of a plugin's program (PLG-2, ESC-174): JSON-RPC 2.0 on stdio, one message per line,
// batches (a JSON array on one line) answered by a batch.
//
// `initialize` answers protocol version 1, the one of `--protocol <n>`, or the one it is offered with
// `--protocol echo`; `--ignore-initialize` never answers it, `--ignore-shutdown` never answers
// `shutdown` (else it answers, then exits).
// `echo` gives its params back, `frames` counts the frames it was given, `attach` keeps its `key`,
// `test/state` says what it holds; a decision (`permissions/decide`) is answered `ask`, or as
// `--decide allow|deny|ask|silent|crash` says (a refusal with `--decide-message <m>`; `silent` never answers, as
// `--silent-decisions` does, so that its default applies; `crash` exits 3), each request kept for `test/state` (PLG-8). A fact (`events/fact`, PLG-3) asks for the secret
// `token` (or the key of `--fact-secret <key>`), writes it on stderr with `--leak`, and says the fact
// in a `log`; with `--keep-facts` it only keeps it, for `test/state` (PLG-5). `--deaf` stops reading its
// input once initialized (its pipe fills: Escouade's queue of facts for it fills then). `--slow <ms>` holds every answer but `initialize`'s that long. `test/do` behaves as its
// `behave` says:
//   answer  → { ok: true, said: <say> }          silent → no answer, ever
//   crash   → a line on stderr, then exit 3       loop   → a busy loop: nothing answered any more
//   grow    → <mb> MB taken and held, then { grown } stderr → <lines> written on stderr, then { ok }
//   secret / steal → asks Escouade `secrets/get` for <key>, and gives back what came: { result } or { error }
//   settings → asks Escouade `settings/get` (of <project>, if given), and gives back what came: { result }
//   log     → a `log` notification of <message>, then { ok }
// It also serves a ticket source and a code host (INT-5, ESC-211): `account/me`, `tickets/*` and `forge/*`, on a
// few fixed records. `--untyped` answers every one with an error that is not typed; `--token-from <key>` makes every one ask Escouade `secrets/get` for that key first and
// answer 401 (-32401) unless it is `good`; a container named `gone` answers 404, `refused` a lasting refusal,
// `crash` exits, `slow` is never answered. `forge/*` answer { value, rateLimit }; `forge/merge` is not served (-32601).
import process from 'node:process';

const args = process.argv.slice(2);
const flag = (name) => args.includes(name);
const option = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};

const attached = [];
const facts = [];
const decisions = [];
const held = [];
const asked = new Map();
let nextAsk = 1;

function send(msg) {
  process.stdout.write(JSON.stringify(msg) + '\n');
}

function ask(method, params) {
  const id = `p${nextAsk++}`;
  return new Promise((resolve) => {
    asked.set(id, resolve);
    send({ jsonrpc: '2.0', id, method, params });
  });
}

const ok = (id, result) => ({ jsonrpc: '2.0', id, result });
const fail = (id, code, message) => ({ jsonrpc: '2.0', id, error: { code, message } });

// The answer to `msg` (null: none, or none yet), or a promise of it.
function handle(msg) {
  if (msg.method === undefined) {
    const resolve = asked.get(msg.id);
    if (resolve) {
      asked.delete(msg.id);
      resolve(msg);
    }
    return null;
  }
  const { id, method } = msg;
  const params = msg.params ?? {};
  switch (method) {
    case 'initialize':
      if (flag('--ignore-initialize')) return null;
      // Its paused input no longer keeps node alive: a timer does, as a real plugin's work would.
      if (flag('--deaf')) setTimeout(() => (process.stdin.pause(), setInterval(() => {}, 1 << 30)), 0);
      return ok(id, {
        protocolVersion: option('--protocol') === 'echo' ? params.protocolVersion : Number(option('--protocol') ?? 1),
        capabilities: { batch: true },
        info: { name: 'fake-plugin', pid: process.pid },
      });
    case 'shutdown':
      if (flag('--ignore-shutdown')) return null;
      send(ok(id, null));
      setTimeout(() => process.exit(0), 10);
      return null;
    case 'echo':
      return ok(id, params);
    case 'frames':
      return ok(id, { count: (params.frames ?? []).length });
    case 'attach':
      attached.push(params.key);
      return ok(id, { attached: attached.length });
    case 'test/state':
      return ok(id, { attached, facts, decisions, pid: process.pid });
    case 'permissions/decide': {
      decisions.push(params);
      const mode = option('--decide') ?? (flag('--silent-decisions') ? 'silent' : 'ask');
      if (mode === 'silent') return null;
      if (mode === 'crash') {
        process.stderr.write('fake-plugin: crashing on a decision\n');
        process.exit(3);
      }
      const message = option('--decide-message');
      return ok(id, mode === 'deny' && message ? { decision: 'deny', message } : { decision: mode });
    }
    case 'events/fact':
      if (flag('--keep-facts')) {
        facts.push(params);
        return null;
      }
      ask('secrets/get', { key: option('--fact-secret') ?? 'token' }).then((answer) => {
        if (flag('--leak')) process.stderr.write(`fake-plugin: the secret is ${answer.result}\n`);
        send({ jsonrpc: '2.0', method: 'log', params: { level: 'info', message: `fact ${params.name}` } });
      });
      return null;
    case 'test/calls':
      return ok(id, calls);
    case 'test/do':
      return behave(id, params);
    default:
      if (method === 'account/me' || method.startsWith('tickets/') || method.startsWith('forge/')) return serve(id, method, params);
      return id === undefined ? null : fail(id, -32601, `unknown method ${method}`);
  }
}

const ISSUES = [
  { id: '1', key: 'FAKE-1', title: 'Premier', container: 'demo', url: 'https://fake.test/1' },
  { id: '2', key: 'FAKE-2', title: 'Second', container: 'demo', url: 'https://fake.test/2' },
];
const PRS = [{ number: 7, url: 'https://fake.test/pr/7', title: 'Une PR', state: 'open', head: 'feat/x', base: 'main', checks: 'success' }];
const RATE = { limit: 5000, remaining: 4990, reset: 1900000000 };
const calls = [];

// The tickets and forge calls (INT-5): the records are fixed, the failures are asked for by name.
async function serve(id, method, p) {
  calls.push({ method, params: p });
  if (flag('--untyped')) return fail(id, -32000, 'boom');
  const tokenKey = option('--token-from');
  if (tokenKey) {
    const answer = await ask('secrets/get', { key: tokenKey });
    if (answer.result !== 'good') return fail(id, -32401, 'le jeton est refusé');
  }
  const name = p.container ?? p.repo?.name;
  if (name === 'gone') return fail(id, -32404, 'introuvable');
  if (name === 'refused') return fail(id, -32010, 'aucune transition ne mène là');
  if (name === 'crash') {
    process.stderr.write('fake-plugin: crashing on a ticket call\n');
    process.exit(3);
  }
  if (name === 'slow') return null;
  const forge = (value) => ok(id, { value, rateLimit: RATE });
  switch (method) {
    case 'account/me':
      return ok(id, p.point === 'forge' ? { user: 'ada', label: '@ada', scopes: ['repo'] } : { label: 'ada@fake.test', user: 'ada' });
    case 'tickets/containers':
      return ok(id, [
        { id: 'demo', name: 'Démo', own: true },
        { id: 'other', name: 'Autre', group: 'Équipe' },
      ]);
    case 'tickets/states':
      return ok(id, [
        { id: 'open', name: 'Ouvert' },
        { id: 'done', name: 'Fait' },
      ]);
    case 'tickets/issues': {
      const q = p.query ?? {};
      const issues = ISSUES.filter((i) => !q.text || i.title.toLowerCase().includes(q.text.toLowerCase()));
      return ok(id, { issues, filters: [{ id: 'mine', label: 'Les miens' }], next: null, total: issues.length });
    }
    case 'tickets/setState':
    case 'tickets/comment':
      return ok(id, null);
    case 'forge/pullRequest':
      return forge(PRS.find((x) => x.number === p.number) ?? PRS[0]);
    case 'forge/pullRequestsOfBranch':
      return forge(PRS.filter((x) => x.head === p.branch));
    case 'forge/checks':
      return forge([{ name: 'ci', state: 'success', url: null, job: '9', started: null, finished: null, cancelled: false }]);
    case 'forge/createPr':
      return forge({ ...PRS[0], number: 8, title: p.pr.title, head: p.pr.head, base: p.pr.base });
    case 'forge/comment':
    case 'forge/parent':
      return forge(null);
    case 'forge/merge':
      return fail(id, -32601, 'unknown method forge/merge');
    default:
      return fail(id, -32601, `unknown method ${method}`);
  }
}

function behave(id, p) {
  switch (p.behave) {
    case 'answer':
      return ok(id, { ok: true, said: p.say ?? null });
    case 'silent':
      return null;
    case 'crash':
      process.stderr.write('fake-plugin: crashing on purpose\n');
      process.exit(3);
      return null;
    case 'loop':
      for (;;) {
        // Busy: neither this nor anything after it is answered.
      }
    case 'grow': {
      const mb = p.mb ?? 300;
      for (let i = 0; i < mb; i++) held.push(Buffer.alloc(1024 * 1024, i & 0xff));
      return ok(id, { grown: mb });
    }
    case 'stderr':
      for (const line of p.lines ?? []) process.stderr.write(line + '\n');
      return ok(id, { ok: true });
    case 'secret':
    case 'steal':
      return ask('secrets/get', { key: p.key }).then((answer) =>
        ok(id, answer.error ? { error: answer.error } : { result: answer.result }),
      );
    case 'settings':
      return ask('settings/get', p.project ? { project: p.project } : {}).then((answer) => ok(id, { result: answer.result ?? null }));
    case 'log':
      send({ jsonrpc: '2.0', method: 'log', params: { level: 'info', message: p.message } });
      return ok(id, { ok: true });
    default:
      return fail(id, -32602, `unknown behaviour ${p.behave}`);
  }
}

let buffer = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', (chunk) => {
  buffer += chunk;
  let nl;
  while ((nl = buffer.indexOf('\n')) >= 0) {
    const line = buffer.slice(0, nl).trim();
    buffer = buffer.slice(nl + 1);
    if (line) take(line);
  }
});
process.stdin.on('end', () => process.exit(0));

async function take(line) {
  let msg;
  try {
    msg = JSON.parse(line);
  } catch {
    send(fail(null, -32700, 'parse error'));
    return;
  }
  if (Array.isArray(msg)) {
    const answers = (await Promise.all(msg.map(handle))).filter(Boolean);
    if (answers.length) process.stdout.write(JSON.stringify(answers) + '\n');
    return;
  }
  const answer = await handle(msg);
  if (!answer) return;
  const slow = Number(option('--slow') ?? 0);
  if (slow > 0 && msg.method !== 'initialize') setTimeout(() => send(answer), slow);
  else send(answer);
}
