#!/usr/bin/env node
// A fake `agy` of Antigravity CLI for the tests (ESC-270): `agy -p= --input-format stream-json --output-format
// stream-json`, as `agy` 1.4.9 speaks it (tests/fixtures/agy/): `init` at its start, then for each message on its input
// (`{"event":"user","message":{"content":…}}`) its steps (`step_update`) and its `result`. In `-p` it asks nothing: what
// its rules do not allow is refused at once (the step `ERROR`, `denied_actions` in the result; a write ends `DONE`, its
// transcript `ERROR`, as `agy`'s), its questions are skipped. Its rules are the `permissions` of
// $USERPROFILE/.gemini/antigravity-cli/settings.json (or $HOME), read again before each tool, `deny` first. Its
// transcript is written in brain/<id>/.system_generated/logs/transcript_full.jsonl (its calls' arguments, its reasoning),
// before the event of each step. The words of each message pick what it plays:
//   bonjour (or anything else) — « Antigravity a reçu : <its words> »;  raisonnement — a reasoning (in the transcript), then a reply;
//   commande — `npm test` (run_command);  fichier — notes.txt written in the agent's folder (write_file, its text in the
//   transcript); « fichier dehors »: a file beside the folder;  question — ask_question, skipped, then a reply;
//   propose un plan — in `--mode plan`, its plan written in brain/<id>/plan_fake.md, then a reply saying so;
//   sous-agent — a subagent (invoke_subagent);  lent — a turn of 6 s;  modèle — says its model, effort and mode;
//   outil escouade <outil> — calls <outil> of Escouade's MCP server (config/mcp_config.json and its header);
//   erreur — a `result` in ERROR;  limite — its quota exhausted (a `result` in ERROR, in `agy`'s words);
//   capacité — the model's capacity exhausted;  compte — says who it runs as: « Compte : clé …<end> » with a Gemini API
//   key and `modelProvider: gemini` (ESC-271), else « Compte : Google de la machine »; a key `cle-refusee…` is refused.
// What Escouade says once the user answered: « The user allowed … » does again what was refused, « The user answered
// your questions: » says the answers, « The plan is approved » carries it out.
// `--conversation <id>` resumes a conversation kept in .gemini/antigravity-cli/conversations/fake/<id>.json (an unknown
// one: its error on stderr, exit 1). `models` lists the models of `agy models`. Each launch ({launch: {argv, cwd, home,
// geminiKey, autoUpdate, gitConfig, provider, deny}}) and message ({user}) goes to $FAKE_AGY_LOG when set.
// Without -p (nor models), it is its interface `agy` (fake-agy-tui.mjs, « Vue Antigravity »).
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const argv = process.argv.slice(2);
if (argv.includes('--version')) {
  console.log(process.env.FAKE_AGY_VERSION || '1.4.9');
  process.exit(0);
}
const home = process.env.USERPROFILE || process.env.HOME || process.cwd();
const logFile = process.env.FAKE_AGY_LOG;
const log = (entry) => logFile && fs.appendFileSync(logFile, JSON.stringify(entry) + '\n');
const settingsAtStart = (() => {
  try {
    return JSON.parse(fs.readFileSync(path.join(home, '.gemini', 'antigravity-cli', 'settings.json'), 'utf8'));
  } catch {
    return {};
  }
})();
const geminiKey = process.env.GEMINI_API_KEY || '';
const byKey = settingsAtStart.modelProvider === 'gemini' && geminiKey !== '';
log({
  launch: {
    argv,
    cwd: process.cwd(),
    home,
    geminiKey: process.env.GEMINI_API_KEY ?? null,
    autoUpdate: process.env.AGY_CLI_DISABLE_AUTO_UPDATE ?? null,
    gitConfig: process.env.GIT_CONFIG_GLOBAL ?? null,
    provider: settingsAtStart.modelProvider ?? null,
    deny: settingsAtStart.permissions?.deny ?? null,
  },
});
// Its interface (no `-p`) not signed in yet, as « Se connecter… » runs it (ESC-271): signed in, the file of
// $ESCOUADE_AGY_SIGNED_IN stands for the keychain's entry, then it waits until it is killed. Signed in already (that
// file there, or no such variable), its interface is fake-agy-tui.mjs, below (« Vue Antigravity », ESC-272): as `agy`.
const signedOut = !!process.env.ESCOUADE_AGY_SIGNED_IN && !fs.existsSync(process.env.ESCOUADE_AGY_SIGNED_IN);
if (signedOut && !argv.some((a) => a.startsWith('-p')) && argv[0] !== 'models') {
  console.log('You are currently not signed in. Opening your browser to sign in with Google…');
  setTimeout(() => {
    if (process.env.ESCOUADE_AGY_SIGNED_IN) fs.writeFileSync(process.env.ESCOUADE_AGY_SIGNED_IN, 'signed in');
    console.log('Signed in.');
  }, 500);
  setInterval(() => {}, 1 << 30);
  await new Promise(() => {});
}
if (argv[0] === 'models') {
  console.log('Fetching available models...');
  console.log(
    [
      'gemini-3.8-flash-high\tGemini 3.8 Flash (High)',
      'gemini-3.8-flash-medium\tGemini 3.8 Flash (Medium)',
      'gemini-3.8-flash-low\tGemini 3.8 Flash (Low)',
      'claude-sonnet-4-6\tClaude Sonnet 4.6 (Thinking)',
    ].join('\n'),
  );
  process.exit(0);
}
if (!argv.some((a) => a === '-p' || a.startsWith('-p=') || a === '--print')) {
  // Its interface (« Vue Antigravity »): fake-agy-tui.mjs, which ends the process itself.
  const { tui } = await import('./fake-agy-tui.mjs');
  tui(argv);
  // The rest is its `-p`'s: never reached.
  await new Promise(() => {});
}
const arg = (name) => {
  const at = argv.indexOf(name);
  return at >= 0 ? argv[at + 1] : undefined;
};
const model = arg('--model') ?? 'gemini-3.8-flash-medium';
const effort = arg('--effort') ?? null;
const mode = arg('--mode') ?? 'default';
const cwd = process.cwd();
const agyDir = path.join(home, '.gemini', 'antigravity-cli');
const store = path.join(agyDir, 'conversations', 'fake');
fs.mkdirSync(store, { recursive: true });

