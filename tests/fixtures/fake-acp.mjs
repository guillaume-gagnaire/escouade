#!/usr/bin/env node
// A fake ACP agent for the tests of the generic ACP plugin (PLG-6, `dev.escouade.acp`): plain ACP v1 (JSON-RPC 2.0, one
// message a line), no extension of anyone's, its files read and written through the client (`fs/read_text_file`,
// `fs/write_text_file`) when the client says it can, else by itself. It is described by the manifest of
// tests/fixtures/plugins/fake-acp/: not a line of Rust of its own. The words of each message pick what it plays:
//   bonjour (or anything else) — a reply;  raisonnement — a reasoning, then a reply;
//   commande — `npm test`, asking its permission, run once allowed;
//   fichier — notes.txt of its folder written (« bonjour »), its permission with its diff, through the client;
//   lis <path> — reads <path> through the client and says what came back (a file of a private project is refused);
//   lis seul <path> — reads <path> by itself (a call of kind `read`, no `fs/*`): what the client cannot fence;
//   question — the turn ends on a block <escouade_question> (no tool to ask in ACP); « The user answered » — says it;
//   sous-agent — a call of its tool `task`, a subagent that reads README.md;  lent — a turn of 4 s, stopped by
//   session/cancel;  mode — says the mode it is in;  serveurs — says the MCP servers its session was given (no token).
// Sessions are kept in $FAKE_ACP_HOME/sessions/<id>.json and replayed by session/load. Each message, and each launch
// ({launch: {argv, cwd, home}}), goes to $FAKE_ACP_LOG when set.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const argv = process.argv.slice(2);
if (argv.includes('--version')) {
  console.log(process.env.FAKE_ACP_VERSION || 'fake-acp 1.2.0');
  process.exit(0);
}
const logFile = process.env.FAKE_ACP_LOG;
const log = (entry) => logFile && fs.appendFileSync(logFile, JSON.stringify(entry) + '\n');
const home = process.env.FAKE_ACP_HOME || path.join(process.cwd(), '.fake-acp');
log({ launch: { argv, cwd: process.cwd(), home: process.env.FAKE_ACP_HOME ?? null } });
const sessionsDir = path.join(home, 'sessions');
fs.mkdirSync(sessionsDir, { recursive: true });

let clientFs = { readTextFile: false, writeTextFile: false };
const sessions = new Map(); // id → { cwd, mode, history: [update], mcpServers }
let nextId = 1000;
const waiting = new Map(); // our request id → resolve
let running = null; // { session, cancelled, wake }

const send = (msg) => {
  log({ dir: 's2c', msg });
  process.stdout.write(JSON.stringify(msg) + '\n');
};
const ask = (method, params) =>
  new Promise((resolve) => {
    const id = nextId++;
    waiting.set(id, resolve);
    send({ jsonrpc: '2.0', id, method, params });
  });
const update = (s, u) => {
  s.history.push(u);
  save(s);
  send({ jsonrpc: '2.0', method: 'session/update', params: { sessionId: s.id, update: u } });
};
const say = (s, text) => {
  // In two pieces, as an agent streams them.
  const half = Math.ceil(text.length / 2);
  update(s, { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: text.slice(0, half) } });
  update(s, { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: text.slice(half) } });
};
const think = (s, text) => update(s, { sessionUpdate: 'agent_thought_chunk', content: { type: 'text', text } });
const save = (s) =>
  fs.writeFileSync(path.join(sessionsDir, `${s.id}.json`), JSON.stringify({ cwd: s.cwd, mode: s.mode, history: s.history }));
const sleep = (ms) =>
  new Promise((resolve) => {
    const t = setTimeout(resolve, ms);
    if (running) running.wake = () => (clearTimeout(t), resolve());
  });
const MODES = {
  currentModeId: 'ask',
  availableModes: [
    { id: 'ask', name: 'Ask' },
    { id: 'auto', name: 'Auto' },
    { id: 'read', name: 'Read only' },
  ],
};

async function permission(s, toolCall) {
  const r = await ask('session/request_permission', {
    sessionId: s.id,
    toolCall,
    options: [
      { optionId: 'yes', name: 'Allow', kind: 'allow_once' },
      { optionId: 'always', name: 'Always', kind: 'allow_always' },
      { optionId: 'no', name: 'Reject', kind: 'reject_once' },
    ],
  });
  return r.result?.outcome?.outcome === 'selected' && r.result.outcome.optionId !== 'no';
}

async function readFile(p) {
  if (clientFs.readTextFile) {
    const r = await ask('fs/read_text_file', { sessionId: current().id, path: p });
    return r.error ? { error: r.error.message } : { content: r.result.content };
  }
  try {
    return { content: fs.readFileSync(p, 'utf8') };
  } catch (e) {
    return { error: String(e.message) };
  }
}

