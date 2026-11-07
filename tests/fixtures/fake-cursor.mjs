#!/usr/bin/env node
// A fake `agent acp` of Cursor CLI for the tests (ESC-42): ACP v1 (JSON-RPC 2.0, one message a line), as Cursor CLI
// 2026.10.22 speaks it (tests/fixtures/cursor/), its extensions `cursor/*` included. The words of each message pick
// what it plays:
//   bonjour (or anything else) — a reply;  raisonnement — a reasoning, then a reply;
//   commande — `npm test`, asking its permission (« Not in allowlist »), unless « Toujours » allowed it before
//   (`Shell(npm)` in permissions.allow of the cli-config.json of $CURSOR_CONFIG_DIR, as Cursor writes it);
//   fichier — notes.txt written in the agent's folder, its permission with its diff; « fichier dehors »: a file beside
//   the folder, changed;  mcp — a tool of an MCP server (essai, ping);
//   question — cursor/ask_question;  propose un plan — a plan update then cursor/create_plan;
//   sous-agent — tasks (updateTodos, cursor/update_todos) and a subagent (task, subagent_spawned, its own session,
//   cursor/task);  lent — a turn of 4 s (a command), stopped by session/cancel;
//   modèle — says the model and mode it runs;  compte — says the identity it runs as: the key of $CURSOR_API_KEY (its
//   last four characters) or the machine's sign-in (ESC-43);  limite — the usage limit of the month, as Cursor says it
//   (« Upgrade your plan to continue » after its words, `end_turn`);  bloquée — a write refused (by a hook, say): its
//   call ends `completed` with no diff, as Cursor ends it.
//   outil escouade <outil> — calls <outil> of Escouade's MCP server, by the entry `escouade` its session was given
//   (`mcpServers` of session/new or session/load) and its header, and says its answer.
// CURSOR_API_KEY=cle-refusee-… : every turn answers the text of a key Cursor refuses.
// FAKE_CURSOR_FREE=1 (or {"free": true} in $CURSOR_CONFIG_DIR/fake.json): a model other than Auto answers only
// « Upgrade your plan to continue », as a Free account does. A second session/prompt during a turn cancels it, as
// Cursor does. Sessions are kept in $CURSOR_CONFIG_DIR/acp-sessions/<id>.json and replayed by session/load. Each
// message, and each launch ({launch: {argv, cwd, configDir, msystem, apiKey, authToken}}: the fake values of the tests),
// goes to $FAKE_CURSOR_LOG when set.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const argv = process.argv.slice(2);
if (argv.includes('--version')) {
  console.log(process.env.FAKE_CURSOR_VERSION || '2026.10.22-27dcf5f');
  process.exit(0);
}
const logFile = process.env.FAKE_CURSOR_LOG;
const log = (entry) => logFile && fs.appendFileSync(logFile, JSON.stringify(entry) + '\n');
const home = process.env.CURSOR_CONFIG_DIR || path.join(process.cwd(), '.fake-cursor');
const apiKey = process.env.CURSOR_API_KEY || null;
log({
  launch: {
    argv,
    cwd: process.cwd(),
    configDir: process.env.CURSOR_CONFIG_DIR ?? null,
    msystem: process.env.MSYSTEM ?? null,
    apiKey,
    authToken: process.env.CURSOR_AUTH_TOKEN || null,
  },
});
if (argv[0] !== 'acp') {
  if (argv.includes('-p') || argv.includes('--print')) {
    process.stderr.write('fake cursor: only `agent acp` and its interface are played\n');
    process.exit(1);
  }
  // Its interface (« Vue Cursor »): fake-cursor-tui.mjs, which ends the process itself.
  const { tui } = await import('./fake-cursor-tui.mjs');
  tui(argv);
  await new Promise(() => {});
}
const sessionsDir = path.join(home, 'acp-sessions');
fs.mkdirSync(sessionsDir, { recursive: true });
const configFile = path.join(home, 'cli-config.json');
const readConfig = () => {
  try {
    return JSON.parse(fs.readFileSync(configFile, 'utf8'));
  } catch {
    return { version: 1, permissions: { allow: ['Shell(ls)'], deny: [] } };
  }
};
const writeConfig = (c) => fs.writeFileSync(configFile, JSON.stringify(c, null, 2));
const fake = (() => {
  try {
    return JSON.parse(fs.readFileSync(path.join(home, 'fake.json'), 'utf8'));
  } catch {
    return {};
  }
})();
const free = !!(process.env.FAKE_CURSOR_FREE || fake.free);

