// The interface of the fake `agy` (« Vue Antigravity »): `agy [--conversation <id>] [--model <m>] [--effort <e>]
// [--mode <m>] [--add-dir <dir>]`, in a pseudo-terminal, as Antigravity CLI 1.4.9's own was tried (docs/TODO-1.8.md,
// point 0, « Antigravity CLI », ESC-269 and ESC-272). Its home is $USERPROFILE (or $HOME), as the fake `agy -p`'s.
//
// - Each launch is added to <home>/fake-tui-launches.jsonl ({argv, cwd, home, mcp, hooks, settings, onboarding,
//   hideAccount, geminiKey}): what Escouade gave it in its variables and its home.
// - Its first screens: the choice of its colours when <home>/.gemini/antigravity-cli/cache/onboarding.json is not
//   complete, the trust of its folder when the `trustedWorkspaces` of its settings do not have it ; each waits for
//   Enter, which writes what it chose, as `agy` does.
// - Its conversation is `--conversation <id>` when its store has it (the fake `agy -p`'s,
//   .gemini/antigravity-cli/conversations/fake/<id>.json); else it says `warning: conversation "<id>" not found` and
//   goes on in a new one, as `agy` does. It shows `fake-agy-tui <id>`.
// - Its transcript is the conversation's (brain/<id>/.system_generated/logs/transcript_full.jsonl), a step a line,
//   as `agy` writes it (tests/fixtures/agy/tui-transcript-*.jsonl). Each line it reads (pasted text, then Enter) is a
//   prompt: `USER_INPUT`, then the answer « Fini : <prompt> ». "commande" runs `npm test` first (an answer with its call,
//   then its run); "permission" asks its permission first and waits for Enter; "question" asks a question
//   (ask_question) and waits for Enter; "lent" writes « Je commence… » and waits for Escape, which ends the turn with
//   what it wrote (no hook). A line read during a turn waits for its end. Ctrl+C twice out of a turn ends the process.
// - Its hooks (<home>/.gemini/config/hooks.json) run as `agy` runs them, in a shell, from their file's folder, their
//   payload on their input: `SessionStart` with the first message of a new conversation, `PreToolUse` and
//   `PostToolUse` around its tool, `Stop` at the end of each turn. Each one run is added to <home>/fake-tui-hooks.jsonl
//   ({event, command}).

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';

const read = (file) => {
  try {
    return fs.readFileSync(file, 'utf8');
  } catch {
    return null;
  }
};
const readJson = (file) => {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch {
    return null;
  }
};