async function writeFile(p, content) {
  if (clientFs.writeTextFile) {
    const r = await ask('fs/write_text_file', { sessionId: current().id, path: p, content });
    return r.error ? { error: r.error.message } : {};
  }
  fs.writeFileSync(p, content);
  return {};
}

let currentSession = null;
const current = () => currentSession;

async function turn(s, text) {
  const words = text.toLowerCase();
  if (text.startsWith('The user answered your questions')) {
    say(s, `Tu as répondu : ${text.split('→').pop().trim()}`);
  } else if (words.includes('raisonnement')) {
    think(s, 'Je réfléchis à la question.');
    say(s, 'Voici ma réponse réfléchie.');
  } else if (words.includes('commande')) {
    const call = { toolCallId: 'call-cmd', title: '`npm test`', kind: 'execute', status: 'pending', rawInput: { command: 'npm test' } };
    update(s, { sessionUpdate: 'tool_call', ...call });
    if (await permission(s, call)) {
      update(s, {
        sessionUpdate: 'tool_call_update',
        toolCallId: 'call-cmd',
        status: 'completed',
        rawOutput: { stdout: 'tests ok', exitCode: 0 },
      });
      say(s, 'Les tests passent.');
    } else {
      update(s, { sessionUpdate: 'tool_call_update', toolCallId: 'call-cmd', status: 'failed' });
      say(s, 'Commande refusée.');
    }
  } else if (words.includes('fichier')) {
    const file = path.join(s.cwd, 'notes.txt');
    const before = await readFile(file);
    const oldText = before.error ? null : before.content;
    const call = { toolCallId: 'call-edit', title: 'Write notes.txt', kind: 'edit', status: 'pending', locations: [{ path: file }] };
    update(s, { sessionUpdate: 'tool_call', ...call });
    const diff = { type: 'diff', path: file, oldText, newText: 'bonjour\n' };
    if (await permission(s, { ...call, content: [diff] })) {
      const w = await writeFile(file, 'bonjour\n');
      if (w.error) {
        update(s, { sessionUpdate: 'tool_call_update', toolCallId: 'call-edit', status: 'failed', rawOutput: { error: w.error } });
        say(s, `Écriture refusée : ${w.error}`);
      } else {
        update(s, { sessionUpdate: 'tool_call_update', toolCallId: 'call-edit', status: 'completed', content: [diff] });
        say(s, 'notes.txt écrit.');
      }
    } else {
      update(s, { sessionUpdate: 'tool_call_update', toolCallId: 'call-edit', status: 'failed' });
      say(s, 'Écriture refusée.');
    }
  } else if (words.startsWith('lis seul ')) {
    const p = text.slice('lis seul '.length).trim();
    update(s, {
      sessionUpdate: 'tool_call',
      toolCallId: 'call-read-alone',
      title: `Read ${p}`,
      kind: 'read',
      status: 'in_progress',
      locations: [{ path: p }],
    });
    let content;
    try {
      content = fs.readFileSync(p, 'utf8');
    } catch (e) {
      content = String(e.message);
    }
    update(s, {
      sessionUpdate: 'tool_call_update',
      toolCallId: 'call-read-alone',
      status: 'completed',
      content: [{ type: 'content', content: { type: 'text', text: content } }],
    });
    say(s, `Lu seul : ${content.trim()}`);
  } else if (words.startsWith('lis ')) {
    const p = text.slice('lis '.length).trim();
    update(s, {
      sessionUpdate: 'tool_call',
      toolCallId: 'call-read',
      title: `Read ${p}`,
      kind: 'read',
      status: 'in_progress',
      locations: [{ path: p }],
    });
    const r = await readFile(p);
    const shown = r.error ? `refusé : ${r.error}` : r.content.trim();
    update(s, {
      sessionUpdate: 'tool_call_update',
      toolCallId: 'call-read',
      status: r.error ? 'failed' : 'completed',
      content: [{ type: 'content', content: { type: 'text', text: shown } }],
    });
    say(s, `Lu : ${shown}`);
  } else if (words.includes('question')) {
    const block = { questions: [{ question: 'Quelle couleur préfères-tu ?', options: ['rouge', 'bleu'], multiSelect: false }] };
    say(s, `Il me faut ton choix.\n<escouade_question>${JSON.stringify(block)}</escouade_question>`);
  } else if (words.includes('sous-agent')) {
    update(s, {
      sessionUpdate: 'tool_call',
      toolCallId: 'call-task',
      title: 'task',
      kind: 'other',
      status: 'in_progress',
      rawInput: { description: 'Lire le README', prompt: 'Résume README.md' },
    });
    update(s, {
      sessionUpdate: 'tool_call_update',
      toolCallId: 'call-task',
      status: 'completed',
      content: [{ type: 'content', content: { type: 'text', text: 'Le README est court.' } }],
    });
    say(s, 'Le sous-agent a résumé le README.');
  } else if (words.includes('lent')) {
    update(s, {
      sessionUpdate: 'tool_call',
      toolCallId: 'call-slow',
      title: '`sleep 4`',
      kind: 'execute',
      status: 'in_progress',
      rawInput: { command: 'sleep 4' },
    });
    say(s, 'Je patiente…');
    await sleep(4000);
    if (!running?.cancelled) {
      update(s, { sessionUpdate: 'tool_call_update', toolCallId: 'call-slow', status: 'completed', rawOutput: { stdout: '' } });
      say(s, 'Fini.');
    }
  } else if (words.includes('mode')) {
    say(s, `Mode : ${s.mode}`);
  } else if (words.includes('serveurs')) {
    say(
      s,
      `Serveurs : ${JSON.stringify(s.mcpServers.map((m) => ({ name: m.name, url: m.url, headers: (m.headers ?? []).map((h) => h.name) })))}`,
    );
  } else {
    say(s, `Faux ACP a reçu : ${text}`);
  }
}