const MODELS = [
  { modelId: 'default[]', name: 'Auto' },
  { modelId: 'claude-opus-5-5[context=300k,effort=medium,fast=false]', name: 'claude-opus-5-5' },
  { modelId: 'gpt-5.5[context=272k,reasoning=medium,fast=false]', name: 'gpt-5.5' },
  { modelId: 'composer-2.5[fast=true]', name: 'composer-2.5' },
];
const OPTIONS = [
  { value: 'default', name: 'Auto', configOptions: [] },
  {
    value: 'claude-opus-5-5',
    name: 'Claude Opus 5.5',
    configOptions: [
      {
        id: 'effort',
        name: 'Effort',
        category: 'thought_level',
        type: 'select',
        currentValue: 'medium',
        options: ['low', 'medium', 'high'].map((v) => ({ value: v, name: v })),
      },
    ],
  },
  {
    value: 'gpt-5.5',
    name: 'GPT-5.5',
    configOptions: [
      {
        id: 'reasoning',
        name: 'Reasoning',
        category: 'thought_level',
        type: 'select',
        currentValue: 'medium',
        options: ['low', 'medium', 'high', 'xhigh'].map((v) => ({ value: v, name: v })),
      },
    ],
  },
  {
    value: 'composer-2.5',
    name: 'Composer 2.5',
    configOptions: [{ id: 'fast', category: 'model_config', type: 'select', currentValue: 'true', options: [] }],
  },
];
const MODES = [
  { id: 'agent', name: 'Agent', description: 'Full agent capabilities with tool access' },
  { id: 'plan', name: 'Plan', description: 'Read-only mode for planning and designing before implementation' },
  { id: 'ask', name: 'Ask', description: 'Q&A mode - no edits or command execution' },
];

function write(msg) {
  const m = { jsonrpc: '2.0', ...msg };
  log({ dir: 's2c', msg: m });
  process.stdout.write(JSON.stringify(m) + '\n');
}
const result = (id, r) => write({ id, result: r });
const error = (id, message, code = -32603) => write({ id, error: { code, message } });

// ----- requests to the client -----
let nextId = 0;
const waiting = new Map();
function ask(method, params) {
  const id = nextId++;
  write({ id, method, params });
  return new Promise((resolve) => waiting.set(id, resolve));
}

