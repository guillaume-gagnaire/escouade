#!/usr/bin/env node
// Test double of the `claude` CLI in `--input-format/--output-format stream-json` mode.
// It replays the frame shapes documented in docs/PROTOCOL.md. The user message text picks the
// scenario: "question", "permission", "edit", "slow", "crash", "grandchild"; anything else is a
// plain reply. A `--resume=missing…` session fails like an unknown session does, forked or not;
// with `--fork-session`, the resumed conversation goes on under a new session id, and a
// `--resume-session-at=missing…` entry fails as one Claude Code cannot find does. Each assistant
// message carries the uuid of its entry in the session (`entry-<pid>-<n>`).
// Every launch appends {argv, cwd, proxy, tls, configDir} (its HTTPS_PROXY,
// NODE_TLS_REJECT_UNAUTHORIZED and CLAUDE_CONFIG_DIR, null when unset) to $FAKE_CLAUDE_LOG, by
// default <tmp>/fake-claude-<cwd with non-alphanumerics replaced by _>.jsonl, and every user message
// read on stdin to <log>.stdin.jsonl.
// With CLAUDE_CONFIG_DIR (a Claude account of its own), it keeps its sessions where Claude Code
// does, in <dir>/projects/<cwd with non-alphanumerics replaced by ->/<session id>.jsonl (each user
// message and assistant entry on a line), and a --resume of a session absent from every
// <dir>/projects/* folder fails as an unknown one does; while a <dir>/fake-limit file is there, every
// turn is stopped by the usage limit (as "limite") and its quota windows say 100 % (rate_limit_event,
// get_usage). While a <dir>/fake-error file is there, every turn fails with an error of the API
// ("Invalid API key", as an account the user is not signed in to gets), which is no usage limit.
// A user message with [ferme-lentement] makes the process take 800 ms to end once its input is
// closed, and write {"type":"closed"} to its session just before it does (a resume that copies
// the session before the process is gone misses that line). Without CLAUDE_CONFIG_DIR, none of this.
// What Escouade tells it is read in French or in English (« Langue des textes rédigés par Claude »):
// each scenario below is recognized in both, its answers stay the same.
// Started with --append-system-prompt (a ticket's protocol; not what a copy of an agent is told of
// its new folder, « Cette conversation a été copiée… », “This conversation was copied…”), it plays the ticket's agent: it writes
// <key>.txt ("Boucle n") in its folder and ends each turn with an ```escouade report (criteria and
// "avancement"). The ticket's title, in the protocol, steers it: [ok] every criterion met at once,
// [jamais] none ever, [sans-bilan] no report, [lent] a turn that lasts 30 s, [recette] a launch
// recipe (its process on the second port of its block), [question] a question first (the turn goes
// on once it is answered), [fin-d-abord] an interrupted turn's end sent before the answer to the
// interrupt, [commite] its work committed by itself (every file of its folder, a copied .env forced
// in), [retire-env] then .env taken out of git again in a second commit (still in the branch's
// history), [tenace] a process that lasts 5 s once its input is closed, [rien] no file written (nothing
// to merge); by default criterion n is met from loop n on.
// Asked to prepare a test launch (« Prépare le lancement… Ports réservés : <base> », “Prepare the test
// launch… Ports reserved: <base>”), any agent answers with a recipe whose process listens on <base + 1>.
// In one-shot mode, asked for a ticket's commit message, it answers `feat: travail du faux claude
// [<KEY>]` (`feat: work of the fake claude (en) [<KEY>]` when asked in English), or a sentence out of form when the ticket's title says [message-libre]; with [sourd]
// in its system prompt it never reads its input and answers nothing for 20 s. Asked for a project's
// worktree commands (<worktrees>), it suggests a setup (one of whose folders leaves the project, one
// that holds a line break) and a teardown, or only refused ones in a folder named "all-refused". Asked
// for a project's launch commands (<lancement>, <launch>), it suggests five (two of whose folders are no folders
// of the project, one that holds a line break), only refused ones in a folder named "all-refused", or
// nothing readable in a folder named "unreadable". Asked for a direct commit's message (<fichiers>, <files>),
// it names the latest commit subject (<sujets-recents>, <recent-subjects>) and the files of the diff it read.
// Asked to name a task (<tache>, <task>), it answers with its first two words. Asked in English (its
// question and its role both), a name ends with "-en", a direct commit's message with " (en)"; with
// a question and a role that disagree, "mixed" says so in the same places.

import { execFileSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import readline from 'node:readline';

const argv = process.argv.slice(2);
const logFile = process.env.FAKE_CLAUDE_LOG || path.join(os.tmpdir(), `fake-claude-${process.cwd().replace(/[^a-zA-Z0-9]/g, '_')}.jsonl`);
fs.appendFileSync(
  logFile,
  JSON.stringify({
    argv,
    cwd: process.cwd(),
    proxy: process.env.HTTPS_PROXY ?? null,
    tls: process.env.NODE_TLS_REJECT_UNAUTHORIZED ?? null,
    configDir: process.env.CLAUDE_CONFIG_DIR ?? null,
  }) + '\n',
);

// The account's own folder, if it has one (set empty: as if it were not).
const configDir = process.env.CLAUDE_CONFIG_DIR || null;
const sessionFile = (id) => path.join(configDir, 'projects', process.cwd().replace(/[^a-zA-Z0-9]/g, '-'), `${id}.jsonl`);
// Claude Code finds a session to resume in any project folder of the account (a copy made in a
// worktree of its own resumes the original's from another one).
const sessionKept = (id) => {
  const projects = path.join(configDir, 'projects');
  const folders = fs.existsSync(projects) ? fs.readdirSync(projects) : [];
  return folders.some((f) => fs.existsSync(path.join(projects, f, `${id}.jsonl`)));
};
// Out of quota, for as long as the file is there.
const limited = () => configDir !== null && fs.existsSync(path.join(configDir, 'fake-limit'));
// Failing with an error of the API, for as long as the file is there.
const failing = () => configDir !== null && fs.existsSync(path.join(configDir, 'fake-error'));

// One-shot mode (`-p --output-format json`), used by the app to name agents.
if (argv.includes('-p') && argv.some((a) => a.includes('[sourd]'))) {
  setTimeout(() => {}, 20_000);
} else if (argv.includes('-p')) {
  let input = '';
  // The role it is given (every one starts with « Tu » in French, "You" in English).
  const role = argv[argv.indexOf('--system-prompt') + 1] ?? '';
  // The language a question is asked in, as its frame (`english`) and its role say it: "mixed" when
  // they disagree. The answers that are written text (an agent's name, a commit message) say it
  // when it is not French, for a test to see which one Escouade asked in.
  const tongue = (english) => (/^You /.test(role) === english ? (english ? 'en' : 'fr') : 'mixed');
  process.stdin.on('data', (d) => (input += d));
  process.stdin.on('end', () => {
    // A project's worktree commands: a setup of two that stand, one whose folder leaves the project and
    // one on two lines, and a teardown. Only such ones on two lines or padded with blanks when the
    // project's folder is named "all-refused".
    if (input.includes('<worktrees>')) {
      const multiline = { commande: 'npm run gen\nrm -rf ~', dossier: '' };
      const steps = process.cwd().includes('all-refused')
        ? { preparation: [multiline], demontage: [{ commande: `docker compose down${' '.repeat(40)}; rm -rf ~`, dossier: '' }] }
        : {
            preparation: [
              { commande: 'npm ci', dossier: '' },
              { commande: 'npm run gen', dossier: 'src' },
              { commande: 'rm -rf /', dossier: '../dehors' },
              multiline,
            ],
            demontage: [{ commande: 'docker compose down', dossier: '' }],
          };
      const result = `Voici les commandes.\n\n\`\`\`json\n${JSON.stringify(steps)}\n\`\`\``;
      process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result }));
      return;
    }
    // A project's launch commands: two that stand, one whose folder leaves the project, one whose
    // folder is not there and one on two lines. Only such ones when the project's folder is named
    // "all-refused", nothing readable when it is named "unreadable".
    if (input.includes('<lancement>') || input.includes('<launch>')) {
      const multiline = { nom: 'Deux lignes', commande: 'npm start\nrm -rf ~' };
      const commands = process.cwd().includes('all-refused')
        ? { commandes: [multiline, { nom: 'Large', commande: `npm start${' '.repeat(40)}; rm -rf ~` }] }
        : {
            commandes: [
              { nom: 'Front', commande: 'npm run dev', dossier: 'src' },
              { nom: 'API', commande: 'cargo run', dossier: '' },
              { nom: 'Piège', commande: 'rm -rf /', dossier: '../dehors' },
              { nom: 'Fantôme', commande: 'npm start', dossier: 'absent' },
              multiline,
            ],
          };
      const result = process.cwd().includes('unreadable')
        ? 'Je ne vois rien à lancer ici.'
        : `Voici les commandes.\n\n\`\`\`json\n${JSON.stringify(commands)}\n\`\`\``;
      process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result }));
      return;
    }
    // A direct commit's message: it says the latest subject it read and the files whose diff it got.
    if (input.includes('<fichiers>') || input.includes('<files>')) {
      const latest = input.match(/<(?:sujets-recents|recent-subjects)>\n([^\n]*)/)?.[1] ?? 'aucun';
      const diffed = [...input.matchAll(/^\+\+\+ b\/(.+)$/gm)].map((m) => m[1]);
      const asked = tongue(input.includes('<files>'));
      const result = `feat: proposé par le faux claude${asked === 'fr' ? '' : ` (${asked})`}\n\nD'après « ${latest} ». Diff de : ${diffed.join(', ')}.`;
      process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result }));
      return;
    }
    // A ticket's commit message.
    const ticket = input.match(/<ticket>\s*([A-Z]+-\d+)/)?.[1];
    if (ticket) {
      const asked = tongue(input.startsWith('Write the commit message'));
      const work = asked === 'fr' ? 'travail du faux claude' : `work of the fake claude (${asked})`;
      const result = input.includes('[message-libre]') ? 'Voici le message : travail fait' : `feat: ${work} [${ticket}]`;
      process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result }));
      return;
    }
    const [, frame, task = ''] = input.match(/<(tache|task)>\s*([\s\S]*?)\s*<\/\1>/) ?? [];
    const words = task.toLowerCase().match(/[a-z]+/g) ?? ['tache'];
    const asked = tongue(frame === 'task');
    const named = `${words.slice(0, 2).join('-')}-fake${asked === 'fr' ? '' : `-${asked}`}`;
    process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result: named }));
  });
} else {
  startSession();
}

