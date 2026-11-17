#!/usr/bin/env node
// A fake Unharnessed 0.9.8 (`unharnessed --mode acp`) for the tests of its plugin (ESC-484, plugins/unharnessed/): it
// speaks as the real one did in the sandbox of the study (tests/fixtures/unharnessed/, recorded): one tool, a cell of
// Python (`kind: execute`, title « Python cell », the code in `rawInput.code`), no mode, no permission ever asked, its
// model a configuration option whose values are `["<provider>","<model>"]`, its cost and tokens in the `_meta` of a
// `session_info_update`, notifications of its own (`_unharnessed/kernel/changed`). A process holds one session: a
// `session/load` of another is refused unless it was started with `--resume <id>`, as the real one does. The words of
// each message pick what it plays:
//   python — a cell (`print(2 ** 100)`) and its output, then a reply;  long — a cell that waits, stopped by
//   session/cancel (the cell « Tool execution aborted », the turn `cancelled`);  réveil — a reply, then a turn of its
//   own (a wake), no prompt asked, ended by the phases the real one ends its turns with;  serveurs — the MCP servers its session was given (names and header names, no
//   value);  anything else — « Unharnessed a reçu : <text> ».
// Sessions are kept in $UNHARNESSED_CODING_AGENT_DIR/sessions/<id>.json. Each message, and the launch ({launch: {argv,
// home}}), goes to $FAKE_UNHARNESSED_LOG when set.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const argv = process.argv.slice(2);
if (argv.includes('--version') || argv.includes('-v')) {
  console.log(process.env.FAKE_UNHARNESSED_VERSION || '0.9.8');
  process.exit(0);
}
const logFile = process.env.FAKE_UNHARNESSED_LOG;
const log = (entry) => logFile && fs.appendFileSync(logFile, JSON.stringify(entry) + '\n');
const home = process.env.UNHARNESSED_CODING_AGENT_DIR || path.join(process.cwd(), '.fake-unharnessed');
log({ launch: { argv, home: process.env.UNHARNESSED_CODING_AGENT_DIR ?? null } });
const mode = argv[argv.indexOf('--mode') + 1];
if (!argv.includes('--mode') || mode !== 'acp') {
  console.error('fake unharnessed: only --mode acp is played');
  process.exit(2);
}
const sessionsDir = path.join(home, 'sessions');
fs.mkdirSync(sessionsDir, { recursive: true });

const MODELS = [
  ['ollama', 'qwen3-coder:30b-32k', 'Qwen3 Coder 30B'],
  ['ollama', 'gpt-oss:20b-32k', 'GPT-OSS 20B'],
];
const value = ([provider, id]) => JSON.stringify([provider, id]);
// The saved session this process holds: a new one, or the one `--resume` names.
const resumed = argv.includes('--resume') ? argv[argv.indexOf('--resume') + 1] : null;
const held = resumed ? path.basename(resumed, '.jsonl') : crypto.randomUUID();
const sessions = new Map(); // ACP id → { saved, cwd, model, history, mcpServers }
let running = null; // { session, cancelled, wake }
let seq = 0;

const send = (msg) => {
  log({ dir: 's2c', msg });
  process.stdout.write(JSON.stringify(msg) + '\n');
};
const meta = (turn) => ({ unharnessed: { promptTurnId: turn, eventSequence: ++seq, phase: 'event' } });
const file = (saved) => path.join(sessionsDir, `${saved}.json`);
const save = (s) => fs.writeFileSync(file(s.saved), JSON.stringify({ cwd: s.cwd, model: s.model, history: s.history }));
const update = (id, s, u, kept = true) => {
  if (kept) {
    s.history.push(u);
    save(s);
  }
  send({ jsonrpc: '2.0', method: 'session/update', params: { sessionId: id, update: u } });
};
const say = (id, s, text, turn) =>
  update(id, s, {
    sessionUpdate: 'agent_message_chunk',
    messageId: `unharnessed-assistant-${turn}`,
    content: { type: 'text', text },
    _meta: meta(turn),
  });