// ----- sessions -----
const sessions = new Map();
const fileOf = (id) => path.join(sessionsDir, `${id}.json`);
function save(s) {
  fs.writeFileSync(fileOf(s.id), JSON.stringify({ id: s.id, cwd: s.cwd, history: s.history }));
}
function config() {
  const c = readConfig();
  return c.model?.modelId ?? 'default[]';
}
function opened(s) {
  const model = config();
  return {
    modes: { currentModeId: s.mode, availableModes: MODES },
    models: { currentModelId: model, availableModels: MODELS },
    configOptions: [
      {
        id: 'mode',
        name: 'Mode',
        category: 'mode',
        type: 'select',
        currentValue: s.mode,
        options: MODES.map((m) => ({ value: m.id, name: m.name })),
      },
      {
        id: 'model',
        name: 'Model',
        category: 'model',
        type: 'select',
        currentValue: model,
        options: MODELS.map((m) => ({ value: m.modelId, name: m.name })),
      },
    ],
  };
}
function update(s, u, keep = true) {
  write({ method: 'session/update', params: { sessionId: s.id, update: u } });
  if (keep && /chunk$/.test(u.sessionUpdate)) {
    s.history.push(u);
  }
}
const say = (s, text) => {
  // In small pieces, as Cursor sends them.
  for (const piece of text.match(/.{1,12}/gsu) ?? [])
    update(s, { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: piece } });
};
const think = (s, text) => update(s, { sessionUpdate: 'agent_thought_chunk', content: { type: 'text', text } });
const callId = (n) => `call-${crypto.randomUUID()}-${n}\nfc_${crypto.randomUUID()}_0`;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const allowed = (rule) => (readConfig().permissions?.allow ?? []).includes(rule);
function allowAlways(rule) {
  const c = readConfig();
  c.permissions = c.permissions ?? { allow: [], deny: [] };
  if (!c.permissions.allow.includes(rule)) c.permissions.allow.push(rule);
  writeConfig(c);
}
const OPTS = [
  { optionId: 'allow-once', name: 'Allow once', kind: 'allow_once' },
  { optionId: 'allow-always', name: 'Allow always', kind: 'allow_always' },
  { optionId: 'reject-once', name: 'Reject', kind: 'reject_once' },
];
async function permission(s, toolCall, rule) {
  const r = await ask('session/request_permission', { sessionId: s.id, toolCall: { status: 'pending', ...toolCall }, options: OPTS });
  const option = r?.outcome?.optionId;
  if (option === 'allow-always' && rule) allowAlways(rule);
  return option === 'allow-once' || option === 'allow-always';
}

/** The text of the tool `tool` of the MCP server `entry` (`{url, headers: [{name, value}]}`, as ACP gives it), called as
 * Cursor's client does: initialize, initialized, tools/call. */
