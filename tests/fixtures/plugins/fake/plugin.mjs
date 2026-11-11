// Test double of a plugin (PLG-2, ESC-174), a module of `run.js` since the process transport left (ESC-355): JSON-RPC 2.0
// on the lines Escouade hands it (`__esc.listen`, `__esc.write`), batches (a JSON array on one line) answered by a batch.
// What a test asks of it is in `args.json`, beside it (a list of flags and values, as a command line was).
//
// `initialize` answers protocol version 1, the one of `--protocol <n>`, or the one it is offered with
// `--protocol echo`; `--ignore-initialize` never answers it, `--ignore-shutdown` never answers
// `shutdown` (else it answers, and nothing more is handed to it).
// `echo` gives its params back, `frames` counts the frames it was given, `attach` keeps its `key`,
// `test/state` says what it holds; a decision (`permissions/decide`) is answered `ask`, or as
// `--decide allow|deny|ask|silent|crash` says (a refusal with `--decide-message <m>`; `silent` never answers, as
// `--silent-decisions` does, so that its default applies; `crash` fails the instance), each request kept for
// `test/state` (PLG-8). A fact (`events/fact`, PLG-3) asks for the secret `token` (or the key of `--fact-secret <key>`),
// writes it on its journal with `--leak`, and says the fact in a `log`; with `--keep-facts` it only keeps it, for
// `test/state` (PLG-5), busy `--busy <ms>` on each (what comes for it waits meanwhile). `--slow <ms>` holds every answer but `initialize`'s that long. `test/do` behaves as its `behave`
// says:
//   answer  → { ok: true, said: <say> }          silent → no answer, ever
//   crash   → a line on its journal, then the instance fails
//   loop    → a busy loop: the turn passes its time, and the instance is stopped
//   grow    → <mb> MB taken and held, then { grown } (past its memory, the instance fails)
//   stderr  → <lines> written on its journal, then { ok }
//   secret / steal → asks Escouade `secrets/get` for <key>, and gives back what came: { result } or { error }
//   settings → asks Escouade `settings/get` (of <project>, if given), and gives back what came: { result }
//   log     → a `log` notification of <message>, then { ok }
// It also serves a ticket source and a code host (INT-5, ESC-211): `account/me`, `tickets/*` and `forge/*`, on a
// few fixed records. `--untyped` answers every one with an error that is not typed; `--token-from <key>` makes every one
// ask Escouade `secrets/get` for that key first and answer 401 (-32401) unless it is `good`; a container named `gone`
// answers 404, `refused` a lasting refusal, `crash` fails the instance, `slow` is never answered. `forge/*` answer
// { value, rateLimit }; `forge/merge` is not served (-32601). `--ticket-budget` makes `tickets/*` answer
// { value, rateLimit } too, 4000 calls left (INT-6: the account's budget).
const esc = globalThis.__esc;
let args = [];
try {
  args = JSON.parse(esc.node('node:fs').readFileSync('/plugin/args.json'));
} catch {
  args = [];
}
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
let greeting = null;
let ended = false;
// What tells this instance from the one before it, restarted.
const instance = Math.random().toString(36).slice(2);

function send(msg) {
  esc.write(JSON.stringify(msg));
}

function trace(line) {
  esc.trace(line);
}