async function handle(msg) {
  log({ dir: 'c2s', msg });
  if (msg.method === undefined) {
    const resolve = waiting.get(msg.id);
    if (resolve) {
      waiting.delete(msg.id);
      resolve(msg);
    }
    return;
  }
  const { id, method, params } = msg;
  const result = (r) => send({ jsonrpc: '2.0', id, result: r });
  const error = (code, message) => send({ jsonrpc: '2.0', id, error: { code, message } });
  switch (method) {
    case 'initialize':
      clientFs = params?.clientCapabilities?.fs ?? clientFs;
      return result({ protocolVersion: 1, agentCapabilities: { loadSession: true, promptCapabilities: {} }, authMethods: [] });
    case 'session/new': {
      const s = { id: crypto.randomUUID(), cwd: params.cwd, mode: 'ask', history: [], mcpServers: params.mcpServers ?? [] };
      sessions.set(s.id, s);
      currentSession = s;
      save(s);
      return result({ sessionId: s.id, modes: MODES });
    }
    case 'session/load': {
      const file = path.join(sessionsDir, `${params.sessionId}.json`);
      if (!fs.existsSync(file)) return error(-32002, `session ${params.sessionId} not found`);
      const kept = JSON.parse(fs.readFileSync(file, 'utf8'));
      const s = { id: params.sessionId, cwd: params.cwd, mode: kept.mode, history: kept.history, mcpServers: params.mcpServers ?? [] };
      sessions.set(s.id, s);
      currentSession = s;
      for (const u of kept.history) send({ jsonrpc: '2.0', method: 'session/update', params: { sessionId: s.id, update: u } });
      return result({ modes: { ...MODES, currentModeId: s.mode } });
    }
    case 'session/set_mode': {
      const s = sessions.get(params.sessionId);
      if (!s) return error(-32602, 'no such session');
      s.mode = params.modeId;
      save(s);
      result({});
      return send({
        jsonrpc: '2.0',
        method: 'session/update',
        params: { sessionId: s.id, update: { sessionUpdate: 'current_mode_update', currentModeId: s.mode } },
      });
    }
    case 'session/set_model':
      return result({});
    case 'session/prompt': {
      const s = sessions.get(params.sessionId);
      if (!s) return error(-32602, 'no such session');
      currentSession = s;
      const text =
        params.prompt
          .filter((b) => b.type === 'text')
          .map((b) => b.text)
          .pop() ?? '';
      update(s, { sessionUpdate: 'user_message_chunk', content: { type: 'text', text } });
      running = { session: s.id, cancelled: false, wake: null };
      await turn(s, text);
      const cancelled = running.cancelled;
      running = null;
      return result({ stopReason: cancelled ? 'cancelled' : 'end_turn' });
    }
    case 'session/cancel':
      if (running) {
        running.cancelled = true;
        running.wake?.();
      }
      return;
    default:
      if (id !== undefined) error(-32601, `unknown method ${method}`);
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
    let msg;
    try {
      msg = JSON.parse(line);
    } catch {
      continue;
    }
    handle(msg).catch((e) => process.stderr.write(`fake acp: ${e.stack}\n`));
  }
});
process.stdin.on('end', () => process.exit(0));
