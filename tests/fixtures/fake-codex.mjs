#!/usr/bin/env node
// A fake `codex app-server` (Codex 0.163.0's JSON-RPC over stdio, one message a line), as the real
// streams of tests/fixtures/codex/ show it: what the conformance bench (src-tauri/src/engine/codex/bench.rs)
// and the end-to-end tests drive instead of Codex.
//
// `codex --version` says its version; `codex exec` and `codex login` play theirs (below); `codex app-server …`
// serves; anything else is its interface (« Vue Codex »,
// tests/fixtures/fake-codex-tui.mjs). The app-server lists the hooks of its `-c hooks.<Event>=…` options
// (`hooks/list`: their keys and hashes, trusted when a `-c hooks.state=…` option says so), and writes the
// rollout of a thread it starts or forks (its `session_meta`), where its interface finds it to resume it.
// It answers `initialize`, `thread/start`
// (a new thread `thr-<pid>-<n>`), `thread/resume` (any thread but one whose id starts with "perdu": refused),
// `thread/fork` (a new thread), `model/list`, `account/rateLimits/read`, `turn/start`, `turn/steer`,
// `turn/interrupt`, `thread/name/set`, `thread/compact/start` (a turn of its own, as `compact` says). What a
// turn plays is picked by its message (French words, as the scenarios of the fake `claude`):
// - "raisonnement": a reasoning (its summary by deltas), then a reply;
// - "commande": a command (`rm -rf build`) that asks its approval (item/commandExecution/requestApproval):
//   run once accepted, declined otherwise;
// - "git": `git add -A` asked the same way (the sandbox keeps `.git` out), in the agent's folder;
// - "fichier": a change of src/app.ts (written for real when it is there: `const a = 2;`), with its diff;
// - "question": a question (item/tool/requestUserInput), its answer said back;
// - "propose un plan", or the turn in the mode Plan: a plan (item `plan`), the turn over on it;
//   "Mets le plan en œuvre" / "Implement the plan": its work, a plain reply;
// - "étapes": the plan tool's three steps (turn/plan/updated), done one after the other;
// - "sous-agent": a subagent (subAgentActivity, its own thread and turn, its report), waited for;
// - "lent": a turn that lasts 30 s, unless interrupted (turn/interrupt) or steered (turn/steer: its message
//   said back, and the turn ends);
// - "limite": a turn stopped by the usage limit (usageLimitExceeded), the quota at 100 %;
// - a message said in a voice conversation (<escouade-voice>): a reply with its passage to be read aloud;
// - anything else: « Bonjour, tu as dit : <message> ».
// A thread started with `developerInstructions` (a ticket's protocol) plays the ticket's agent, as the fake
// `claude` does: <key>.txt written ("Boucle n"), and each turn ends with an ```escouade report; [ok] in the
// ticket's title meets every criterion at once, otherwise criterion n is met from loop n on.
// FAKE_CODEX_LOG: a file each launch appends to: {"launch": {argv, cwd, pid, codexHome, token}} (token: whether
// ESCOUADE_MCP_TOKEN was given), then {"in": <message>} for each message it is sent.

import fs from 'node:fs';
import path from 'node:path';
import readline from 'node:readline';

// Asked to name a task (<tache>, <task>, ESC-156): its first two words and "-codex".
const nameOf = (asked) => {
  const task = asked.match(/<(tache|task)>\s*([\s\S]*?)\s*<\/\1>/)?.[2];
  return task === undefined ? null : `${(task.toLowerCase().match(/[a-z]+/g) ?? ['tache']).slice(0, 2).join('-')}-codex`;
};