/** Fails the instance, as a process would exit: an exception QuickJS does not catch, out of the turn. */
function crash(why) {
  trace(`fake-plugin: ${why}`);
  throw new Error(`fake-plugin: ${why}`);
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

// The answer to `msg` (null: none, or none yet), or a promise of it. What fails the instance throws here, in the turn.
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
      greeting = params;
      return ok(id, {
        protocolVersion: option('--protocol') === 'echo' ? params.protocolVersion : Number(option('--protocol') ?? 1),
        capabilities: { batch: true },
        info: { name: 'fake-plugin' },
      });
    case 'shutdown':
      if (flag('--ignore-shutdown')) return null;
      send(ok(id, null));
      ended = true;
      esc.end();
      return null;
    case 'echo':
      return ok(id, params);
    case 'frames':
      return ok(id, { count: (params.frames ?? []).length });
    case 'attach':
      attached.push(params.key);
      return ok(id, { attached: attached.length });
    case 'test/state':
      return ok(id, { attached, facts, decisions, greeting, instance });
    case 'permissions/decide': {
      decisions.push(params);
      const mode = option('--decide') ?? (flag('--silent-decisions') ? 'silent' : 'ask');
      if (mode === 'silent') return null;
      if (mode === 'crash') crash('crashing on a decision');
      const message = option('--decide-message');
      return ok(id, mode === 'deny' && message ? { decision: 'deny', message } : { decision: mode });
    }
    case 'events/fact':
      if (flag('--keep-facts')) {
        facts.push(params);
        // Busy that long on each fact: what comes for it waits, as for a plugin that no longer reads.
        const busy = Number(option('--busy') ?? 0);
        for (const until = Date.now() + busy; Date.now() < until;) {
          // Busy.
        }
        return null;
      }
      ask('secrets/get', { key: option('--fact-secret') ?? 'token' }).then((answer) => {
        if (flag('--leak')) trace(`fake-plugin: the secret is ${answer.result}`);
        send({ jsonrpc: '2.0', method: 'log', params: { level: 'info', message: `fact ${params.name}` } });
      });
      return null;
    case 'test/calls':
      return ok(id, calls);
    case 'test/do':
      return behave(id, params);
    default:
      if (method.startsWith('tickets/') && flag('--ticket-budget')) {
        const m = serve(id, method, params);
        return Promise.resolve(m).then((m) =>
          m?.result === undefined ? m : ok(id, { value: m.result, rateLimit: { ...RATE, remaining: 4000 } }),
        );
      }
      if (method === 'account/me' || method.startsWith('tickets/') || method.startsWith('forge/') || method.startsWith('workflow/'))
        return serve(id, method, params);
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

// The tickets and forge calls (INT-5): the records are fixed, the failures are asked for by name. A crash is told in the
// turn, before anything is awaited.
function serve(id, method, p) {
  calls.push({ method, params: p });
  if (flag('--untyped')) return fail(id, -32000, 'boom');
  const name = p.container ?? p.repo?.name ?? p.reader;
  if (name === 'crash') crash('crashing on a ticket call');
  const tokenKey = option('--token-from');
  if (tokenKey) {
    return ask('secrets/get', { key: tokenKey }).then((answer) =>
      answer.result !== 'good' ? fail(id, -32401, 'le jeton est refusé') : records(id, method, p, name),
    );
  }
  return records(id, method, p, name);
}

function records(id, method, p, name) {
  if (name === 'gone') return fail(id, -32404, 'introuvable');
  if (name === 'refused') return fail(id, -32010, 'aucune transition ne mène là');
  if (name === 'slow') return null;
  const forge = (value) => ok(id, { value, rateLimit: RATE });
  switch (method) {
    case 'account/me':
      return ok(id, p.point === 'forge' ? { user: 'ada', label: '@ada', scopes: ['repo'] } : { label: 'ada@fake.test', user: 'ada' });
    // The reader of workflows (ESC-200): a graph of two steps named after the files it is given, or
    // one that does not hold (`broken`), or an answer too big (`many`).
    case 'workflow/read': {
      const files = (p.files ?? []).map((x) => x.path);
      if (p.reader === 'broken') {
        return ok(id, { workflows: [{ id: 'bad', name: 'Bad', steps: [{ name: 'a' }], transitions: [{ from: 'a', to: 'nowhere' }] }] });
      }
      if (p.reader === 'many') {
        return ok(id, { workflows: Array.from({ length: 51 }, (_, i) => ({ id: `w${i}`, name: 'W', steps: [{ name: 'a' }] })) });
      }
      return ok(id, {
        workflows: [
          {
            id: 'bmad',
            name: 'BMAD',
            repo: p.repos?.[0] ?? '',
            steps: [
              { name: 'brief', command: '/bmad:brief', description: files.join(','), writes: ['docs/brief.md'] },
              { name: 'build', command: '/bmad:build', agents: ['dev'] },
              { name: 'review', command: '/bmad:review' },
            ],
            transitions: [
              { from: 'brief', to: 'build' },
              { from: 'build', to: 'review' },
              { from: 'review', to: 'build', back: true, condition: 'when it finds gaps', max: 3 },
            ],
            exit: 'the review is clean',
          },
        ],
      });
    }
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
      crash('crashing on purpose');
      return null;
    case 'loop':
      for (;;) {
        // Busy: the turn passes its time.
      }
    case 'grow': {
      const mb = p.mb ?? 300;
      for (let i = 0; i < mb; i++) held.push(new Uint8Array(1024 * 1024).fill(i & 0xff));
      return ok(id, { grown: mb });
    }
    case 'stderr':
      for (const line of p.lines ?? []) trace(line);
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

function answered(msg, answer) {
  if (!answer) return;
  const slow = Number(option('--slow') ?? 0);
  if (slow > 0 && msg.method !== 'initialize') setTimeout(() => send(answer), slow);
  else send(answer);
}

esc.listen((line) => {
  if (ended || !line.trim()) return;
  let msg;
  try {
    msg = JSON.parse(line);
  } catch {
    send(fail(null, -32700, 'parse error'));
    return;
  }
  if (Array.isArray(msg)) {
    const answers = msg.map(handle);
    Promise.all(answers).then((all) => {
      const given = all.filter(Boolean);
      if (given.length) esc.write(JSON.stringify(given));
    });
    return;
  }
  const answer = handle(msg);
  if (answer && typeof answer.then === 'function') answer.then((a) => answered(msg, a));
  else answered(msg, answer);
});
