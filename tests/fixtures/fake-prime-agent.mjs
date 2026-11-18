#!/usr/bin/env node
// A fake Prime Agent 0.10.3 (`prime-agent --mode acp --no-session`) for the tests of its plugin (ESC-485,
// plugins/prime-agent/): it speaks as the real one did in the sandbox of the study (tests/fixtures/prime-agent/,
// recorded): one tool, a cell of Python (`kind: execute`, title « Python cell », the code in `rawInput.code`), no mode
// (`session/set_mode` and `session/set_model` are « Method not found »), no permission ever asked, no `fs/*`, its model a
// configuration option whose values are `["<provider>","<model>"]`, `loadSession: false`, `session_info_update`s of its
// own under `_meta["ai.primeintellect.prime-agent"]`, no tokens. The words of each message pick what it plays:
//   python — a cell (`print(2 ** 100)`) and its output, then a reply;  long — a cell that starts a command in the
//   background, then a reply that waits, stopped by session/cancel (the turn `cancelled`);  serveurs — the MCP servers
//   its session was given (names and header names, no value);  anything else — « Prime Agent a reçu : <text> ».
// Its daemon: like the real one, `--mode acp` starts a detached process that outlives it (on Unix in a session of its
// own; on Windows detached, which no Job lets go), unless the one of $FAKE_PRIME_AGENT_DAEMON (a file, its pid) is alive.
// Each message, and the launch ({launch: {argv, cwd}}), goes to $FAKE_PRIME_AGENT_LOG when set.
import { spawn } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';

const argv = process.argv.slice(2);
const VERSION = process.env.FAKE_PRIME_AGENT_VERSION || '0.10.3';
if (argv.includes('--version') || argv.includes('-v')) {
  console.log(VERSION);
  process.exit(0);
}
const logFile = process.env.FAKE_PRIME_AGENT_LOG;
const log = (entry) => logFile && fs.appendFileSync(logFile, JSON.stringify(entry) + '\n');
log({ launch: { argv, cwd: process.cwd() } });
if (argv[argv.indexOf('--mode') + 1] !== 'acp') {
  console.error('fake prime-agent: only --mode acp is played');
  process.exit(2);
}

/** Whether the process `pid` runs. */
function alive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

// Its daemon, started at the first `--mode acp` of the user, shared by the next ones.
const daemonFile = process.env.FAKE_PRIME_AGENT_DAEMON;
if (daemonFile) {
  const known = fs.existsSync(daemonFile) ? Number(fs.readFileSync(daemonFile, 'utf8')) : 0;
  if (!known || !alive(known)) {
    const daemon = spawn(process.execPath, ['-e', 'setInterval(() => {}, 1 << 30)'], { detached: true, stdio: 'ignore' });
    daemon.unref();
    fs.writeFileSync(daemonFile, String(daemon.pid));
  }
}

const OWN = 'ai.primeintellect.prime-agent';
const MODELS = [
  ['ollama', 'qwen3-coder:30b-32k', 'Qwen3 Coder 30B (local)'],
  ['ollama', 'gpt-oss:20b-32k', 'GPT-OSS 20B (local)'],
];
const value = ([provider, id]) => JSON.stringify([provider, id]);
const sessions = new Map(); // ACP id → { model, mcpServers }
let running = null; // { session, cancelled }
let seq = 0;
let turns = 0;

const send = (msg) => {
  log({ dir: 's2c', msg });
  process.stdout.write(JSON.stringify(msg) + '\n');
};
const meta = (turn, more = {}) => ({ [OWN]: { promptTurnId: turn, eventSequence: ++seq, phase: 'event', ...more } });
const update = (id, u) => send({ jsonrpc: '2.0', method: 'session/update', params: { sessionId: id, update: u } });
const say = (id, text, turn) => update(id, { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text } });
const info = (id, turn, more) => update(id, { sessionUpdate: 'session_info_update', _meta: meta(turn, more) });
const options = (s) => [
  {
    id: 'model',
    name: 'Model',
    type: 'select',
    category: 'model',
    currentValue: s.model,
    options: MODELS.map((m) => ({ value: value(m), name: `${m[2]} (${m[0]})` })),
  },
];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function cell(id, turn, code, output) {
  const call = `call_${crypto.randomUUID().slice(0, 8)}`;
  update(id, {
    sessionUpdate: 'tool_call',
    toolCallId: call,
    title: 'Python cell',
    kind: 'execute',
    status: 'in_progress',
    rawInput: { code },
    _meta: meta(turn),
  });
  update(id, {
    sessionUpdate: 'tool_call_update',
    toolCallId: call,
    status: 'completed',
    content: [{ type: 'content', content: { type: 'text', text: output } }],
    _meta: meta(turn),
  });
}