const usage = (id, s, tokens) =>
  update(
    id,
    s,
    {
      sessionUpdate: 'session_info_update',
      _meta: { unharnessed: { ...meta(0).unharnessed, usage: { tokens, window: 32768, compactAt: 8192, cost: 0, billed: false } } },
    },
    false,
  );
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
const kernel = (id, what) => send({ jsonrpc: '2.0', method: '_unharnessed/kernel/changed', params: { sessionId: id, ...what } });

async function cell(id, s, turn, code, output, wait = 0) {
  const call = `call_${crypto.randomUUID().slice(0, 8)}`;
  update(id, s, {
    sessionUpdate: 'tool_call',
    toolCallId: call,
    title: 'Python cell',
    kind: 'execute',
    status: 'in_progress',
    rawInput: { code },
    _meta: meta(turn),
  });
  kernel(id, { commands: [] });
  for (let waited = 0; waited < wait && !running.cancelled; waited += 50) await sleep(50);
  if (running.cancelled) {
    update(id, s, {
      sessionUpdate: 'tool_call_update',
      toolCallId: call,
      status: 'failed',
      content: [{ type: 'content', content: { type: 'text', text: 'Tool execution aborted' } }],
      _meta: meta(turn),
    });
    return false;
  }
  update(id, s, {
    sessionUpdate: 'tool_call_update',
    toolCallId: call,
    status: 'completed',
    content: [{ type: 'content', content: { type: 'text', text: output } }],
    _meta: meta(turn),
  });
  kernel(id, { variables: [{ name: 'r', type: 'int', rev: '7fe0cc204cf0' }] });
  return true;
}

let turns = 0;
async function prompt(msg) {
  const id = msg.params.sessionId;
  const s = sessions.get(id);
  if (!s)
    return send({
      jsonrpc: '2.0',
      id: msg.id,
      error: { code: -32603, message: 'Internal error', data: { details: `Unknown ACP session: ${id}` } },
    });
  const text = (msg.params.prompt ?? []).map((b) => b.text ?? '').join('');
  const turn = ++turns;
  running = { session: id, cancelled: false };
  // Its message is not echoed during the turn, only in the replay of a `session/load`.
  s.history.push({
    sessionUpdate: 'user_message_chunk',
    messageId: `unharnessed-user-${turn}`,
    content: { type: 'text', text },
    _meta: meta(turn),
  });
  save(s);
  usage(id, s, 1200 + turn);
  let stop = 'end_turn';
  const asked = text.toLowerCase();
  if (asked.includes('python')) {
    await cell(id, s, turn, 'print(2 ** 100)', '1267650600228229401496703205376\n');
    say(id, s, '2 ** 100 vaut 1267650600228229401496703205376.', turn);
  } else if (asked.includes('long')) {
    const done = await cell(id, s, turn, "r = await bash('sleep 90')", 'BashResult(exit_code=0)', 30_000);
    if (!done) stop = 'cancelled';
    else say(id, s, 'Fini.', turn);
  } else if (asked.includes('serveurs')) {
    const said = (s.mcpServers ?? []).map((m) => `${m.name} (${m.type}) ${(m.headers ?? []).map((h) => h.name).join(',')}`).join('; ');
    say(id, s, `Serveurs : ${said || 'aucun'}`, turn);
  } else {
    // The words of the message alone: the client's instructions come in a block before them.
    say(id, s, `Unharnessed a reçu : ${(msg.params.prompt ?? []).at(-1)?.text ?? ''}`, turn);
  }
  usage(id, s, 1300 + turn);
  running = null;
  send({ jsonrpc: '2.0', id: msg.id, result: { stopReason: stop } });
  // A turn of its own, once a command of the last cell ended: no prompt asked it.
  if (asked.includes('réveil')) {
    await sleep(300);
    const wake = ++turns;
    update(
      id,
      s,
      {
        sessionUpdate: 'session_info_update',
        _meta: { unharnessed: { ...meta(wake).unharnessed, turnStart: { origin: 'wake', wake: 'bash' } } },
      },
      false,
    );
    say(id, s, 'Réveillé par la fin de sa commande.', wake);
    // Its end, as the real one marks the end of each of its turns (tests/fixtures/unharnessed/full-turn.jsonl).
    const phase = (p, more) =>
      update(
        id,
        s,
        { sessionUpdate: 'session_info_update', _meta: { unharnessed: { ...meta(wake).unharnessed, phase: p, ...more } } },
        false,
      );
    phase('responseBoundary', { outcome: 'result', terminalQuiescenceExpected: true });
    phase('terminalQuiescence', { outcome: 'result', quiescence: { remainingAutonomousContinuations: 0 } });
  }
}