let id = arg('--conversation');
let conv;
if (id) {
  try {
    conv = JSON.parse(fs.readFileSync(path.join(store, `${id}.json`), 'utf8'));
  } catch {
    process.stderr.write(`Error: conversation ${id} not found\n`);
    process.exit(1);
  }
} else {
  id = crypto.randomUUID();
  conv = { id, steps: 0, said: [] };
}
const save = () => fs.writeFileSync(path.join(store, `${id}.json`), JSON.stringify(conv));
save();
const brain = path.join(agyDir, 'brain', id);
const logs = path.join(brain, '.system_generated', 'logs');
fs.mkdirSync(logs, { recursive: true });
// FAKE_AGY_LATE_MS: each line of the transcript written that long after the step it tells is over, as `agy` does at times.
const late = Number(process.env.FAKE_AGY_LATE_MS || 0);
const transcript = (entry) => {
  const write = () => fs.appendFileSync(path.join(logs, 'transcript_full.jsonl'), JSON.stringify({ status: 'DONE', ...entry }) + '\n');
  if (late > 0) setTimeout(write, late);
  else write();
};

const out = (event) => process.stdout.write(JSON.stringify(event) + '\n');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const usage = { input_tokens: 7000, output_tokens: 20, thinking_tokens: 5, cache_read_tokens: 0, total_tokens: 7025 };
const stepUpdate = (s) => out({ event: 'step_update', step_update: { conversation_id: id, ...s } });

