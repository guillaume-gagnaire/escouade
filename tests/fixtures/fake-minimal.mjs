#!/usr/bin/env node
// The harness of the demonstration plugin (`dev.escouade.minimal`, tests/fixtures/plugins/minimal): the minimal fake
// engine of the conformance bench (`engine/minimal/`) as a program of its own, which the core launches from the
// description the plugin gives (INT-9). One JSON a line on its input (`{"say": text}`, `{"answer": {"request", "value"}}`,
// `{"interrupt": true}`), one frame a line on its output, in the protocol `engine/minimal` lists. Its message picks the
// scenario as the in-process fake's does, with three more: « plantage » (its frames make the plugin crash once, mid-turn),
// « plantage-toujours » (each time), « flux <n> » (n lines of text at once, each stamped, to measure), « rythme <n> » (the
// same, one every 5 ms).
//
// The plugin names it `minimal-harness` first among its arguments (it was a program of `escouade-plugins` until
// ESC-355): that word is passed over. `--session <id>` resumes a session; `--once <prompt>` answers a one-shot question
// and exits.
import process from 'node:process';
import path from 'node:path';

const args = process.argv.slice(2).filter((a, i) => !(i === 0 && a === 'minimal-harness'));
const value = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};

if (value('--once') !== undefined) {
  process.stdout.write(`Réponse courte : ${value('--once')}\n`);
  process.exit(0);
}

const session = value('--session') ?? `min-${process.hrtime.bigint().toString(16)}`;
const cwd = process.cwd();
let n = 0;

// What came on the input, in order; who waits for the next of it.
const inputs = [];
let waker = null;
let closed = false;

function push(input) {
  inputs.push(input);
  if (waker) {
    const w = waker;
    waker = null;
    w();
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
    let v;
    try {
      v = JSON.parse(line);
    } catch {
      continue;
    }
    if (typeof v.say === 'string') push({ say: v.say });
    else if (v.interrupt === true) push({ interrupt: true });
    else if (typeof v.answer?.request === 'string') push({ answer: v.answer.request, value: v.answer.value });
  }
});
process.stdin.on('end', () => {
  closed = true;
  if (waker) {
    const w = waker;
    waker = null;
    w();
  }
});

/** The next input, or null once the input closed; `undefined` past `ms`. */
function next(ms) {
  if (inputs.length) return Promise.resolve(inputs.shift());
  if (closed) return Promise.resolve(null);
  return new Promise((resolve) => {
    const timer =
      ms === undefined
        ? null
        : setTimeout(() => {
            waker = null;
            resolve(undefined);
          }, ms);
    waker = () => {
      if (timer) clearTimeout(timer);
      resolve(inputs.length ? inputs.shift() : null);
    };
  });
}

const out = (frame) => process.stdout.write(JSON.stringify(frame) + '\n');
const say = (text) => out({ type: 'say', text });
const id = (prefix) => `${prefix}-${++n}`;
const micros = () => String(Math.floor(performance.timeOrigin * 1000 + performance.now() * 1000));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

class Cut {
  constructor(why) {
    this.why = why;
  }
}

/** The answer to `request` within `ms`: its value, null past the time; a Cut for an interruption or the input's end. */
async function wait(request, ms) {
  const deadline = Date.now() + ms;
  for (;;) {
    const left = deadline - Date.now();
    if (left <= 0) return null;
    const input = await next(left);
    if (input === undefined) return null;
    if (input === null) throw new Cut('closed');
    if (input.interrupt) throw new Cut('interrupted');
    if (input.answer !== undefined && input.answer === request) return input.value ?? null;
  }
}

async function turn(text) {
  out({ type: 'start', session });
  if (text.includes('raisonnement')) {
    out({ type: 'thinking', text: 'Deux pistes ; la seconde est la bonne.' });
    say('Réponse réfléchie.');
  } else if (text.includes('commande')) {
    const call = id('call');
    const request = id('ask');
    out({ type: 'call', id: call, tool: 'shell', input: { command: 'rm -rf build' } });
    out({ type: 'ask', request, call });
    const answer = (await wait(request, 60_000)) ?? {};
    if (answer.allow === true) {
      out({ type: 'done', call, output: 'build supprimé' });
      say('Commande exécutée.');
    } else {
      out({ type: 'done', call, output: 'refusée', error: true });
      say('Compris.');
    }
  } else if (text.includes('fichier')) {
    const call = id('call');
    out({ type: 'call', id: call, tool: 'patch', input: { path: 'src/app.ts', diff: '-const a = 1;\n+const a = 2;' } });
    out({ type: 'done', call, output: '1 fichier modifié' });
    say('Fichier modifié.');
  } else if (text.includes('question')) {
    const call = id('call');
    const request = id('question');
    out({ type: 'question', request, call, text: 'Quelle base de données ?', choices: ['PostgreSQL', 'SQLite'] });
    const answers = (await wait(request, 60_000)) ?? {};
    const chosen = typeof answers === 'object' ? Object.values(answers).filter((v) => typeof v === 'string') : [];
    say(`Choix retenu : ${chosen.join(', ')}`);
  } else if (text.includes('lent')) {
    say('Je commence…');
    await wait('', 30_000);
  } else if (text.includes('sous-agent')) {
    const call = id('call');
    out({ type: 'call', id: call, tool: 'agent', input: { task: 'Chercher la cause du bug' } });
    out({ type: 'say', text: 'Cause trouvée : le filtre refuse les PDF.', parent: call });
    out({ type: 'done', call, output: 'Le filtre refuse les PDF.' });
    say('Rapport reçu.');
  } else if (text.includes('plantage')) {
    // A tool under way when the plugin crashes: the core closes it at the end of the turn.
    const call = id('call');
    out({ type: 'call', id: call, tool: 'shell', input: { command: 'make' } });
    say('Avant le plantage.');
    out({ type: 'crash', flag: path.join(cwd, 'plantage.flag'), always: text.includes('toujours') });
    // Long enough for the plugin to start again: its frames wait in the core meanwhile.
    await sleep(300);
    say('Après le plantage.');
  } else if (/^rythme \d+$/.test(text.trim())) {
    // At a pace: a line every 5 ms, as a model writes.
    const count = Number(text.trim().slice('rythme '.length));
    for (let i = 0; i < count; i++) {
      say(micros());
      await sleep(5);
    }
  } else if (/^flux \d+$/.test(text.trim())) {
    const count = Number(text.trim().slice('flux '.length));
    for (let i = 0; i < count; i++) say(micros());
  } else {
    say(`Bonjour, tu as dit : ${text}`);
  }
}

for (;;) {
  const input = await next();
  if (input === null) break;
  if (input.say === undefined) continue;
  try {
    await turn(input.say);
    out({ type: 'end', usage: { input: 120, output: 30 } });
  } catch (e) {
    if (!(e instanceof Cut)) throw e;
    if (e.why === 'closed') break;
    out({ type: 'end', interrupted: true });
  }
}
process.exit(0);