const VERSION = '0.163.0';
const argv = process.argv.slice(2);
if (argv.includes('--version')) {
  process.stdout.write(`codex-cli ${VERSION}\n`);
  process.exit(0);
}
const LOG_FILE = process.env.FAKE_CODEX_LOG;
const logLine = (entry) => LOG_FILE && fs.appendFileSync(LOG_FILE, JSON.stringify(entry) + '\n');
if (argv[0] === 'exec') {
  // `codex exec … -` (the texts written by a model): its question read from its input, its answer the last
  // message of its `--json` stream, « Réponse de Codex : <the question's last line> »; a question that says
  // "échoue" fails its turn as Codex 0.163.0 does.
  let question = '';
  process.stdin.setEncoding('utf8');
  process.stdin.on('data', (d) => (question += d));
  process.stdin.on('end', () => {
    logLine({ exec: { argv, cwd: process.cwd(), codexHome: process.env.CODEX_HOME ?? null, question } });
    // Also in its account's folder, for the tests of the core (which give the fake no variable of its own).
    if (process.env.CODEX_HOME) {
      fs.mkdirSync(process.env.CODEX_HOME, { recursive: true });
      fs.appendFileSync(path.join(process.env.CODEX_HOME, 'exec.jsonl'), JSON.stringify({ argv, question }) + '\n');
    }
    const say = (v) => process.stdout.write(JSON.stringify(v) + '\n');
    say({ type: 'thread.started', thread_id: `exec-${process.pid}` });
    say({ type: 'turn.started' });
    if (question.includes('échoue')) {
      const api = { type: 'error', status: 400, error: { type: 'invalid_request_error', message: 'Le modèle a refusé.' } };
      say({ type: 'turn.failed', error: { message: JSON.stringify(api) } });
      process.exit(1);
    }
    say({ type: 'item.completed', item: { id: 'item_0', type: 'agent_message', text: 'Je réfléchis.' } });
    const last = question.trim().split('\n').at(-1);
    say({ type: 'item.completed', item: { id: 'item_1', type: 'agent_message', text: nameOf(question) ?? `Réponse de Codex : ${last}` } });
    say({ type: 'turn.completed', usage: { input_tokens: 100, cached_input_tokens: 0, output_tokens: 5 } });
    process.exit(0);
  });
} else if (argv[0] === 'login') {
  // `codex login` (« Ajouter un compte »): signed in at once, `auth.json` written in CODEX_HOME as Codex writes
  // it (its id_token says the email), after a line of its own. FAKE_CODEX_LOGIN_WAIT: it waits for a line typed
  // first (to try the terminal).
  const home = process.env.CODEX_HOME || path.join(process.env.USERPROFILE || process.env.HOME || '.', '.codex');
  logLine({ login: { argv, codexHome: process.env.CODEX_HOME ?? null } });
  process.stdout.write('Starting local login server on http://localhost:1455.\r\n');
  const done = () => {
    const b64 = (v) => Buffer.from(JSON.stringify(v)).toString('base64url');
    const idToken = `${b64({ alg: 'none' })}.${b64({ email: process.env.FAKE_CODEX_EMAIL || 'codex@example.com' })}.sig`;
    fs.mkdirSync(home, { recursive: true });
    fs.writeFileSync(
      path.join(home, 'auth.json'),
      JSON.stringify({
        auth_mode: 'chatgpt',
        OPENAI_API_KEY: null,
        tokens: { id_token: idToken, access_token: 'acces', refresh_token: 'rafraichi', account_id: 'compte-1' },
        last_refresh: new Date().toISOString(),
      }),
    );
    process.stdout.write('Successfully logged in\r\n');
    process.exit(0);
  };
  if (process.env.FAKE_CODEX_LOGIN_RELAY) {
    // As Codex does (ESC-538, S13): a local server waits for the page's return on localhost, its address said in the
    // page's `redirect_uri`. On the e2e's « server » (this machine), the port said is free here and the fake `ssh` maps it
    // to the one it listens on (FAKE_SSH_PORTS_FILE), as two machines would have the same port each.
    const http = await import('node:http');
    const net = await import('node:net');
    const said = await new Promise((resolve) => {
      const probe = net.createServer().listen(0, '127.0.0.1', () => {
        const p = probe.address().port;
        probe.close(() => resolve(p));
      });
    });
    const server = http.createServer((req, res) => {
      if (!req.url?.startsWith('/auth/callback?code=')) {
        res.writeHead(404).end();
        return;
      }
      res.writeHead(200, { 'Content-Type': 'text/plain' }).end('Signed in to Codex. You may close this page.');
      server.close();
      done();
    });
    server.listen(0, '127.0.0.1', () => {
      const real = server.address().port;
      if (process.env.FAKE_SSH_PORTS_FILE) fs.appendFileSync(process.env.FAKE_SSH_PORTS_FILE, `${said}=${real}\n`);
      const back = encodeURIComponent(`http://localhost:${said}/auth/callback`);
      process.stdout.write(
        `If your browser did not open, navigate to this URL to authenticate:\r\n\r\nhttps://auth.openai.com/oauth/authorize?response_type=code&client_id=app_test&redirect_uri=${back}&scope=openid\r\n`,
      );
    });
  } else if (process.env.FAKE_CODEX_LOGIN_WAIT) process.stdin.once('data', done);
  else done();
}
// `exec` and `login` end the process themselves: neither the interface nor the app-server below is played.
if (argv[0] === 'exec' || argv[0] === 'login') await new Promise(() => {});