out({
  event: 'init',
  conversation_id: id,
  init: {
    model,
    cwd,
    tools: ['run_command', 'write_to_file', 'ask_question', 'invoke_subagent', 'call_mcp_tool'],
    permission_mode: 'request-review',
  },
});

/** The rules of its settings, read again each time. */
function rules() {
  try {
    return JSON.parse(fs.readFileSync(path.join(agyDir, 'settings.json'), 'utf8')).permissions ?? {};
  } catch {
    return {};
  }
}
const norm = (p) => p.replace(/\\/g, '/').replace(/\/$/, '').toLowerCase();
/** Whether `rule` (`action(target)`) covers `action` on `target`: refusing to read refuses to write (`deny`), allowing to
 * write allows to read (`allow`). */
function covers(rule, action, target, list) {
  const m = rule.match(/^(\w+)\((.*)\)$/s);
  if (!m) return false;
  const [, a, t] = m;
  const implied = list === 'deny' ? a === 'read_file' && action === 'write_file' : a === 'write_file' && action === 'read_file';
  if (a !== action && !implied) return false;
  if (t === '*') return true;
  if (action === 'command') return target === t || target.startsWith(`${t} `);
  if (action === 'mcp') return t === target || (t.endsWith('/*') && target.startsWith(t.slice(0, -1)));
  const [p, r] = [norm(target), norm(t)];
  return p === r || p.startsWith(`${r}/`);
}
const inside = (p) => (norm(p) + '/').startsWith(norm(cwd) + '/');
function allowed(action, target) {
  const r = rules();
  if ((r.deny ?? []).some((x) => covers(x, action, target, 'deny'))) return false;
  if (action === 'write_file' && mode === 'accept-edits' && inside(target)) return true;
  if (action === 'read_file' && inside(target)) return true;
  // Its own conversation's files (the plan of the mode `plan`).
  if (action === 'write_file' && (norm(target) + '/').startsWith(norm(brain) + '/')) return true;
  return (r.allow ?? []).some((x) => covers(x, action, target, 'allow'));
}
const denied = (text) =>
  `permission check failed for unsandboxed "${text}": user denied permission to run command:\n${text}\nDo not attempt to circumvent this denial by rephrasing the command, using alternative tools/scripts (e.g. python, sh, curl), or accessing the same target resource. Proceed without performing this action.`;

/** A turn's state: its steps, what was refused. */
let turn = null;
const nextStep = () => conv.steps++;

/** A step of the model: its reasoning and calls in the transcript, its text in deltas. */
async function planner({ text, thinking, calls }) {
  const step = nextStep();
  transcript({
    step_index: step,
    source: 'MODEL',
    type: 'PLANNER_RESPONSE',
    ...(thinking ? { thinking } : {}),
    ...(calls ? { tool_calls: calls } : {}),
    ...(text ? { content: text } : {}),
  });
  if (text) {
    for (const piece of text.match(/.{1,12}/gsu) ?? []) {
      stepUpdate({ step_index: step, state: 'ACTIVE', step_type: 'agent_response', text_delta: piece });
      await sleep(5);
    }
    stepUpdate({ step_index: step, state: 'DONE', step_type: 'agent_response', text_delta: '\n', duration_seconds: 0.1, usage });
  } else {
    stepUpdate({ step_index: step, state: 'DONE', step_type: 'agent_response', duration_seconds: 0.1, usage });
  }
  turn.usage += 1;
  turn.response += text ?? '';
}