function handle(msg) {
  const reply = (result) => send({ jsonrpc: '2.0', id: msg.id, result });
  const fail = (code, message, data) => send({ jsonrpc: '2.0', id: msg.id, error: { code, message, ...(data ? { data } : {}) } });
  switch (msg.method) {
    case 'initialize':
      return reply({
        protocolVersion: 1,
        agentCapabilities: {
          loadSession: true,
          promptCapabilities: { image: true, embeddedContext: true },
          mcpCapabilities: { http: true },
          sessionCapabilities: { close: {}, list: {} },
        },
        agentInfo: { name: 'unharnessed', title: 'Unharnessed', version: process.env.FAKE_UNHARNESSED_VERSION || '0.9.8' },
        _meta: { unharnessed: {} },
      });
    case 'session/new': {
      const id = crypto.randomUUID();
      const s = { saved: held, cwd: msg.params.cwd, model: value(MODELS[0]), history: [], mcpServers: msg.params.mcpServers };
      sessions.set(id, s);
      save(s);
      reply({ sessionId: id, configOptions: options(s), _meta: { unharnessed: { saved: { sessionId: held, path: file(held) } } } });
      update(
        id,
        s,
        {
          sessionUpdate: 'available_commands_update',
          availableCommands: [{ name: 'compact', description: 'Compact the session context' }],
          _meta: meta(0),
        },
        false,
      );
      return usage(id, s, 0);
    }
    case 'session/load': {
      const want = msg.params.sessionId;
      if (want !== held || !fs.existsSync(file(want))) {
        return fail(-32603, 'Internal error', {
          details: `This process holds session ${held}: start unharnessed with --resume <its file> to load ${want}.`,
        });
      }
      const kept = JSON.parse(fs.readFileSync(file(want), 'utf8'));
      const s = { saved: want, cwd: msg.params.cwd, model: kept.model, history: kept.history, mcpServers: msg.params.mcpServers };
      sessions.set(want, s);
      for (const u of kept.history) send({ jsonrpc: '2.0', method: 'session/update', params: { sessionId: want, update: u } });
      return reply({ sessionId: want, configOptions: options(s) });
    }
    case 'session/set_config_option': {
      const s = sessions.get(msg.params.sessionId);
      if (!s) return fail(-32603, 'Internal error', { details: `Unknown ACP session: ${msg.params.sessionId}` });
      if (msg.params.configId === 'model') {
        if (!MODELS.some((m) => value(m) === msg.params.value)) {
          return fail(-32602, 'Invalid params', { reason: `Unavailable model: ${msg.params.value}` });
        }
        s.model = msg.params.value;
        save(s);
        return reply({ configOptions: options(s) });
      }
      if (msg.params.configId === 'thought_level')
        return fail(-32602, 'Invalid params', { reason: `Unsupported reasoning effort: ${msg.params.value}` });
      return fail(-32602, 'Invalid params', { reason: `Unknown config option: ${msg.params.configId}` });
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
      if (msg.id !== undefined && msg.method) {
        return fail(-32601, `"Method not found": ${msg.method}`, { method: msg.method });
      }
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