async function prompt(msg) {
  const id = msg.params.sessionId;
  const s = sessions.get(id);
  if (!s)
    return send({
      jsonrpc: '2.0',
      id: msg.id,
      error: { code: -32603, message: 'Internal error', data: { details: `Unknown ACP session: ${id}` } },
    });
  if (running)
    return send({
      jsonrpc: '2.0',
      id: msg.id,
      error: { code: -32603, message: 'Internal error', data: { details: 'A prompt turn is already running' } },
    });
  const turn = ++turns;
  running = { session: id, cancelled: false };
  // The words of the message alone: the client's instructions come in a block before them.
  const text = (msg.params.prompt ?? []).at(-1)?.text ?? '';
  const asked = text.toLowerCase();
  info(id, turn, { compaction: {} });
  let stop = 'end_turn';
  if (asked.includes('python')) {
    cell(id, turn, 'print(2 ** 100)', '1267650600228229401496703205376\n');
    say(id, '2 ** 100 vaut 1267650600228229401496703205376.', turn);
  } else if (asked.includes('long')) {
    cell(id, turn, 'handle = bash("sleep 90")\nprint(handle.pid)', '4242\n');
    say(id, 'La commande tourne', turn);
    for (let waited = 0; waited < 30_000 && !running.cancelled; waited += 50) await sleep(50);
    if (running.cancelled) stop = 'cancelled';
    else say(id, ' et a fini.', turn);
  } else if (asked.includes('serveurs')) {
    const said = (s.mcpServers ?? []).map((m) => `${m.name} (${m.type}) ${(m.headers ?? []).map((h) => h.name).join(',')}`).join('; ');
    say(id, `Serveurs : ${said || 'aucun'}`, turn);
  } else {
    say(id, `Prime Agent a reçu : ${text}`, turn);
  }
  if (stop === 'end_turn') {
    info(id, turn, { phase: 'responseBoundary', outcome: 'result', terminalQuiescenceExpected: true });
    info(id, turn, {
      phase: 'terminalQuiescence',
      outcome: 'result',
      quiescence: { outstandingSubagents: 0, remainingAutonomousContinuations: 0 },
    });
  }
  running = null;
  send({ jsonrpc: '2.0', id: msg.id, result: { stopReason: stop } });
}

function handle(msg) {
  const reply = (result) => send({ jsonrpc: '2.0', id: msg.id, result });
  const fail = (code, message, data) => send({ jsonrpc: '2.0', id: msg.id, error: { code, message, ...(data ? { data } : {}) } });
  switch (msg.method) {
    case 'initialize':
      return reply({
        protocolVersion: 1,
        agentCapabilities: {
          loadSession: false,
          promptCapabilities: { image: true, embeddedContext: true },
          mcpCapabilities: { http: true },
          sessionCapabilities: { close: {} },
        },
        agentInfo: { name: 'prime-agent', title: 'Prime Agent', version: VERSION },
        _meta: { [OWN]: {} },
      });
    case 'session/new': {
      const id = crypto.randomUUID();
      const s = { model: value(MODELS[0]), mcpServers: msg.params.mcpServers };
      sessions.set(id, s);
      return reply({
        sessionId: id,
        configOptions: options(s),
        _meta: { [OWN]: { cwd: { requested: msg.params.cwd, actual: process.cwd() } } },
      });
    }
    case 'session/set_config_option': {
      const s = sessions.get(msg.params.sessionId);
      if (!s) return fail(-32603, 'Internal error', { details: `Unknown ACP session: ${msg.params.sessionId}` });
      if (msg.params.configId === 'model' && MODELS.some((m) => value(m) === msg.params.value)) {
        s.model = msg.params.value;
        return reply({ configOptions: options(s) });
      }
      return fail(-32602, 'Invalid params', { reason: `Invalid configuration option: ${msg.params.configId}` });
    }
    case 'session/prompt':
      return void prompt(msg);
    case 'session/cancel':
      if (running && running.session === msg.params.sessionId) running.cancelled = true;
      return;
    case 'session/close':
      sessions.delete(msg.params.sessionId);
      return reply({});
    default:
      if (msg.id !== undefined && msg.method) return fail(-32601, `"Method not found": ${msg.method}`, { method: msg.method });
  }
}

let buffer = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', (chunk) => {
  buffer += chunk;
  let at;
  while ((at = buffer.indexOf('\n')) >= 0) {
    const line = buffer.slice(0, at).trim();
    buffer = buffer.slice(at + 1);
    if (!line) continue;
    const msg = JSON.parse(line);
    log({ dir: 'c2s', msg });
    handle(msg);
  }
});
process.stdin.on('end', () => process.exit(0));
