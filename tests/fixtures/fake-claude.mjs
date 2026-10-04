#!/usr/bin/env node
// Test double of the `claude` CLI in `--input-format/--output-format stream-json` mode.
// It replays the frame shapes documented in docs/PROTOCOL.md. The user message text picks the
// scenario: "question", "permission", "edit", "slow", "crash", "grandchild"; anything else is a
// plain reply. A `--resume=missing…` session fails like an unknown session does.
// Every launch appends {argv, cwd} to $FAKE_CLAUDE_LOG, by default
// <tmp>/fake-claude-<cwd with non-alphanumerics replaced by _>.jsonl, and every user message
// read on stdin to <log>.stdin.jsonl.
// Started with --append-system-prompt (a ticket's protocol), it plays the ticket's agent: it writes
// <key>.txt ("Boucle n") in its folder and ends each turn with an ```escouade report (criteria and
// "avancement"). The ticket's title, in the protocol, steers it: [ok] every criterion met at once,
// [jamais] none ever, [sans-bilan] no report, [lent] a turn that lasts 30 s, [recette] a launch
// recipe (its process on the second port of its block), [question] a question first (the turn goes
// on once it is answered), [fin-d-abord] an interrupted turn's end sent before the answer to the
// interrupt, [commite] its work committed by itself (every file of its folder, a copied .env forced
// in), [retire-env] then .env taken out of git again in a second commit (still in the branch's
// history), [tenace] a process that lasts 5 s once its input is closed; by default criterion n is
// met from loop n on.
// Asked to prepare a test launch (« Prépare le lancement… Ports réservés : <base> »), any agent
// answers with a recipe whose process listens on <base + 1>.
// In one-shot mode, asked for a ticket's commit message, it answers `feat: travail du faux claude
// [<KEY>]`, or a sentence out of form when the ticket's title says [message-libre]; with [sourd]
// in its system prompt it never reads its input and answers nothing for 20 s.

import { execFileSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import readline from 'node:readline';

const argv = process.argv.slice(2);
const logFile = process.env.FAKE_CLAUDE_LOG || path.join(os.tmpdir(), `fake-claude-${process.cwd().replace(/[^a-zA-Z0-9]/g, '_')}.jsonl`);
fs.appendFileSync(logFile, JSON.stringify({ argv, cwd: process.cwd(), proxy: process.env.HTTPS_PROXY ?? null }) + '\n');

// One-shot mode (`-p --output-format json`), used by the app to name agents.
if (argv.includes('-p') && argv.some((a) => a.includes('[sourd]'))) {
  setTimeout(() => {}, 20_000);
} else if (argv.includes('-p')) {
  let input = '';
  process.stdin.on('data', (d) => (input += d));
  process.stdin.on('end', () => {
    // A ticket's commit message.
    const ticket = input.match(/<ticket>\s*([A-Z]+-\d+)/)?.[1];
    if (ticket) {
      const result = input.includes('[message-libre]') ? 'Voici le message : travail fait' : `feat: travail du faux claude [${ticket}]`;
      process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result }));
      return;
    }
    const task = input.match(/<tache>\s*([\s\S]*?)\s*<\/tache>/)?.[1] ?? '';
    const words = task.toLowerCase().match(/[a-z]+/g) ?? ['tache'];
    process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result: `${words.slice(0, 2).join('-')}-fake` }));
  });
} else {
  startSession();
}

function startSession() {
  const resume = argv.find((a) => a.startsWith('--resume='))?.slice('--resume='.length);
  if (resume?.startsWith('missing')) {
    process.stderr.write(`No conversation found with session ID: ${resume}\n`);
    process.exit(1);
  }
  const sessionId = resume ?? `sess-${process.pid}`;
  const model = argv[argv.indexOf('--model') + 1] ?? 'sonnet';
  const usage = { inputTokens: 0, outputTokens: 0, cacheReadInputTokens: 0, cacheCreationInputTokens: 0, costUSD: 0 };
  let msg = 0;
  let pendingAnswer = null;
  let slowTimer = null;
  let asked = false;
  const replay = argv.includes('--replay-user-messages');
  const sys = argv.includes('--append-system-prompt') ? (argv[argv.indexOf('--append-system-prompt') + 1] ?? '') : '';
  let remoteSent = false;

  const out = (o) => process.stdout.write(JSON.stringify(o) + '\n');
  const ok = (id, response = {}) => out({ type: 'control_response', response: { subtype: 'success', request_id: id, response } });

  function assistant(block, id = `msg_${process.pid}_${++msg}`) {
    out({ type: 'assistant', message: { id, role: 'assistant', content: [block] }, parent_tool_use_id: null, session_id: sessionId });
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
    out({
      type: 'rate_limit_event',
      rate_limit_info: {
        unifiedWindows: { five_hour: { utilization: 0.12, resetsAt: 1790558400 }, seven_day: { utilization: 0.34, resetsAt: 1790805600 } },
      },
    });
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
    const key = sys.match(/le ticket ([A-Z]+-\d+)/)?.[1] ?? 'TIC-0';
    const count = Number(sys.match(/Critères d'acceptation \((\d+)\)/)?.[1] ?? 1);
    const loop = Number(text.match(/Boucle (\d+)\//)?.[1] ?? 1);
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
    fs.writeFileSync(path.join(process.cwd(), file), `Boucle ${loop}\n`);
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
      const port = Number(sys.match(/Ports réservés à ce worktree : (\d+)/)?.[1] ?? 4100) + 1;
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
    if (text.includes('crash')) process.exit(3);
    if (text.includes('grandchild')) {
      // Like a dev server started by the Bash tool: must die with the agent's process tree.
      const child = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60000)'], { stdio: 'ignore' });
      streamText(`pid:${child.pid}`);
      result();
      return;
    }
    if (text.includes('limite')) {
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
    if (text.includes('Prépare le lancement')) {
      const port = Number(text.match(/Ports réservés : (\d+)/)?.[1] ?? 4100) + 1;
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
              five_hour: { utilization: 12, resets_at: '2026-09-28T01:20:00+00:00' },
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
  rl.on('close', () => (sys.includes('[tenace]') ? setTimeout(() => process.exit(0), 5000) : process.exit(0)));
}