export function tui(argv) {
  const home = process.env.USERPROFILE || process.env.HOME;
  const cwd = process.cwd();
  const gemini = path.join(home, '.gemini');
  const agyDir = path.join(gemini, 'antigravity-cli');
  const settingsFile = path.join(agyDir, 'settings.json');
  const onboardingFile = path.join(agyDir, 'cache', 'onboarding.json');
  const hooksFile = path.join(gemini, 'config', 'hooks.json');
  fs.mkdirSync(home, { recursive: true });
  fs.appendFileSync(
    path.join(home, 'fake-tui-launches.jsonl'),
    JSON.stringify({
      argv,
      cwd,
      home,
      autoUpdate: process.env.AGY_CLI_DISABLE_AUTO_UPDATE ?? null,
      mcp: readJson(path.join(gemini, 'config', 'mcp_config.json')),
      hooks: readJson(hooksFile),
      settings: readJson(settingsFile),
      onboarding: readJson(onboardingFile),
      hideAccount: process.env.AGY_CLI_HIDE_ACCOUNT_INFO ?? null,
      geminiKey: process.env.GEMINI_API_KEY ?? null,
    }) + '\n',
  );
  const flag = (name) => {
    const i = argv.indexOf(name);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const store = path.join(agyDir, 'conversations', 'fake');
  fs.mkdirSync(store, { recursive: true });
  const asked = flag('--conversation');
  let notFound = null;
  if (asked && !fs.existsSync(path.join(store, `${asked}.json`))) notFound = asked;
  const id = asked && !notFound ? asked : crypto.randomUUID();
  const kept = path.join(store, `${id}.json`);
  const conv = readJson(kept) ?? { id, steps: 0, said: [] };
  const fresh = !readJson(kept);
  const save = () => fs.writeFileSync(kept, JSON.stringify(conv));
  const logs = path.join(agyDir, 'brain', id, '.system_generated', 'logs');
  const transcriptPath = path.join(logs, 'transcript_full.jsonl');
  const step = (entry) => {
    fs.mkdirSync(logs, { recursive: true });
    const index = conv.steps++;
    fs.appendFileSync(
      transcriptPath,
      JSON.stringify({ step_index: index, status: 'DONE', created_at: new Date().toISOString(), ...entry }) + '\n',
    );
    save();
    return index;
  };
  const hooks = readJson(hooksFile) ?? {};
  const runHooks = (event, more) =>
    Promise.all(
      Object.values(hooks)
        .filter((h) => h.enabled !== false)
        .flatMap((h) => (h[event] ?? []).flatMap((g) => g.hooks ?? [g]))
        .map(
          (h) =>
            new Promise((resolve) => {
              fs.appendFileSync(path.join(home, 'fake-tui-hooks.jsonl'), JSON.stringify({ event, command: h.command }) + '\n');
              const child = spawn(h.command, { shell: true, cwd: path.dirname(hooksFile), stdio: ['pipe', 'ignore', 'ignore'] });
              child.on('error', resolve);
              child.on('exit', resolve);
              child.stdin.end(
                JSON.stringify({
                  conversationId: id,
                  workspacePaths: [cwd.replace(/\\/g, '/')],
                  transcriptPath: transcriptPath.replace(/\\/g, '/'),
                  artifactDirectoryPath: path.join(agyDir, 'brain', id).replace(/\\/g, '/'),
                  modelName: flag('--model') ?? 'gemini-3.8-flash-high',
                  ...more,
                }),
              );
            }),
        ),
    );

  // Raw before its first screen (ESC-125): a key typed as soon as that shows would else reach a terminal still cooked.
  if (process.stdin.isTTY) process.stdin.setRawMode(true);
  process.stdin.setEncoding('utf8');

  // Its first screens, each passed by Enter.
  const screens = [];
  if (readJson(onboardingFile)?.onboardingComplete !== true) screens.push('onboarding');
  const settings = readJson(settingsFile) ?? {};
  if (!(settings.trustedWorkspaces ?? []).includes(cwd)) screens.push('trust');
  const showScreen = () => {
    if (screens[0] === 'onboarding') process.stdout.write('Welcome to Antigravity CLI!\r\nChoose your color scheme:\r\n');
    else if (screens[0] === 'trust') process.stdout.write('Do you trust the contents of this project?\r\n> Yes, I trust this folder\r\n');
  };
  const passScreen = () => {
    const done = screens.shift();
    if (done === 'onboarding') {
      fs.mkdirSync(path.dirname(onboardingFile), { recursive: true });
      fs.writeFileSync(onboardingFile, JSON.stringify({ onboardingComplete: true }));
    } else if (done === 'trust') {
      const s = readJson(settingsFile) ?? {};
      s.trustedWorkspaces = [...(s.trustedWorkspaces ?? []), cwd];
      fs.mkdirSync(path.dirname(settingsFile), { recursive: true });
      fs.writeFileSync(settingsFile, JSON.stringify(s));
    }
    if (screens.length) showScreen();
    else ready();
  };
  const ready = () => {
    if (notFound) process.stdout.write(`warning: conversation "${notFound}" not found\r\n`);
    process.stdout.write(`\x1b[?2004hfake-agy-tui ${id}\r\n`);
  };
  if (screens.length) showScreen();
  else ready();

  let waiting = null;
  let queue = Promise.resolve();
  let ctrlC = 0;
  let started = !fresh;
  const wait = (kind) => new Promise((resolve) => (waiting = { resolve, kind }));
  const turn = async (prompt) => {
    if (!started) {
      started = true;
      await runHooks('SessionStart', {});
    }
    step({
      source: 'USER_EXPLICIT',
      type: 'USER_INPUT',
      content: `<USER_REQUEST>\n${prompt}\n</USER_REQUEST>\n<ADDITIONAL_METADATA>\nThe current local time is: now.\n</ADDITIONAL_METADATA>`,
    });
    conv.said.push(prompt);
    save();
    process.stdout.write(`fake-turn ${prompt}\r\n`);
    const tool = async (name, args, output) => {
      const at = step({ source: 'MODEL', type: 'PLANNER_RESPONSE', input_tokens: 7000, output_tokens: 30, tool_calls: [{ name, args }] });
      await runHooks('PreToolUse', { stepIdx: at + 1, toolCall: { name, args } });
      return async () => {
        step({ source: 'MODEL', type: 'GENERIC', content: output });
        await runHooks('PostToolUse', { stepIdx: at + 1, toolCall: { name, args }, error: '' });
      };
    };
    if (prompt.includes('commande') || prompt.includes('permission')) {
      const command = prompt.includes('permission') ? 'node --version' : 'npm test';
      const done = await tool(
        'run_command',
        { CommandLine: command, WaitMsBeforeAsync: 5000 },
        `Created At: now\nCompleted At: now\nCommand exited with code 0.\n\nCommand output:\nok\r\n`,
      );
      if (prompt.includes('permission')) {
        process.stdout.write('Requesting permission for:\r\n   node --version\r\nRun this command?\r\n');
        await wait('permission');
      }
      await done();
    }
    if (prompt.includes('question')) {
      const done = await tool(
        'ask_question',
        { questions: [{ question: 'Bleu ou rouge ?', options: ['Bleu', 'Rouge'], is_multi_select: false }] },
        'A1: Bleu',
      );
      process.stdout.write('Question 1/1: Bleu ou rouge ?\r\n');
      await wait('permission');
      await done();
    }
    if (prompt.includes('lent')) {
      process.stdout.write('esc to cancel\r\n');
      await wait('turn');
      // Escape: what it wrote is its answer; no hook.
      step({ source: 'MODEL', type: 'PLANNER_RESPONSE', content: 'Je commence un long travail…' });
      return;
    }
    step({ source: 'MODEL', type: 'PLANNER_RESPONSE', input_tokens: 7000, output_tokens: 10, content: `Fini : ${prompt}` });
    await runHooks('Stop', { executionNum: 0, terminationReason: 'NO_TOOL_CALL', error: '', fullyIdle: true });
  };

  let line = '';
  let pasting = false;
  const draw = () => process.stdout.write(`\r\x1b[2K> ${line}`);
  process.stdin.on('data', (chunk) => {
    for (const _ of chunk.matchAll(/\x03/g)) {
      if (waiting) continue;
      if (++ctrlC >= 2) process.exit(0);
      process.stdout.write('\r\npress ctrl+c again to exit\r\n');
    }
    chunk = chunk.replaceAll('\x03', '');
    for (const part of chunk.split(/(\x1b\[20[01]~|\r)/)) {
      if (part === '\x1b[200~') pasting = true;
      else if (part === '\x1b[201~') {
        pasting = false;
        draw();
      } else if (part === '') continue;
      else if (part === '\x1b' && !pasting) {
        if (waiting?.kind === 'turn') {
          const w = waiting;
          waiting = null;
          w.resolve();
        }
      } else if (part === '\r') {
        if (screens.length) passScreen();
        else if (waiting?.kind === 'permission') {
          const w = waiting;
          waiting = null;
          w.resolve();
        } else if (line.trim() && !waiting) {
          const prompt = line.trim();
          line = '';
          ctrlC = 0;
          draw();
          // A message typed during a turn waits for its end.
          queue = queue.then(() => turn(prompt));
        }
      } else {
        for (const c of part) {
          if (c === '\x7f' || c === '\b') line = line.slice(0, -1);
          else if (c === '\x15') line = '';
          else line += c;
        }
        if (!pasting) draw();
      }
    }
  });
}