/** A tool's step: allowed, run; else refused. */
async function tool(name, args, run) {
  await planner({ calls: [{ name, args }] });
  const step = nextStep();
  const parameters = {
    ...(args.Path ? { Path: args.Path } : {}),
    ...(args.CommandLine ? { CommandLine: args.CommandLine } : {}),
    ...(args.ServerName ? { ServerName: args.ServerName, ToolName: args.ToolName, Arguments: args.Arguments } : {}),
  };
  stepUpdate({ step_index: step, state: 'ACTIVE', step_type: 'tool', tool_name: name, tool_info: { name, parameters } });
  const [action, target, display] =
    name === 'run_command'
      ? ['command', args.CommandLine, 'RunCommand']
      : name === 'call_mcp_tool'
        ? ['mcp', `${args.ServerName}/${args.ToolName}`, 'CallMcpTool']
        : ['write_file', args.Path, 'WriteFile'];
  if (!allowed(action, target)) {
    turn.denied.push({ action, display_name: display });
    const error = denied(target);
    transcript({
      step_index: step,
      source: 'MODEL',
      type: 'GENERIC',
      status: 'ERROR',
      error,
      content: `Encountered error in step execution: ${error}`,
    });
    if (action === 'write_file') {
      stepUpdate({
        step_index: step,
        state: 'DONE',
        step_type: 'tool',
        tool_name: name,
        duration_seconds: 0.03,
        tool_info: { name, parameters },
      });
    } else {
      stepUpdate({
        step_index: step,
        state: 'ERROR',
        step_type: 'tool',
        tool_name: name,
        duration_seconds: 0.03,
        tool_info: { name, parameters, error: { type: 'TOOL_ERROR', message: error } },
      });
    }
    return false;
  }
  const output = await run();
  transcript({ step_index: step, source: 'MODEL', type: 'GENERIC', content: output ?? '' });
  stepUpdate({
    step_index: step,
    state: 'DONE',
    step_type: 'tool',
    tool_name: name,
    duration_seconds: 0.05,
    tool_info: { name, parameters, ...(output ? { output } : {}) },
  });
  return true;
}

const runCommand = (cmd) =>
  tool(
    'run_command',
    { CommandLine: cmd, WaitMsBeforeAsync: 5000 },
    async () => `Command exited with code 0.\n\nCommand output:\n${cmd === 'npm test' ? 'tests ok' : 'ok'}\r\n`,
  );
const writeFile = (file, content) =>
  tool('write_file', { Path: file, Content: content }, async () => {
    fs.writeFileSync(file, content);
    return null;
  });