const LOG = process.env.FAKE_CODEX_LOG;
const log = (entry) => LOG && fs.appendFileSync(LOG, JSON.stringify(entry) + '\n');
const launch = {
  argv,
  cwd: process.cwd(),
  pid: process.pid,
  codexHome: process.env.CODEX_HOME ?? null,
  token: !!process.env.ESCOUADE_MCP_TOKEN,
  tui: argv[0] !== 'app-server',
};
// Its launch and what it is sent, in its account's folder too when it is there: the tests of an
// account read them in `requests.jsonl`.
function noteInHome(entry) {
  const home = process.env.CODEX_HOME;
  if (home && fs.existsSync(home)) fs.appendFileSync(path.join(home, 'requests.jsonl'), JSON.stringify(entry) + '\n');
}
log({ launch });
noteInHome({ launch });
const ui = await import('./fake-codex-tui.mjs');
if (argv[0] !== 'app-server') {
  ui.tui(argv, log);
  // The rest is the app-server's: never reached.
  await new Promise(() => {});
}
// Also in its account's folder (`<CODEX_HOME>/launches.jsonl`), for the tests of the core, which give the fake no
// variable of its own; and that folder's `quota.json` ({"rateLimits": …}), when there is one, is its quota.
const HOME = process.env.CODEX_HOME;
if (HOME) {
  fs.mkdirSync(HOME, { recursive: true });
  // With the token itself, which the tests let in the server.
  const own = { ...launch, escouadeToken: process.env.ESCOUADE_MCP_TOKEN ?? null };
  fs.appendFileSync(path.join(HOME, 'launches.jsonl'), JSON.stringify(own) + '\n');
}
function ownQuota() {
  try {
    return JSON.parse(fs.readFileSync(path.join(HOME, 'quota.json'), 'utf8')).rateLimits;
  } catch {
    return null;
  }
}

const out = (msg) => process.stdout.write(JSON.stringify(msg) + '\n');
const notify = (method, params) => out({ method, params });
let serial = 0;
const id = (prefix) => `${prefix}-${process.pid}-${++serial}`;

// The thread of this process, and what it was started with.
let thread = null;
let instructions = '';
let cwd = process.cwd();
// The turn under way: its id, its mode, what it waits for.
let turn = null;
let mode = 'default';
// Requests of ours waiting for their answer, by id.
let nextRequest = 0;
const waiting = new Map();
// Ticket's agent: its asked question once.
let loopsDone = 0;

function threadObject(tid, extra = {}) {
  return {
    id: tid,
    sessionId: tid,
    forkedFromId: null,
    parentThreadId: null,
    preview: '',
    ephemeral: false,
    modelProvider: 'openai',
    model: 'gpt-6-luna',
    cwd,
    cliVersion: VERSION,
    status: { type: 'idle' },
    turns: [],
    ...extra,
  };
}

const MODELS = [
  {
    id: 'gpt-6-luna',
    model: 'gpt-6-luna',
    displayName: 'GPT-6-Luna',
    description: 'Fast and affordable model for easier tasks.',
    hidden: false,
    supportedReasoningEfforts: ['low', 'medium', 'high', 'xhigh', 'max'].map((e) => ({ reasoningEffort: e, description: e })),
    defaultReasoningEffort: 'medium',
    inputModalities: ['text', 'image'],
    isDefault: true,
  },
  {
    id: 'gpt-5.6-terra',
    model: 'gpt-5.6-terra',
    displayName: 'GPT-5.6-Terra',
    description: 'Older balanced model for straightforward work.',
    hidden: false,
    supportedReasoningEfforts: ['low', 'medium', 'high'].map((e) => ({ reasoningEffort: e, description: e })),
    defaultReasoningEffort: 'medium',
    inputModalities: ['text'],
    isDefault: false,
  },
  {
    id: 'gpt-6-luna-review',
    model: 'gpt-6-luna-review',
    displayName: 'GPT-6-Luna Review',
    description: 'Hidden from the picker.',
    hidden: true,
    supportedReasoningEfforts: [{ reasoningEffort: 'medium', description: 'medium' }],
    defaultReasoningEffort: 'medium',
    inputModalities: ['text'],
    isDefault: false,
  },
];