function startSession() {
  const resume = argv.find((a) => a.startsWith('--resume='))?.slice('--resume='.length);
  // A session of another account (or none) is not in this one's folder.
  if (resume?.startsWith('missing') || (resume && configDir && !sessionKept(resume))) {
    process.stderr.write(`No conversation found with session ID: ${resume}\n`);
    process.exit(1);
  }
  const at = argv.find((a) => a.startsWith('--resume-session-at='))?.slice('--resume-session-at='.length);
  if (at?.startsWith('missing')) {
    process.stderr.write(`No message found with message.uuid of: ${at}\n`);
    process.exit(1);
  }
  // Forked, as Claude Code does it: the session resumed is left as it is, a new one goes on.
  const sessionId = resume && !argv.includes('--fork-session') ? resume : `sess-${process.pid}`;
  const model = argv[argv.indexOf('--model') + 1] ?? 'sonnet';
  const usage = { inputTokens: 0, outputTokens: 0, cacheReadInputTokens: 0, cacheCreationInputTokens: 0, costUSD: 0 };
  let msg = 0;
  let pendingAnswer = null;
  let slowTimer = null;
  let asked = false;
  const replay = argv.includes('--replay-user-messages');
  const appended = argv.includes('--append-system-prompt') ? (argv[argv.indexOf('--append-system-prompt') + 1] ?? '') : '';
  // What a copy of an agent is told of its folder is no ticket's protocol: it plays a plain agent.
  const copied = ['Cette conversation a été copiée', 'This conversation was copied'];
  const sys = copied.some((c) => appended.startsWith(c)) ? '' : appended;
  let remoteSent = false;
  let entries = 0;
  let slowClose = false;

  const out = (o) => process.stdout.write(JSON.stringify(o) + '\n');
  const ok = (id, response = {}) => out({ type: 'control_response', response: { subtype: 'success', request_id: id, response } });

  // An entry of the session, kept in the account's folder when it has one.
  function keep(entry) {
    if (!configDir) return;
    const file = sessionFile(sessionId);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.appendFileSync(file, JSON.stringify({ ...entry, sessionId, cwd: process.cwd() }) + '\n');
  }

  // The quota windows it tells: full while the account is out of quota.
  const windows = () => ({
    five_hour: { utilization: limited() ? 1 : 0.12, resetsAt: 1790558400 },
    seven_day: { utilization: 0.34, resetsAt: 1790805600 },
  });

  function assistant(block, id = `msg_${process.pid}_${++msg}`) {
    const uuid = `entry-${process.pid}-${++entries}`;
    const message = { id, role: 'assistant', content: [block] };
    out({ type: 'assistant', message, parent_tool_use_id: null, session_id: sessionId, uuid });
    keep({ type: 'assistant', uuid, message });
    return id;
  }

  function streamText(text) {
    const id = `msg_${process.pid}_${++msg}`;
    const ev = (event) => out({ type: 'stream_event', event, parent_tool_use_id: null, session_id: sessionId });
    ev({
      type: 'message_start',
      message: { id, usage: { input_tokens: 10, cache_read_input_tokens: 1000, cache_creation_input_tokens: 0 } },
    });
    ev({ type: 'content_block_start', index: 0, content_block: { type: 'text', text: '' } });
    for (const part of text.match(/.{1,6}/gs) ?? [])
      ev({ type: 'content_block_delta', index: 0, delta: { type: 'text_delta', text: part } });
    assistant({ type: 'text', text }, id);
    ev({ type: 'content_block_stop', index: 0 });
    ev({ type: 'message_stop' });
  }

  function result({ isError = false, subtype = 'success' } = {}) {
    usage.inputTokens += 100;
    usage.outputTokens += 20;
    usage.cacheReadInputTokens += 1000;
    usage.costUSD = Math.round((usage.costUSD + 0.05) * 1e6) / 1e6;
    out({
      type: 'result',
      subtype,
      is_error: isError,
      duration_ms: 1200,
      session_id: sessionId,
      result: isError ? 'Erreur simulée' : 'ok',
      modelUsage: { [`claude-${model}-test`]: { ...usage, contextWindow: 200000, canonicalModel: `claude-${model}-test` } },
    });
    out({ type: 'rate_limit_event', rate_limit_info: { unifiedWindows: windows() } });
  }

  // AskUserQuestion; once answered, `then` goes on with the turn (else a plain reply ends it).
  function askQuestion(then = null) {
    const tuid = `toolu_q${msg}`;
    assistant({ type: 'tool_use', id: tuid, name: 'AskUserQuestion', input: {} });
    const input = {
      questions: [
        {
          question: 'Quelle base de données ?',
          header: 'Base',
          multiSelect: false,
          options: [
            { label: 'PostgreSQL', description: 'Relationnelle' },
            { label: 'SQLite', description: 'Embarquée' },
          ],
        },
      ],
    };
    pendingAnswer = { id: 'req_question', tuid, kind: 'question', then };
    out({
      type: 'control_request',
      request_id: 'req_question',
      request: { subtype: 'can_use_tool', tool_name: 'AskUserQuestion', input, tool_use_id: tuid, requires_user_interaction: true },
    });
  }

  function ticketTurn(text) {
    const key = sys.match(/ticket ([A-Z]+-\d+)/)?.[1] ?? 'TIC-0';
    const count = Number(sys.match(/(?:Critères d'acceptation|Acceptance criteria) \((\d+)\)/)?.[1] ?? 1);
    const loop = Number(text.match(/(?:Boucle|Loop) (\d+)\//)?.[1] ?? 1);
    const all = `${sys}\n${text}`;
    if (all.includes('[question]') && !asked) {
      asked = true;
      return askQuestion(() => ticketTurn(text));
    }
    if (all.includes('[lent]')) {
      streamText('Je commence…');
      slowTimer = setTimeout(() => result(), 30_000);
      return;
    }
    const file = `${key.toLowerCase()}.txt`;
    if (!all.includes('[rien]')) fs.writeFileSync(path.join(process.cwd(), file), `Boucle ${loop}\n`);
    if (all.includes('[commite]')) {
      execFileSync('git', ['add', '-A']);
      // The copied .env, which git ignores, forced in.
      if (fs.existsSync('.env')) execFileSync('git', ['add', '-f', '.env']);
      execFileSync('git', ['commit', '-qm', `Travail sur ${key}`]);
    }
    if (all.includes('[retire-env]')) {
      execFileSync('git', ['rm', '--cached', '-q', '.env']);
      execFileSync('git', ['commit', '-qm', `Retire .env de ${key}`]);
    }
    if (all.includes('[sans-bilan]')) {
      streamText('Travail fait, sans bilan.');
      result();
      return;
    }
    const met = (n) => all.includes('[ok]') || (!all.includes('[jamais]') && n <= loop);
    const report = {
      criteres: Array.from({ length: count }, (_, i) => ({
        n: i + 1,
        ok: met(i + 1),
        note: met(i + 1) ? 'vérifié' : `reste le critère ${i + 1}`,
      })),
      avancement: [`Fichier ${file} écrit`, `Boucle ${loop} faite`],
    };
    if (all.includes('[recette]')) {
      // On its block of ports, as « Prépare le lancement » answers.
      const port = Number(sys.match(/(?:Ports réservés à ce worktree : |Ports reserved for this worktree: )(\d+)/)?.[1] ?? 4100) + 1;
      report.lancement = {
        processus: [{ nom: 'web', commande: 'node serveur.js', url: `http://localhost:${port}` }],
        ouvrir: `http://localhost:${port}/fonction`,
      };
    }
    streamText(`Boucle ${loop} faite.\n\n\`\`\`escouade\n${JSON.stringify(report)}\n\`\`\``);
    result();
  }

  function toolResult(toolUseId, content, extra = {}) {
    out({
      type: 'user',
      message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: toolUseId, content, is_error: false }] },
      parent_tool_use_id: null,
      ...extra,
    });
  }

  function onUser(text) {
    out({ type: 'system', subtype: 'init', session_id: sessionId, model, cwd: process.cwd(), permissionMode: 'default' });
    if (text.includes('[ferme-lentement]')) slowClose = true;
    if (text.includes('crash')) process.exit(3);
    if (text.includes('grandchild')) {
      // Like a dev server started by the Bash tool: must die with the agent's process tree.
      const child = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60000)'], { stdio: 'ignore' });
      streamText(`pid:${child.pid}`);
      result();
      return;
    }
    if (text.includes('limite') || limited()) {
      // The usage limit, as Claude Code tells it: the rejected window with its reset, its own
      // message typed as a rate limit, and a failed turn.
      const resetsAt = Math.floor(Date.now() / 1000) + 3600;
      out({
        type: 'rate_limit_event',
        rate_limit_info: {
          status: 'rejected',
          resetsAt,
          rateLimitType: 'five_hour',
          unifiedWindows: { five_hour: { utilization: 1, resetsAt } },
        },
        session_id: sessionId,
      });
      out({
        type: 'assistant',
        error: 'rate_limit',
        message: { id: `msg_limit${msg}`, role: 'assistant', content: [{ type: 'text', text: "You've hit your limit · resets 3pm" }] },
        parent_tool_use_id: null,
        session_id: sessionId,
      });
      out({
        type: 'result',
        subtype: 'success',
        is_error: true,
        duration_ms: 100,
        session_id: sessionId,
        result: "You've hit your limit · resets 3pm",
      });
      return;
    }
    if (failing()) {
      // An error of the API, as Claude Code tells it: a failed turn, no rate limit.
      const said = 'Invalid API key · Please run /login';
      out({
        type: 'assistant',
        error: 'authentication_failed',
        message: { id: `msg_error${msg}`, role: 'assistant', content: [{ type: 'text', text: said }] },
        parent_tool_use_id: null,
        session_id: sessionId,
      });
      out({ type: 'result', subtype: 'success', is_error: true, duration_ms: 100, session_id: sessionId, result: said });
      return;
    }
    if (text.includes('Prépare le lancement') || text.includes('Prepare the test launch')) {
      const port = Number(text.match(/(?:Ports réservés : |Ports reserved: )(\d+)/)?.[1] ?? 4100) + 1;
      const lancement = {
        processus: [{ nom: 'web', commande: 'node serveur.js', url: `http://localhost:${port}` }],
        ouvrir: `http://localhost:${port}/fonction`,
      };
      streamText(`Voici la recette.\n\n\`\`\`escouade\n${JSON.stringify({ lancement })}\n\`\`\``);
      result();
      return;
    }
    if (sys) return ticketTurn(text);
    if (text.includes('tâche de fond')) {
      // A command left running in the background: it ends once the turn is over, and Claude Code
      // only tells it with a system frame before starting a turn by itself.
      const tuid = `toolu_b${msg}`;
      out({
        type: 'system',
        subtype: 'task_started',
        task_id: 'fakebg1',
        tool_use_id: tuid,
        description: 'npm test',
        is_backgrounded: true,
      });
      assistant({ type: 'tool_use', id: tuid, name: 'Bash', input: { command: 'npm test', run_in_background: true } });
      toolResult(tuid, 'Command running in background with ID: fakebg1.');
      streamText('Lancé en arrière-plan.');
      result();
      setTimeout(() => {
        out({
          type: 'system',
          subtype: 'task_notification',
          task_id: 'fakebg1',
          tool_use_id: tuid,
          status: 'completed',
          summary: 'Background command "npm test" completed (exit code 0)',
          uuid: `bgdone-${msg}`,
          session_id: sessionId,
        });
        streamText('Les tests sont passés.');
        result();
      }, 400);
      return;
    }
    if (text.includes('sous-agent')) {
      // A background subagent: launched, then its report and the end of its task are passed on
      // to Claude by Claude Code itself (replayed with their origin), as mid-turn.
      const tuid = `toolu_a${msg}`;
      assistant({ type: 'tool_use', id: tuid, name: 'Agent', input: { description: 'Chercher la cause du bug', prompt: '…' } });
      toolResult(tuid, 'Async agent launched successfully.\nagentId: fakeagent1 (internal ID)');
      const passOn = (uuid, content, origin) =>
        out({
          type: 'user',
          message: { role: 'user', content },
          parent_tool_use_id: null,
          uuid,
          isReplay: true,
          origin,
          session_id: sessionId,
        });
      passOn(
        `peer-${msg}`,
        '<agent-message from="fakeagent1">\n[Subagent hand-back] The text below is the final report of a subagent. The report follows:\n  **Cause trouvée** : le filtre refuse les PDF.\n</agent-message>',
        { kind: 'peer', from: 'fakeagent1', senderTaskId: 'fakeagent1' },
      );
      passOn(
        `notif-${msg}`,
        `<task-notification>\n<task-id>fakeagent1</task-id>\n<tool-use-id>${tuid}</tool-use-id>\n<status>completed</status>\n<summary>Agent "Chercher la cause du bug" finished</summary>\n</task-notification>`,
        { kind: 'task-notification', producer: 'session-task' },
      );
      streamText('Rapport reçu.');
      result();
      return;
    }
    if (text.includes('question')) return askQuestion();
    if (text.includes('permission')) {
      const tuid = `toolu_p${msg}`;
      const input = { command: 'rm -rf build', description: 'Supprime le dossier build' };
      assistant({ type: 'tool_use', id: tuid, name: 'Bash', input });
      pendingAnswer = { id: 'req_perm', tuid, kind: 'permission' };
      out({
        type: 'control_request',
        request_id: 'req_perm',
        request: {
          subtype: 'can_use_tool',
          tool_name: 'Bash',
          input,
          tool_use_id: tuid,
          decision_reason: 'Commande destructive',
          permission_suggestions: [
            {
              type: 'addRules',
              rules: [{ toolName: 'Bash', ruleContent: 'rm -rf build' }],
              behavior: 'allow',
              destination: 'localSettings',
            },
          ],
        },
      });
      return;
    }
    if (text.includes('edit')) {
      const tuid = `toolu_e${msg}`;
      const file = path.join(process.cwd(), 'src', 'app.ts');
      assistant({ type: 'tool_use', id: tuid, name: 'Edit', input: { file_path: file, old_string: 'a', new_string: 'b' } });
      toolResult(tuid, 'The file has been updated.', {
        tool_use_result: {
          filePath: file,
          structuredPatch: [
            { oldStart: 1, oldLines: 1, newStart: 1, newLines: 2, lines: ['-const a = 1;', '+const a = 2;', '+const b = 3;'] },
          ],
        },
      });
      streamText('Fichier modifié.');
      result();
      return;
    }
    if (text.includes('slow')) {
      streamText('Je commence…');
      slowTimer = setTimeout(() => result(), 30_000);
      return;
    }
    streamText(`Bonjour, tu as dit : ${text}`);
    result();
  }

  function onControlResponse(resp) {
    if (!pendingAnswer || resp.request_id !== pendingAnswer.id) return;
    const p = pendingAnswer;
    pendingAnswer = null;
    const r = resp.response ?? {};
    if (p.kind === 'question') {
      const answers = r.updatedInput?.answers ?? {};
      toolResult(p.tuid, `Réponses : ${JSON.stringify(answers)}`, { tool_use_result: { answers } });
      if (p.then) return p.then();
      streamText(`Choix retenu : ${Object.values(answers).join(', ')}`);
    } else if (r.behavior === 'allow') {
      toolResult(p.tuid, 'build supprimé', { tool_use_result: { stdout: 'build supprimé', stderr: '', interrupted: false } });
      streamText(r.updatedPermissions ? 'Commande exécutée (règle enregistrée).' : 'Commande exécutée.');
    } else {
      out({
        type: 'user',
        message: {
          role: 'user',
          content: [{ type: 'tool_result', tool_use_id: p.tuid, content: `Refusé : ${r.message}`, is_error: true }],
        },
        parent_tool_use_id: null,
      });
      streamText(`Compris : ${r.message}`);
    }
    result();
  }

  const rl = readline.createInterface({ input: process.stdin });
  rl.on('line', (line) => {
    if (!line.trim()) return;
    const m = JSON.parse(line);
    if (m.type === 'control_response') return onControlResponse(m.response);
    if (m.type === 'control_request') {
      const r = m.request;
      switch (r.subtype) {
        case 'initialize':
          return ok(m.request_id, {
            commands: [
              { name: 'compact', description: 'Compacte le contexte', argumentHint: '' },
              { name: 'review', description: 'Revue de code', argumentHint: '[pr]' },
            ],
            // As Claude Code 2.1.284 lists them (abridged): an alias and the model it runs.
            models: [
              { value: 'default', resolvedModel: 'claude-opus-5-5', displayName: 'Default (recommended)' },
              { value: 'sonnet', resolvedModel: 'claude-sonnet-5-5', displayName: 'Sonnet 5.5' },
              { value: 'claude-fable-5-1', resolvedModel: 'claude-fable-5-1', displayName: 'Fable 5.1' },
            ],
            account: {},
          });
        case 'get_usage':
          return ok(m.request_id, {
            rate_limits: {
              five_hour: { utilization: limited() ? 100 : 12, resets_at: '2026-09-28T01:20:00+00:00' },
              seven_day: { utilization: 34, resets_at: '2026-09-30T22:00:00+00:00' },
            },
          });
        case 'interrupt':
          clearTimeout(slowTimer);
          if (sys.includes('[fin-d-abord]')) {
            // The app reads the turn's end before the interrupt's answer reaches its caller.
            result({ isError: true, subtype: 'error_during_execution' });
            return ok(m.request_id, { still_queued: [] });
          }
          ok(m.request_id, { still_queued: [] });
          return result({ isError: true, subtype: 'error_during_execution' });
        case 'remote_control':
          return remoteControl(m.request_id, r);
        default:
          return ok(m.request_id);
      }
    }
    if (m.type === 'user') {
      fs.appendFileSync(logFile.replace(/\.jsonl$/, '') + '.stdin.jsonl', JSON.stringify(m) + '\n');
      keep({ type: 'user', uuid: m.uuid, message: m.message });
      // Real CLI: echoes stdin messages (same uuid) with --replay-user-messages.
      if (replay) out({ type: 'user', message: m.message, parent_tool_use_id: null, uuid: m.uuid, isReplay: true, session_id: sessionId });
      onUser(typeof m.message.content === 'string' ? m.message.content : m.message.content.map((b) => b.text ?? '').join(' '));
    }
  });

  // Remote Control: requests are logged to <log>.control.jsonl. With $FAKE_CLAUDE_REMOTE_MESSAGE,
  // a message "sent from claude.ai" arrives once it is on (replayed with origin human, then run).
  function remoteControl(id, r) {
    fs.appendFileSync(logFile.replace(/\.jsonl$/, '') + '.control.jsonl', JSON.stringify(r) + '\n');
    if (!r.enabled) {
      ok(id);
      return out({ type: 'system', subtype: 'bridge_state', state: 'disconnected', session_id: sessionId });
    }
    const bridge = r.reattach_session_id ?? `cse_fake_${process.pid}`;
    out({ type: 'system', subtype: 'bridge_state', state: 'ready', session_id: sessionId });
    ok(id, { session_url: `https://claude.ai/code/session_${bridge.slice(4)}`, bridge_session_id: bridge, bridge_epoch: 1 });
    out({ type: 'system', subtype: 'bridge_state', state: 'connected', bridge_epoch: 1, session_id: sessionId });
    const text = process.env.FAKE_CLAUDE_REMOTE_MESSAGE;
    if (text && !remoteSent) {
      remoteSent = true;
      setTimeout(() => {
        out({
          type: 'user',
          message: { role: 'user', content: text },
          parent_tool_use_id: null,
          uuid: `remote-${process.pid}`,
          isReplay: true,
          origin: { kind: 'human' },
          session_id: sessionId,
        });
        onUser(text);
      }, 300);
    }
  }
  // [tenace]: like a CLI that takes its time to finish once its input is closed.
  rl.on('close', () => {
    if (slowClose) {
      // Still writing its session for a moment: copied too early, the copy misses the last line.
      return setTimeout(() => {
        keep({ type: 'closed' });
        process.exit(0);
      }, 800);
    }
    return sys.includes('[tenace]') ? setTimeout(() => process.exit(0), 5000) : process.exit(0);
  });
}