/** The text of the tool `tool` of Escouade's MCP server, by its entry in config/mcp_config.json. */
async function escouadeTool(name) {
  const config = JSON.parse(fs.readFileSync(path.join(home, '.gemini', 'config', 'mcp_config.json'), 'utf8'));
  const entry = config.mcpServers?.escouade;
  if (!entry) throw new Error('no escouade server');
  const base = { 'content-type': 'application/json', accept: 'application/json, text/event-stream', ...(entry.headers ?? {}) };
  let session = null;
  const call = async (method, params, rid) => {
    const headers = { ...base, ...(session ? { 'mcp-session-id': session } : {}) };
    const body = JSON.stringify(rid === undefined ? { jsonrpc: '2.0', method, params } : { jsonrpc: '2.0', id: rid, method, params });
    const r = await fetch(entry.serverUrl, { method: 'POST', headers, body });
    session = r.headers.get('mcp-session-id') ?? session;
    const text = await r.text();
    if (!r.ok) throw new Error(`${r.status}`);
    if (rid === undefined) return null;
    const answers = text.includes('data:')
      ? text
          .split('\n')
          .filter((l) => l.startsWith('data:'))
          .map((l) => JSON.parse(l.slice(5)))
      : [JSON.parse(text)];
    const answer = answers.find((a) => a.id === rid);
    if (answer?.error) throw new Error(answer.error.message);
    return answer?.result;
  };
  await call('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'agy', version: '0' } }, 1);
  await call('notifications/initialized', {});
  const result = await call('tools/call', { name, arguments: {} }, 2);
  return (result?.content ?? []).map((c) => c.text ?? '').join('');
}

async function play(text) {
  const own = text.replace(/<(instructions|escouade)>[\s\S]*?<\/\1>\n*/g, '').trim();
  const words = own.toLowerCase();
  if (words.startsWith('the user allowed')) {
    const cmd = own.match(/the command `([^`]+)`/)?.[1];
    const file = own.match(/writing (.+?)(?:, |: do it)/)?.[1];
    const mcp = own.match(/the tool (\S+) of the MCP server (\S+?)[,:]/);
    if (cmd) await runCommand(cmd);
    if (file) await writeFile(file, 'bonjour\n');
    if (mcp)
      await tool('call_mcp_tool', { ServerName: mcp[2], ToolName: mcp[1], Arguments: {} }, async () =>
        escouadeTool(mcp[1]).catch((e) => `error: ${e.message}`),
      );
    return planner({ text: 'Fait.' });
  }
  if (words.startsWith('the user answered your questions')) {
    const answer = own.match(/→ (.*)$/m)?.[1] ?? '';
    return planner({ text: `Réponse reçue : ${answer}` });
  }
  if (words.includes('the plan is approved')) return planner({ text: 'Plan mis en œuvre.' });
  const task = own.match(/<(tache|task)>\s*([\s\S]*?)\s*<\/\1>/)?.[2];
  if (task !== undefined) return planner({ text: `${(task.toLowerCase().match(/[a-z]+/g) ?? ['tache']).slice(0, 2).join('-')}-agy` });
  if (words.startsWith('outil escouade ')) {
    const name = own.slice('outil escouade '.length).trim();
    let said = '';
    const ok = await tool('call_mcp_tool', { ServerName: 'escouade', ToolName: name, Arguments: {} }, async () => {
      said = await escouadeTool(name).catch((e) => `refusé : ${e.message}`);
      return said;
    });
    return ok ? planner({ text: `Escouade a répondu : ${said}` }) : undefined;
  }
  if (byKey && geminiKey.startsWith('cle-refusee')) {
    turn.error = 'API key not valid. Please pass a valid API key.';
    return;
  }
  if (words.includes('erreur')) {
    turn.error = 'model "inconnu" not found';
    return;
  }
  if (words.includes('limite')) {
    turn.error = 'You have exhausted your quota on this model.';
    return;
  }
  if (words.includes('capacité')) {
    turn.error = 'MODEL_CAPACITY_EXHAUSTED: No capacity available for model, retry in 30s';
    return;
  }
  if (words.includes('compte')) {
    return planner({ text: byKey ? `Compte : clé …${geminiKey.slice(-4)}` : 'Compte : Google de la machine' });
  }
  if (words.includes('raisonnement')) {
    await planner({ thinking: 'Je réfléchis à la question posée.' });
    return planner({ text: 'Voici ma réponse réfléchie.' });
  }
  if (words.includes('commande')) {
    if (await runCommand('npm test')) await planner({ text: 'Tests passés.' });
    return;
  }
  if (words.includes('fichier')) {
    const file = words.includes('dehors') ? path.join(path.dirname(cwd), 'dehors.txt') : path.join(cwd, 'notes.txt');
    if (await writeFile(file, 'bonjour\n')) await planner({ text: 'Fichier écrit.' });
    return;
  }
  if (words.includes('question')) {
    await planner({
      calls: [
        {
          name: 'ask_question',
          args: { questions: [{ question: 'Bleu ou rouge ?', options: ['(Recommended) Bleu', 'Rouge'], is_multi_select: false }] },
        },
      ],
    });
    const step = nextStep();
    transcript({ step_index: step, source: 'MODEL', type: 'GENERIC', content: 'A1: User Skipped' });
    stepUpdate({ step_index: step, state: 'DONE', step_type: 'unknown', duration_seconds: 0.1 });
    return;
  }
  if (words.includes('propose un plan')) {
    if (mode !== 'plan') return planner({ text: 'Je ferais : écrire notes.txt.' });
    const file = path.join(brain, 'plan_fake.md');
    await tool('write_file', { Path: file, Content: '# Plan\n\n1. Écrire notes.txt' }, async () => {
      fs.writeFileSync(file, '# Plan\n\n1. Écrire notes.txt');
      return null;
    });
    return planner({ text: `J'ai rédigé le plan dans [plan_fake.md](file:///${file.replace(/\\/g, '/')}).` });
  }
  if (words.includes('sous-agent')) {
    await planner({ calls: [{ name: 'invoke_subagent', args: { Role: 'Saluer', TypeName: 'self', Prompt: 'Dis bonjour.' } }] });
    const step = nextStep();
    stepUpdate({
      step_index: step,
      state: 'ACTIVE',
      step_type: 'tool',
      tool_name: 'invoke_subagent',
      tool_info: { name: 'invoke_subagent', parameters: { Role: 'Saluer', TypeName: 'self' } },
    });
    transcript({ step_index: step, source: 'MODEL', type: 'GENERIC', content: 'started' });
    stepUpdate({
      step_index: step,
      state: 'DONE',
      step_type: 'subagent',
      tool_name: 'invoke_subagent',
      subagent_info: {
        subagents: [{ type_name: 'self', role: 'Saluer', initial_prompt: 'Dis bonjour.', conversation_id: crypto.randomUUID() }],
      },
    });
    return planner({ text: 'Le sous-agent a dit bonjour.' });
  }
  if (words.includes('lent')) {
    await planner({ text: 'Je commence un long travail…' });
    await sleep(6000);
    return planner({ text: 'Fini.' });
  }
  if (words.includes('modèle')) return planner({ text: `Modèle : ${model}, effort : ${effort ?? 'aucun'}, mode : ${mode}` });
  if (conv.said.length > 0 && words.includes('souviens')) return planner({ text: `Tu as dit : ${conv.said.at(-2)}` });
  return planner({ text: `Antigravity a reçu : ${own}` });
}