async function escouadeTool(entry, tool) {
  const base = { 'content-type': 'application/json', accept: 'application/json, text/event-stream' };
  for (const h of entry.headers ?? []) base[h.name] = h.value;
  let session = null;
  const call = async (method, params, id) => {
    const headers = { ...base, ...(session ? { 'mcp-session-id': session } : {}) };
    const body = JSON.stringify(id === undefined ? { jsonrpc: '2.0', method, params } : { jsonrpc: '2.0', id, method, params });
    const r = await fetch(entry.url, { method: 'POST', headers, body });
    session = r.headers.get('mcp-session-id') ?? session;
    const text = await r.text();
    if (!r.ok) throw new Error(`${r.status}`);
    if (id === undefined) return null;
    const answers = /^data:/m.test(text)
      ? text
          .split('\n')
          .filter((l) => l.startsWith('data:'))
          .map((l) => JSON.parse(l.slice(5)))
      : [JSON.parse(text)];
    const answer = answers.find((a) => a.id === id);
    if (answer?.error) throw new Error(answer.error.message);
    return answer?.result;
  };
  await call('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'Cursor', version: '0' } }, 1);
  await call('notifications/initialized', {});
  const result = await call('tools/call', { name: tool, arguments: {} }, 2);
  return (result?.content ?? []).map((c) => c.text ?? '').join('');
}

async function turn(s, text, cancelled) {
  const words = text.split('</instructions>').pop().trim();
  const model = config();
  if (free && model !== 'default[]') {
    say(s, 'Upgrade your plan to continue');
    return;
  }
  if (apiKey?.startsWith('cle-refusee')) {
    say(
      s,
      '\n\nError: [unauthenticated] Backend rejected authentication. Verify this is a User API Key for the same endpoint/environment, then rerun.',
    );
    return;
  }
  if (words.includes('compte')) {
    say(s, apiKey ? `Compte : clé …${apiKey.slice(-4)}` : 'Compte : connexion de la machine');
    return;
  }
  if (words.startsWith('outil escouade ')) {
    // An Escouade tool called through the server its session was given (`mcpServers`), with its header (ESC-43).
    const tool = words.slice('outil escouade '.length).trim();
    const entry = s.mcp.find((m) => m.name === 'escouade');
    if (!entry) return say(s, 'Pas de serveur escouade dans la session.');
    try {
      say(s, `Escouade a répondu : ${await escouadeTool(entry, tool)}`);
    } catch (e) {
      say(s, `Escouade a refusé : ${e.message}`);
    }
    return;
  }
  if (words.includes('limite')) {
    say(s, 'Je commence.');
    say(s, '\n\nUpgrade your plan to continue');
    return;
  }
  if (words.includes('bloquée')) {
    const file = path.join(s.cwd, 'bloquee.txt');
    const id = callId(10);
    update(s, { sessionUpdate: 'tool_call', toolCallId: id, title: 'Edit File', kind: 'edit', status: 'pending', rawInput: {} });
    update(s, {
      sessionUpdate: 'tool_call_update',
      toolCallId: id,
      title: `Edit \`${file}\``,
      rawInput: { path: file },
      locations: [{ path: file }],
    });
    update(s, { sessionUpdate: 'tool_call_update', toolCallId: id, status: 'in_progress' });
    update(s, { sessionUpdate: 'tool_call_update', toolCallId: id, status: 'completed' });
    say(s, 'Je n’ai pas pu écrire bloquee.txt.');
    return;
  }
  if (s.history.length === 0) update(s, { sessionUpdate: 'session_info_update', title: words.slice(0, 30) }, false);
  if (words.includes('raisonnement')) {
    think(s, 'Je réfléchis à la question posée.');
    say(s, 'Voici ma réponse réfléchie.');
  } else if (words.includes('commande')) {
    const id = callId(1);
    const call = { toolCallId: id, title: '`npm test`', kind: 'execute', rawInput: { command: 'npm test' } };
    update(s, { sessionUpdate: 'tool_call', ...call, status: 'pending' });
    update(s, { sessionUpdate: 'tool_call_update', toolCallId: id, status: 'in_progress' });
    const ok =
      allowed('Shell(npm)') ||
      (await permission(
        s,
        { ...call, content: [{ type: 'content', content: { type: 'text', text: 'Not in allowlist: npm' } }] },
        'Shell(npm)',
      ));
    if (ok) {
      update(s, {
        sessionUpdate: 'tool_call_update',
        toolCallId: id,
        status: 'completed',
        rawOutput: { exitCode: 0, stdout: '1 test passed\n', stderr: '' },
      });
      say(s, 'La commande a marché.');
    } else {
      update(s, { sessionUpdate: 'tool_call_update', toolCallId: id, status: 'failed', rawOutput: { error: 'Rejected by user' } });
      say(s, 'Commande refusée.');
    }
  } else if (words.includes('fichier')) {
    const outside = words.includes('dehors');
    const file = outside ? path.resolve(s.cwd, '..', 'cursor-dehors.txt') : path.join(s.cwd, 'notes.txt');
    const before = fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : outside ? 'avant\n' : null;
    if (outside && !fs.existsSync(file)) fs.writeFileSync(file, before);
    const after = (before ?? '') + 'note de Cursor\n';
    const id = callId(2);
    update(s, { sessionUpdate: 'tool_call', toolCallId: id, title: 'Edit File', kind: 'edit', status: 'pending', rawInput: {} });
    update(s, {
      sessionUpdate: 'tool_call_update',
      toolCallId: id,
      title: `Edit \`${file}\``,
      rawInput: { path: file },
      locations: [{ path: file }],
    });
    update(s, { sessionUpdate: 'tool_call_update', toolCallId: id, status: 'in_progress' });
    const diff = [{ type: 'diff', path: file, oldText: before, newText: after }];
    // In the agent's folder, a write asks nothing (as Cursor 2026.10.22 does); beside it, the fake submits it, with its
    // diff, as Cursor does for a write it does not allow by itself.
    const ok =
      !outside ||
      (await permission(s, { toolCallId: id, title: `Edit \`${file}\``, kind: 'edit', content: diff, locations: [{ path: file }] }));
    if (ok) {
      fs.writeFileSync(file, after);
      // Its diff, as Cursor gives it at the end of a write.
      update(s, { sessionUpdate: 'tool_call_update', toolCallId: id, status: 'completed', content: diff });
      say(s, `J’ai modifié ${path.basename(file)}.`);
    } else {
      update(s, { sessionUpdate: 'tool_call_update', toolCallId: id, status: 'failed' });
      say(s, 'Modification refusée.');
    }
  } else if (words.includes('mcp')) {
    const id = callId(3);
    const call = {
      toolCallId: id,
      title: 'essai-ping: ping',
      kind: 'other',
      rawInput: { providerIdentifier: 'essai', toolName: 'ping', args: { message: 'salut' } },
    };
    update(s, { sessionUpdate: 'tool_call', ...call, status: 'pending' });
    const ok = allowed('Mcp(essai:ping)') || (await permission(s, call, 'Mcp(essai:ping)'));
    update(s, { sessionUpdate: 'tool_call_update', toolCallId: id, status: ok ? 'completed' : 'failed', rawOutput: { content: 'pong' } });
    say(s, ok ? 'Le ping a répondu.' : 'Ping refusé.');
  } else if (words.includes('question')) {
    const questions = [
      {
        id: 'couleur',
        prompt: 'Quelle couleur préfères-tu ?',
        options: [
          { id: 'r', label: 'rouge' },
          { id: 'b', label: 'bleu' },
        ],
        allowMultiple: false,
      },
    ];
    const r = await ask('cursor/ask_question', { toolCallId: callId(4), title: 'Couleur', questions });
    const o = r?.outcome;
    if (o?.outcome === 'answered') {
      const picked = o.answers[0]?.selectedOptionIds?.[0];
      say(s, `Tu as choisi ${questions[0].options.find((x) => x.id === picked)?.label ?? 'rien'}.`);
    } else {
      say(s, `Tu as répondu : ${o?.reason ?? 'rien'}.`);
    }
  } else if (words.includes('plan')) {
    const todos = [
      { id: 't1', content: 'Ajouter la fonction sub', status: 'pending' },
      { id: 't2', content: 'Écrire son test', status: 'pending' },
    ];
    update(s, { sessionUpdate: 'plan', entries: todos.map((t) => ({ content: t.content, priority: 'medium', status: t.status })) }, false);
    const r = await ask('cursor/create_plan', {
      toolCallId: callId(5),
      name: 'Ajouter sub',
      overview: 'Une fonction de soustraction et son test.',
      plan: '1. Ajouter la fonction sub.\n2. Écrire son test.',
      todos,
    });
    const o = r?.outcome;
    say(s, o?.outcome === 'accepted' ? 'Plan approuvé, je commence.' : `Je revois le plan : ${o?.reason ?? ''}`);
  } else if (words.includes('sous-agent')) {
    const todo = callId(6);
    const tasks = [
      { id: '1', content: 'Lire le README', status: 'in_progress' },
      { id: '2', content: 'Résumer', status: 'pending' },
    ];
    update(s, {
      sessionUpdate: 'tool_call',
      toolCallId: todo,
      title: 'Update TODOs',
      kind: 'other',
      status: 'pending',
      rawInput: { _toolName: 'updateTodos' },
    });
    update(s, { sessionUpdate: 'tool_call_update', toolCallId: todo, status: 'completed' });
    await ask('cursor/update_todos', { toolCallId: todo, todos: tasks, merge: false });
    const task = callId(7);
    const prompt = 'Lis README.md et résume-le en une phrase.';
    update(s, {
      sessionUpdate: 'tool_call',
      toolCallId: task,
      title: 'Task: Résumer le README',
      kind: 'other',
      status: 'pending',
      rawInput: { _toolName: 'task', prompt, description: 'Résumer le README' },
    });
    update(s, { sessionUpdate: 'tool_call_update', toolCallId: task, status: 'in_progress' });
    const child = { id: crypto.randomUUID(), history: [] };
    const meta = { cursor: { toolCallId: task, agentId: child.id, model: 'default' } };
    update(
      s,
      {
        sessionUpdate: 'subagent_spawned',
        subagentSessionId: child.id,
        name: 'generalPurpose',
        task: prompt,
        capabilities: {},
        _meta: meta,
      },
      false,
    );
    const read = callId(8);
    const readme = path.join(s.cwd, 'README.md');
    update(
      child,
      {
        sessionUpdate: 'tool_call',
        toolCallId: read,
        title: 'Read README.md',
        kind: 'read',
        status: 'pending',
        rawInput: { path: readme },
        locations: [{ path: readme }],
      },
      false,
    );
    update(
      child,
      { sessionUpdate: 'tool_call_update', toolCallId: read, status: 'completed', rawOutput: { content: 'Résumé du README' } },
      false,
    );
    update(child, { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: 'Le README présente le projet.' } }, false);
    update(s, { sessionUpdate: 'subagent_state_update', subagentSessionId: child.id, state: 'completed', _meta: meta }, false);
    update(s, {
      sessionUpdate: 'tool_call_update',
      toolCallId: task,
      status: 'completed',
      rawOutput: { durationMs: 1200, isBackground: false },
    });
    await ask('cursor/task', {
      toolCallId: task,
      description: 'Résumer le README',
      prompt,
      subagentType: 'unspecified',
      model: 'default',
      agentId: child.id,
      durationMs: 1200,
    });
    await ask('cursor/update_todos', {
      toolCallId: todo,
      todos: [
        { id: '1', content: 'Lire le README', status: 'completed' },
        { id: '2', content: 'Résumer', status: 'completed' },
      ],
      merge: true,
    });
    say(s, 'Tâches faites, le sous-agent a résumé le README.');
  } else if (words.includes('lent')) {
    say(s, 'Je commence un long travail…');
    const id = callId(9);
    // « lent 600s » lasts that long, running the tests (the website's screenshots); a plain « lent », `sleep 4`.
    const long = /lent (\d+)s/.exec(words)?.[1];
    const command = long ? 'npm test -- auth' : 'sleep 4';
    update(s, {
      sessionUpdate: 'tool_call',
      toolCallId: id,
      title: `\`${command}\``,
      kind: 'execute',
      status: 'pending',
      rawInput: { command },
    });
    const tenths = Number(long ?? 4) * 10;
    for (let i = 0; i < tenths && !cancelled(); i++) await sleep(100);
    if (cancelled()) return;
    update(s, {
      sessionUpdate: 'tool_call_update',
      toolCallId: id,
      status: 'completed',
      rawOutput: { exitCode: 0, stdout: '', stderr: '' },
    });
    say(s, 'Long travail fini.');
  } else if (words.includes('modèle')) {
    say(s, `Je tourne sur ${model}, en mode ${s.mode}.`);
  } else {
    say(s, `Cursor a reçu : ${words}`);
  }
}