const RATE = (used) => ({
  limitId: 'codex',
  primary: { usedPercent: used, windowDurationMins: 300, resetsAt: Math.floor(Date.now() / 1000) + 3600 },
  secondary: { usedPercent: Math.min(used, 40), windowDurationMins: 10080, resetsAt: Math.floor(Date.now() / 1000) + 86400 },
  credits: { hasCredits: false, unlimited: false, balance: null },
  planType: 'plus',
  rateLimitReachedType: used >= 100 ? 'rate_limit_reached' : null,
});

/** A request of ours to the client (an approval, a question): its answer. */
function ask(method, params) {
  const rid = nextRequest++;
  out({ method, id: rid, params });
  return new Promise((resolve) => waiting.set(rid, { resolve, method }));
}

/** Resolves when the turn is interrupted (or steered, with its input), else after `ms`. */
function pause(ms) {
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve({ timeout: true }), ms);
    turn.wake = (why) => {
      clearTimeout(timer);
      resolve(why);
    };
  });
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function at(params = {}) {
  return { threadId: thread, turnId: turn.id, ...params };
}

function item(started, it, tid = thread, turnId = turn.id) {
  notify(started ? 'item/started' : 'item/completed', { item: it, threadId: tid, turnId });
}

/** A message of the agent, by deltas, then whole. */
async function say(text, phase = 'final_answer', tid = thread, turnId = turn.id) {
  const mid = id('msg');
  item(true, { type: 'agentMessage', id: mid, text: '', phase }, tid, turnId);
  const words = text.match(/\S+\s*/g) ?? [text];
  for (const w of words) notify('item/agentMessage/delta', { threadId: tid, turnId, itemId: mid, delta: w });
  item(false, { type: 'agentMessage', id: mid, text, phase }, tid, turnId);
}

function tokens(tid, n) {
  const total = {
    totalTokens: 1200 * n,
    inputTokens: 1000 * n,
    cachedInputTokens: 400 * n,
    cacheWriteInputTokens: 0,
    outputTokens: 200 * n,
    reasoningOutputTokens: 0,
  };
  notify('thread/tokenUsage/updated', {
    threadId: tid,
    turnId: turn.id,
    tokenUsage: { total, last: { ...total, inputTokens: 1000 }, modelContextWindow: 258400 },
  });
}

let usage = 0;

function command(text, cmd) {
  return {
    type: 'commandExecution',
    id: id('exec'),
    command: `"C:\\Program Files\\PowerShell\\7\\pwsh.exe" -Command '${cmd}'`,
    cwd,
    processId: null,
    source: 'agent',
    status: 'inProgress',
    commandActions: [{ type: 'unknown', command: cmd }],
    aggregatedOutput: null,
    exitCode: null,
    durationMs: null,
  };
}

async function approved(cmd, reason) {
  const it = command('', cmd);
  item(true, it);
  notify('thread/status/changed', { threadId: thread, status: { type: 'active', activeFlags: ['waitingOnApproval'] } });
  const answer = await ask(
    'item/commandExecution/requestApproval',
    at({
      kind: 'command',
      itemId: it.id,
      startedAtMs: Date.now(),
      environmentId: 'local',
      reason,
      command: it.command,
      cwd,
      commandActions: it.commandActions,
      proposedExecpolicyAmendment: cmd.split(' '),
      availableDecisions: ['accept', 'acceptForSession', 'decline', 'cancel'],
    }),
  );
  notify('thread/status/changed', { threadId: thread, status: { type: 'active', activeFlags: [] } });
  const accepted = answer?.decision === 'accept' || answer?.decision === 'acceptForSession';
  item(false, {
    ...it,
    status: accepted ? 'completed' : 'declined',
    aggregatedOutput: accepted ? `${cmd} : fait\n` : null,
    exitCode: accepted ? 0 : null,
    durationMs: 12,
  });
  return accepted;
}