const queue = [];
let running = false;
let closed = false;
async function drain() {
  if (running) return;
  running = true;
  while (queue.length) {
    const text = queue.shift();
    conv.said.push(text.replace(/<(instructions|escouade)>[\s\S]*?<\/\1>\n*/g, '').trim());
    turn = { usage: 0, response: '', denied: [], error: null };
    const step = nextStep();
    transcript({ step_index: step, source: 'USER_EXPLICIT', type: 'USER_INPUT', content: `<USER_REQUEST>\n${text}\n</USER_REQUEST>` });
    stepUpdate({ step_index: step, state: 'DONE', step_type: 'user_input' });
    await play(text);
    save();
    const result = {
      conversation_id: turn.error ? '' : id,
      status: turn.error ? 'ERROR' : 'SUCCESS',
      response: turn.response,
      duration_seconds: 0.2,
      num_turns: conv.said.length,
      usage: { ...usage, input_tokens: usage.input_tokens * turn.usage, output_tokens: usage.output_tokens * turn.usage },
    };
    if (turn.error) result.error = turn.error;
    if (turn.denied.length) result.denied_actions = turn.denied;
    out({ event: 'result', result });
  }
  running = false;
  if (closed) process.exit(0);
}

let buf = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', (d) => {
  buf += d;
  let at;
  while ((at = buf.indexOf('\n')) >= 0) {
    const line = buf.slice(0, at).trim();
    buf = buf.slice(at + 1);
    if (!line) continue;
    let msg;
    try {
      msg = JSON.parse(line);
    } catch {
      process.stderr.write('invalid input line\n');
      continue;
    }
    if (msg.event !== 'user') {
      process.stderr.write(`warning: ignoring event ${msg.event}\n`);
      continue;
    }
    const content = msg.message?.content;
    const text = typeof content === 'string' ? content : (content ?? []).map((b) => b.text ?? '').join('\n');
    log({ user: text });
    queue.push(text);
    drain();
  }
});
process.stdin.on('end', () => {
  closed = true;
  if (!running) process.exit(0);
});