// ----- the protocol -----
let running = null;
async function onMessage(msg) {
  if (msg.method === undefined) {
    const resolve = waiting.get(msg.id);
    waiting.delete(msg.id);
    resolve?.(msg.result ?? null);
    return;
  }
  const p = msg.params ?? {};
  switch (msg.method) {
    case 'initialize': {
      const subagents = !!p.clientCapabilities?._meta?.subagents;
      return result(msg.id, {
        protocolVersion: 1,
        agentCapabilities: {
          loadSession: true,
          mcpCapabilities: { http: true, sse: true },
          promptCapabilities: { audio: false, embeddedContext: false, image: true },
          sessionCapabilities: subagents ? { list: {}, subagents: {} } : { list: {} },
        },
        authMethods: [{ id: 'cursor_login', name: 'Cursor Login' }],
      });
    }
    case 'session/new': {
      const s = { id: crypto.randomUUID(), cwd: p.cwd, history: [], mode: 'agent', mcp: p.mcpServers ?? [] };
      sessions.set(s.id, s);
      save(s);
      return result(msg.id, { sessionId: s.id, ...opened(s) });
    }
    case 'session/load': {
      if (!fs.existsSync(fileOf(p.sessionId))) return error(msg.id, 'Session not found');
      const kept = JSON.parse(fs.readFileSync(fileOf(p.sessionId), 'utf8'));
      const s = { ...kept, cwd: p.cwd, mode: 'agent', mcp: p.mcpServers ?? [] };
      sessions.set(s.id, s);
      // Its history replayed, as Cursor does, before its answer.
      for (const u of s.history) write({ method: 'session/update', params: { sessionId: s.id, update: u } });
      return result(msg.id, opened(s));
    }
    case 'session/set_mode':
    case 'session/set_config_option': {
      const s = sessions.get(p.sessionId);
      if (!s) return error(msg.id, 'Session not found');
      const key = msg.method === 'session/set_mode' ? 'mode' : p.configId;
      const value = msg.method === 'session/set_mode' ? p.modeId : p.value;
      if (key === 'mode') {
        s.mode = value;
        update(s, { sessionUpdate: 'current_mode_update', currentModeId: value }, false);
      } else if (key === 'model') {
        if (!MODELS.some((m) => m.modelId === value)) return error(msg.id, `Unknown model ${value}`);
        // Written in its config, as Cursor does: it holds for the next sessions too.
        const c = readConfig();
        c.model = { modelId: value };
        writeConfig(c);
      } else {
        const c = readConfig();
        c.modelOptions = { ...(c.modelOptions ?? {}), [key]: value };
        writeConfig(c);
      }
      return result(msg.id, msg.method === 'session/set_mode' ? {} : { configOptions: opened(s).configOptions });
    }
    case 'cursor/list_available_models':
      return result(msg.id, { models: OPTIONS });
    case 'session/list':
      return result(msg.id, { sessions: [...sessions.values()].map((s) => ({ sessionId: s.id, cwd: s.cwd })) });
    case 'session/cancel':
      if (running?.session === p.sessionId) running.cancelled = true;
      return;
    case 'session/prompt': {
      const s = sessions.get(p.sessionId);
      if (!s) return error(msg.id, 'Session not found');
      // A prompt during a turn cancels it, as Cursor does.
      if (running) running.cancelled = true;
      while (running) await sleep(20);
      const text = (p.prompt ?? [])
        .filter((b) => b.type === 'text')
        .map((b) => b.text)
        .join('\n');
      const images = (p.prompt ?? []).filter((b) => b.type === 'image').length;
      s.history.push({ sessionUpdate: 'user_message_chunk', content: { type: 'text', text } });
      const run = { session: s.id, cancelled: false };
      running = run;
      // A turn takes a moment, as a real one does: the app shows the message sent before its end.
      await sleep(40);
      // What it plays is picked by the user's own words: not those of Escouade's blocks before them (the instructions,
      // what was said in its interface).
      const own = text.replace(/<(instructions|relais_escouade)>[\s\S]*?<\/\1>\n?/g, '').trim();
      try {
        await turn(s, images ? `${own} (avec ${images} image)` : own, () => run.cancelled);
      } finally {
        running = null;
        save(s);
      }
      return result(msg.id, { stopReason: run.cancelled ? 'cancelled' : 'end_turn' });
    }
    default:
      if (msg.id !== undefined) error(msg.id, `Method not found: ${msg.method}`, -32601);
  }
}

let buf = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', (d) => {
  buf += d;
  for (;;) {
    const nl = buf.indexOf('\n');
    if (nl < 0) return;
    const line = buf.slice(0, nl).trim();
    buf = buf.slice(nl + 1);
    if (!line) continue;
    let msg;
    try {
      msg = JSON.parse(line);
    } catch {
      continue;
    }
    log({ dir: 'c2s', msg });
    onMessage(msg);
  }
});
// Its input closed, `agent acp` goes on (tried on 2026.10.22): Escouade kills it.
process.stdin.on('end', () => {});