function ticketReport(text) {
  const key = instructions.match(/ticket ([A-Z]+-\d+)/)?.[1] ?? 'TIC-0';
  const count = Number(instructions.match(/(?:Critères d'acceptation|Acceptance criteria) \((\d+)\)/)?.[1] ?? 1);
  const loop = Number(text.match(/(?:Boucle|Loop) (\d+)\//)?.[1] ?? ++loopsDone);
  const all = `${instructions}\n${text}`;
  const file = `${key.toLowerCase()}.txt`;
  fs.writeFileSync(path.join(cwd, file), `Boucle ${loop}\n`);
  const met = (n) => all.includes('[ok]') || n <= loop;
  const report = {
    criteres: Array.from({ length: count }, (_, i) => ({
      n: i + 1,
      ok: met(i + 1),
      note: met(i + 1) ? 'vérifié par Codex' : `reste le critère ${i + 1}`,
    })),
    avancement: [`Fichier ${file} écrit par Codex`, `Boucle ${loop} faite`],
  };
  return { file, text: `Boucle ${loop} faite.\n\n\`\`\`escouade\n${JSON.stringify(report)}\n\`\`\`` };
}

async function play(text) {
  const lower = text.toLowerCase();
  if (/Critères d'acceptation|Acceptance criteria/.test(instructions)) {
    const { file, text: reply } = ticketReport(text);
    const change = { path: path.join(cwd, file), kind: { type: 'add' }, diff: 'Boucle\n' };
    const pid = id('patch');
    item(true, { type: 'fileChange', id: pid, changes: [change], status: 'inProgress' });
    item(false, { type: 'fileChange', id: pid, changes: [change], status: 'completed' });
    await say(reply);
    return 'completed';
  }
  if (text.includes('<escouade-voice>')) {
    const said = text.split('<escouade-voice>')[0].trim();
    await say(
      `J'ai bien noté : « ${said} ».\n\n- détail technique : \`src/app.rs\`, ligne 42\n\n<spoken>C'est noté, je m'en occupe avec Codex.</spoken>`,
    );
    return 'completed';
  }
  if (lower.includes('en œuvre') || lower.includes('implement the plan')) {
    await say('Plan mis en œuvre.');
    return 'completed';
  }
  if (lower.includes('limite')) {
    usage = 100;
    notify('account/rateLimits/updated', { rateLimits: RATE(100) });
    const error = { message: "You've hit your usage limit.", codexErrorInfo: 'usageLimitExceeded', additionalDetails: null };
    notify('error', at({ error, willRetry: false }));
    return { status: 'failed', error };
  }
  if (lower.includes('raisonnement')) {
    const rid = id('rs');
    item(true, { type: 'reasoning', id: rid, summary: [], content: [] });
    notify('item/reasoning/summaryPartAdded', at({ itemId: rid, summaryIndex: 0 }));
    for (const d of ['**Deux pistes**', ' ; la seconde est la bonne.'])
      notify('item/reasoning/summaryTextDelta', at({ itemId: rid, delta: d, summaryIndex: 0 }));
    item(false, { type: 'reasoning', id: rid, summary: ['**Deux pistes** ; la seconde est la bonne.'], content: [] });
    await say('Réponse réfléchie.');
    return 'completed';
  }
  if (lower.includes('commande')) {
    await say('Je supprime le dossier build.', 'commentary');
    const ok = await approved('rm -rf build', 'Autorisez-vous la suppression du dossier build ?');
    await say(ok ? 'Commande exécutée.' : 'Compris, je ne la lance pas.');
    return 'completed';
  }
  if (/\bgit\b/.test(lower)) {
    const ok = await approved('git add -A', "Autorisez-vous l'écriture dans l'index Git partagé ?");
    await say(ok ? 'Fichiers ajoutés à git.' : 'git non lancé.');
    return 'completed';
  }
  if (lower.includes('fichier')) {
    const file = path.join(cwd, 'src', 'app.ts');
    if (fs.existsSync(file)) fs.writeFileSync(file, 'const a = 2;\n');
    const change = { path: file, kind: { type: 'update', move_path: null }, diff: '@@ -1 +1 @@\n-const a = 1;\n+const a = 2;\n' };
    const pid = id('patch');
    item(true, { type: 'fileChange', id: pid, changes: [change], status: 'inProgress' });
    item(false, { type: 'fileChange', id: pid, changes: [change], status: 'completed' });
    notify('turn/diff/updated', at({ diff: `diff --git a/src/app.ts b/src/app.ts\n${change.diff}` }));
    await say('Fichier modifié.');
    return 'completed';
  }
  if (lower.includes('question')) {
    const call = id('call');
    const answer = await ask(
      'item/tool/requestUserInput',
      at({
        itemId: call,
        questions: [
          {
            id: 'base',
            header: 'Base',
            question: 'Quelle base de données ?',
            isOther: true,
            isSecret: false,
            options: [
              { label: 'PostgreSQL', description: 'La plus complète.' },
              { label: 'SQLite', description: 'Un fichier, sans serveur.' },
            ],
          },
        ],
        isBlocking: true,
        autoResolutionMs: null,
      }),
    );
    await say(`Choix retenu : ${answer?.answers?.base?.answers?.join(', ') ?? '(aucun)'}`);
    return 'completed';
  }
  if (mode === 'plan' || lower.includes('propose un plan')) {
    const pid = `${turn.id}-plan`;
    const plan = '## Script de la date\n\n### Modifications\n- Ajouter `date.py`.\n\n### Vérification\n- Lancer le script.\n';
    item(true, { type: 'plan', id: pid, text: '' });
    for (const d of plan.match(/.{1,12}/gs)) notify('item/plan/delta', at({ itemId: pid, delta: d }));
    item(false, { type: 'plan', id: pid, text: plan });
    return 'completed';
  }
  if (lower.includes('étapes')) {
    const steps = ['Écrire le fichier', 'Le commiter', 'Vérifier'];
    for (let i = 0; i <= steps.length; i++) {
      notify(
        'turn/plan/updated',
        at({
          explanation: null,
          plan: steps.map((step, j) => ({ step, status: j < i ? 'completed' : j === i ? 'inProgress' : 'pending' })),
        }),
      );
      await sleep(20);
    }
    await say('Les trois étapes sont faites.');
    return 'completed';
  }
  if (lower.includes('sous-agent')) {
    const child = id('thr-child');
    const call = id('call');
    const started = {
      type: 'subAgentActivity',
      model: 'gpt-6-luna',
      reasoningEffort: 'medium',
      id: call,
      kind: 'started',
      agentThreadId: child,
      agentPath: '/root/cause_du_bug',
    };
    item(true, started);
    item(false, started);
    const childTurn = id('turn');
    notify('turn/started', { threadId: child, turn: { id: childTurn, rootTurnId: turn.id, items: [], status: 'inProgress' } });
    const wait = {
      type: 'collabAgentToolCall',
      id: id('call'),
      tool: 'wait',
      status: 'inProgress',
      senderThreadId: thread,
      receiverThreadIds: [],
      agentsStates: {},
    };
    item(true, wait);
    const cmd = command('', 'Get-Content src/filtre.ts');
    item(true, cmd, child, childTurn);
    item(false, { ...cmd, status: 'completed', aggregatedOutput: 'refuse(pdf)\n', exitCode: 0 }, child, childTurn);
    tokens(child, 1);
    await say('Cause trouvée : le filtre refuse les PDF.', 'final_answer', child, childTurn);
    const done = {
      type: 'subAgentActivity',
      model: null,
      reasoningEffort: null,
      id: `subagent-completed-${childTurn}`,
      kind: 'completed',
      agentThreadId: child,
      agentPath: '/root/cause_du_bug',
    };
    item(true, done);
    notify('turn/completed', {
      threadId: child,
      turn: { id: childTurn, rootTurnId: turn.id, items: [], status: 'completed', error: null, durationMs: 40 },
    });
    item(false, done);
    item(false, { ...wait, status: 'completed' });
    await say('Rapport reçu : le filtre refuse les PDF.');
    return 'completed';
  }
  if (lower.includes('lent')) {
    await say('Je commence…', 'commentary');
    // « lent 600s » lasts that long (the website's screenshots), a plain « lent » 30 s.
    const why = await pause(Number(/lent (\d+)s/.exec(lower)?.[1] ?? 30) * 1000);
    if (why?.steer) {
      await say(`J'ai bien reçu en cours de route : ${why.steer}`);
      return 'completed';
    }
    return why?.interrupt ? 'interrupted' : 'completed';
  }
  await say(`Bonjour, tu as dit : ${text}`);
  return 'completed';
}

async function startTurn(params) {
  const text = (params.input ?? [])
    .filter((i) => i.type === 'text')
    .map((i) => i.text)
    .join('\n');
  mode = params.collaborationMode?.mode ?? 'default';
  const tid = id('turn');
  turn = { id: tid, started: Date.now(), wake: null };
  notify('thread/settings/updated', {
    threadId: thread,
    threadSettings: {
      cwd,
      approvalPolicy: params.approvalPolicy ?? 'on-request',
      approvalsReviewer: params.approvalsReviewer ?? 'user',
      sandboxPolicy: params.sandboxPolicy ?? { type: 'workspaceWrite' },
      model: params.model ?? 'gpt-6-luna',
      effort: params.effort ?? null,
      collaborationMode: params.collaborationMode ?? { mode: 'default', settings: { model: params.model } },
    },
  });
  notify('thread/status/changed', { threadId: thread, status: { type: 'active', activeFlags: [] } });
  notify('turn/started', { threadId: thread, turn: { id: tid, rootTurnId: tid, items: [], status: 'inProgress', error: null } });
  item(true, { type: 'userMessage', id: id('user'), content: params.input ?? [] });
  setImmediate(async () => {
    const end = await play(text);
    tokens(thread, ++serial);
    notify('account/rateLimits/updated', { rateLimits: RATE(usage || 12) });
    const status = typeof end === 'string' ? end : end.status;
    const error = typeof end === 'string' ? null : end.error;
    notify('thread/status/changed', { threadId: thread, status: { type: 'idle' } });
    notify('turn/completed', {
      threadId: thread,
      turn: { id: tid, rootTurnId: tid, items: [], status, error, durationMs: Date.now() - turn.started },
    });
    turn = null;
  });
  return { turn: { id: tid, rootTurnId: tid, items: [], status: 'inProgress', error: null } };
}

/** `thread/compact/start`, as full-turn.jsonl recorded it: a turn of its own with a `contextCompaction` item, the
 * history's tokens after it in `last.totalTokens` (its `inputTokens` 0). */
function compact() {
  const tid = id('turn');
  turn = { id: tid, started: Date.now(), wake: null };
  notify('thread/status/changed', { threadId: thread, status: { type: 'active', activeFlags: [] } });
  notify('turn/started', { threadId: thread, turn: { id: tid, rootTurnId: tid, items: [], status: 'inProgress', error: null } });
  const cid = id('compact');
  item(true, { type: 'contextCompaction', id: cid });
  setTimeout(() => {
    const n = ++serial;
    const total = {
      totalTokens: 1200 * n,
      inputTokens: 1000 * n,
      cachedInputTokens: 400 * n,
      cacheWriteInputTokens: 0,
      outputTokens: 200 * n,
      reasoningOutputTokens: 0,
    };
    const last = {
      totalTokens: 300,
      inputTokens: 0,
      cachedInputTokens: 0,
      cacheWriteInputTokens: 0,
      outputTokens: 0,
      reasoningOutputTokens: 0,
    };
    notify('thread/tokenUsage/updated', { threadId: thread, turnId: tid, tokenUsage: { total, last, modelContextWindow: 258400 } });
    item(false, { type: 'contextCompaction', id: cid });
    notify('thread/status/changed', { threadId: thread, status: { type: 'idle' } });
    notify('turn/completed', {
      threadId: thread,
      turn: { id: tid, rootTurnId: tid, items: [], status: 'completed', error: null, durationMs: Date.now() - turn.started },
    });
    turn = null;
  }, 50);
}

function handle(msg) {
  log({ in: msg });
  noteInHome({ in: msg });
  // An answer to one of our requests.
  if (msg.method === undefined) {
    const w = waiting.get(msg.id);
    if (w) {
      waiting.delete(msg.id);
      notify('serverRequest/resolved', { threadId: thread, requestId: msg.id });
      w.resolve(msg.result ?? null);
    }
    return;
  }
  const p = msg.params ?? {};
  const reply = (result) => msg.id !== undefined && out({ id: msg.id, result });
  const refuse = (message) => msg.id !== undefined && out({ id: msg.id, error: { code: -32600, message } });
  switch (msg.method) {
    case 'initialize':
      return reply({
        userAgent: `escouade/${VERSION}`,
        codexHome: process.env.CODEX_HOME ?? '~/.codex',
        platformFamily: process.platform,
        platformOs: process.platform,
      });
    case 'initialized':
      return;
    case 'thread/start':
      cwd = p.cwd ?? cwd;
      instructions = p.developerInstructions ?? '';
      thread = id('thr');
      if (process.env.CODEX_HOME) ui.newRollout(process.env.CODEX_HOME, thread, cwd);
      reply({ thread: threadObject(thread) });
      return notify('thread/started', { thread: threadObject(thread) });
    case 'thread/resume':
      if (String(p.threadId).startsWith('perdu')) return refuse(`no rollout found for thread id ${p.threadId}`);
      cwd = p.cwd ?? cwd;
      thread = p.threadId;
      return reply({ thread: threadObject(thread) });
    case 'thread/fork': {
      cwd = p.cwd ?? cwd;
      const of = p.threadId;
      thread = id('thr-fork');
      if (process.env.CODEX_HOME) ui.newRollout(process.env.CODEX_HOME, thread, cwd, { forked_from_id: of });
      reply({ thread: threadObject(thread, { forkedFromId: of }) });
      return notify('thread/started', { thread: threadObject(thread, { forkedFromId: of }) });
    }
    case 'model/list': {
      // By pages, as the real one may (`nextCursor`): one model a page unless `limit` says more; the hidden ones only
      // when asked (`includeHidden`), as Codex keeps them out of its picker.
      const all = MODELS.filter((m) => p.includeHidden || !m.hidden);
      const from = Number(p.cursor ?? 0);
      const size = p.limit || 1;
      const next = from + size < all.length ? String(from + size) : null;
      return reply({ data: all.slice(from, from + size), nextCursor: next });
    }
    case 'account/rateLimits/read':
      return reply({ rateLimits: (!usage && ownQuota()) || RATE(usage || 12), rateLimitsByLimitId: {} });
    case 'hooks/list': {
      const trust = ui.trustOf(argv);
      const hooks = ui.hooksOf(argv).map((h, i) => ({
        key: h.key,
        eventName: h.event.charAt(0).toLowerCase() + h.event.slice(1),
        handlerType: 'command',
        command: h.command,
        source: 'sessionFlags',
        displayOrder: i,
        enabled: true,
        currentHash: h.hash,
        trustStatus: trust[h.key] === h.hash ? 'trusted' : 'untrusted',
      }));
      return reply({ data: (p.cwds ?? [cwd]).map((c) => ({ cwd: c, hooks })) });
    }
    case 'thread/compact/start':
      if (turn) return refuse('a turn is already running');
      reply({});
      return compact();
    case 'thread/name/set':
      return reply({});
    case 'turn/start':
      if (turn) return refuse('a turn is already running');
      startTurn(p).then(reply);
      return;
    case 'turn/steer':
      if (!turn || p.expectedTurnId !== turn.id) return refuse('no such turn running');
      reply({ turnId: turn.id });
      turn.wake?.({ steer: (p.input ?? []).map((i) => i.text ?? '').join('\n') });
      return;
    case 'turn/interrupt':
      reply({});
      if (turn) {
        // What it waited for goes with the turn.
        for (const [rid, w] of waiting) {
          waiting.delete(rid);
          notify('serverRequest/resolved', { threadId: thread, requestId: rid });
          w.resolve(null);
        }
        turn.wake?.({ interrupt: true });
      }
      return;
    default:
      return refuse(`unknown method ${msg.method}`);
  }
}

const rl = readline.createInterface({ input: process.stdin });
rl.on('line', (line) => {
  if (!line.trim()) return;
  try {
    handle(JSON.parse(line));
  } catch (e) {
    process.stderr.write(`fake codex: ${e.message}\n`);
  }
});
// Its input closed: it ends once the turn under way is over, as the app-server does.
rl.on('close', () => {
  const end = () => (turn ? setTimeout(end, 20) : process.exit(0));
  if (turn?.wake) turn.wake({ interrupt: true });
  end();
});
