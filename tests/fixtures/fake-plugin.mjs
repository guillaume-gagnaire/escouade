#!/usr/bin/env node
// Test double of a plugin's program (PLG-2, ESC-174): JSON-RPC 2.0 on stdio, one message per line,
// batches (a JSON array on one line) answered by a batch.
//
// `initialize` answers protocol version 1, or the one of `--protocol <n>`; `--ignore-initialize`
// never answers it, `--ignore-shutdown` never answers `shutdown` (else it answers, then exits).
// `echo` gives its params back, `frames` counts the frames it was given, `attach` keeps its `key`,
// `test/state` says what it holds; a decision (`permissions/…`) is never answered, so that its default
// applies. `test/do` behaves as its `behave` says:
//   answer  → { ok: true, said: <say> }          silent → no answer, ever
//   crash   → a line on stderr, then exit 3       loop   → a busy loop: nothing answered any more
//   grow    → <mb> MB taken and held, then { grown } stderr → <lines> written on stderr, then { ok }
//   secret / steal → asks Escouade `secrets/get` for <key>, and gives back what came: { result } or { error }
//   log     → a `log` notification of <message>, then { ok }
import process from 'node:process';

const args = process.argv.slice(2);
const flag = (name) => args.includes(name);
const option = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};

const attached = [];
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
      return ok(id, {
        protocolVersion: Number(option('--protocol') ?? 1),
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
      return ok(id, { attached, pid: process.pid });
    case 'permissions/decide':
      return null;
    case 'test/do':
      return behave(id, params);
    default:
      return id === undefined ? null : fail(id, -32601, `unknown method ${method}`);
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
  if (answer) send(answer);
}
