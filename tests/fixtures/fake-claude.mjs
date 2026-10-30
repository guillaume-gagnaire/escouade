#!/usr/bin/env node
// Test double of the `claude` CLI in `--input-format/--output-format stream-json` mode.
// It replays the frame shapes documented in docs/PROTOCOL.md. The user message text picks the
// scenario: "question", "permission", "edit", "slow", "crash", "grandchild", "raisonnement" (a thinking block
// before the reply); anything else is a
// plain reply ("grandchild" starts a program that lasts a minute in its folder, and says its pid; with
// [detached] too, that program outlives the process). A `--resume=missing…` session fails like an
// unknown session does, forked or not; with `--fork-session`, the resumed conversation goes on under a new session id, and a
// `--resume-session-at=missing…` entry fails as one Claude Code cannot find does. Each assistant
// message carries the uuid of its entry in the session (`entry-<pid>-<n>`).
// "flood 90s" (60 s by default) keeps it busy for that long, as an agent hard at work: a text streamed in small deltas (one
// every 15 ms) and a tool call with ~1.5 KB of output every ~170 ms, then the turn ends; an interrupt stops it.
// "flood-real 90s" (60 s by default) plays a busy superpowers agent for that long, timers pacing it: a 40-task plan, then subagents, tool results of 2-30 KB.
// Every launch appends {argv, cwd, pid, proxy, tls, configDir, todoTools, autoUpdater} (its process id, which names a
// new session of it `sess-<pid>`, its HTTPS_PROXY,
// NODE_TLS_REJECT_UNAUTHORIZED, CLAUDE_CONFIG_DIR, CLAUDE_CODE_ENABLE_TODO_TOOLS and DISABLE_AUTOUPDATER, null when unset) to $FAKE_CLAUDE_LOG, by
// default <tmp>/fake-claude-<cwd with non-alphanumerics replaced by _>.jsonl, and every user message
// read on stdin to <log>.stdin.jsonl.
// With CLAUDE_CONFIG_DIR (a Claude account of its own), it keeps its sessions where Claude Code
// does, in <dir>/projects/<cwd with non-alphanumerics replaced by ->/<session id>.jsonl (each user
// message and assistant entry on a line), and a --resume of a session absent from every
// <dir>/projects/* folder fails as an unknown one does; while a <dir>/fake-limit file is there, every
// turn is stopped by the usage limit (as "limite") and its quota windows say 100 % (rate_limit_event,
// get_usage). While a <dir>/fake-error file is there, every turn fails with an error of the API
// ("Invalid API key", as an account the user is not signed in to gets), which is no usage limit.
// While a <dir>/fake-oauth-lost file is there, a turn fails as a sign-in Claude Code could not refresh does
// (« OAuth session expired and could not be refreshed », ESC-162), and the process that failed so fails every turn after
// (its refresh token dead in its memory, as Claude Code keeps it): a number in the file is how many turns fail so,
// the file gone once they have; empty, every one.
// While a <dir>/fake-exit file is there, the process ends before it starts (exit code 1), its stderr the text of the file
// (« No conversation found with session ID: … », « Invalid API key · Please run /login »).
// A user message with [ferme-lentement] makes the process take 800 ms to end once its input is
// closed, and write {"type":"closed"} to its session just before it does (a resume that copies
// the session before the process is gone misses that line). Without CLAUDE_CONFIG_DIR, none of this.
// [quota-rythme] in a message makes the quota windows it tells from then on end a little later than now, as a window
// half way through does: 5 h at 50 % with 3 h left (→ 125 % at its reset), 7 days at 34 % with 3 days and a half left;
// with CLAUDE_CONFIG_DIR, for every process of the account from then on (<dir>/fake-paced: a reading of its quota too).
// `claude mcp add|remove|get|list` keep the user scope's MCP servers in <folder>/.claude.json (see mcpCommand).
// What Escouade tells it is read in French or in English (« Langue des textes rédigés par Claude »):
// each scenario below is recognized in both, its answers stay the same.
// Started with --append-system-prompt (a ticket's protocol; not what a copy of an agent is told of
// its new folder, « Cette conversation a été copiée… », “This conversation was copied…”, nor what
// every agent is told of the shared memory, « Le projet a une mémoire partagée… »), it plays the ticket's agent: it writes
// <key>.txt ("Boucle n") in its folder and ends each turn with an ```escouade report (criteria and
// "avancement"). The ticket's title, in the protocol, steers it: [ok] every criterion met at once,
// [jamais] none ever, [sans-bilan] no report, [lent] a turn that lasts 30 s, [recette] a launch
// recipe (its process on the second port of its block), [question] a question first (the turn goes
// on once it is answered), [fin-d-abord] an interrupted turn's end sent before the answer to the
// interrupt, [commite] its work committed by itself (every file of its folder, a copied .env forced
// in), [retire-env] then .env taken out of git again in a second commit (still in the branch's
// history), [tenace] a process that lasts 5 s once its input is closed, [rien] no file written (nothing
// to merge), [dans:api] its file written in that folder instead (each one named; ESC-169); by default criterion n is
// met from loop n on. "faits-workflow: [...]" in its ticket (ESC-197) plays those facts first, then holds its turn 10 min
// as [lent] does: the agent of a ticket under way that follows a workflow too. [spec:<project>] (ESC-263): before its first
// report it keeps the spec as its protocol asks, by Escouade's docs tools (its --mcp-config server): the page
// paiement/echeances written (« Quatre échéances au plus (<key>). [écart] ») and a decision proposed. In one-shot mode, a page
// compared to the code (<spec-page>) gets a gap when it says [écart] (none otherwise; [muet]: the model fails).
// In one-shot mode, « Planifier » (ESC-262, <changed-page>) gets a ticket per page changed, each after the one before (see
// the planner's branch below).
// [proposer:<project>] in a message (ESC-261, a discussion of the Docs tab): by the same tools, the page
// espace-adherent/relances written (« Trois relances par e-mail… »), the page notifications/general changed when it is there,
// and a decision proposed (« Relances par e-mail seulement »), then « Proposé dans la spec. ». The message of « Mettre la spec à
// jour » (« (.escouade/spec/<path>.md) ne dit pas ce que fait le code ») makes the spec writer propose that page without its
// [écart], « Mis à jour d’après le code par le faux claude. » after it (the project named by its instructions).
// "[worktree <branch>]" in a message goes into a worktree of its own as Claude Code's EnterWorktree does (ESC-161): made
// under <repository>/.claude/worktrees/<branch with non-alphanumerics replaced by -> on the new branch <branch> (or
// gone into again), the session goes on there (the `cwd` of its entries), kept in its first folder's project folder.
// "plan superpowers" in the message plays a plan run as superpowers:subagent-driven-development does it
// (see tests/fixtures/superpowers-run.md): the plan and its ledger written for real in the agent's
// folder, a task list (TaskCreate/TaskUpdate), a foreground implementer and reviewer subagent per
// task, a background subagent and a workflow on the second one, a question in the middle of the third,
// its workspace removed, then "Plan terminé". "plan superpowers sans liste": the same without any task tool, so that the
// plan file and the ledger are all there is to read; "interrompu" in the message cuts the turn in
// the middle of the first implementer.
// "faits-workflow: [...]" in a message plays the facts of an agent that follows a workflow (ESC-193, for the e2e of the
// Workflow tab): a JSON list of { agent: "gsd-planner" } (a subagent in the foreground, back at once), { skill: "name" },
// { write: "path" } (the file written for real in its folder), { bash: "command" }, { say: "text" } (its message) and
// { wait: ms }, played in order in one turn that ends with its cost. The message itself may be a door (`/gsd:plan-phase 2
// faits-workflow: [...]`); "faits-workflow-erreur: [...]" ends the turn in error instead.
// "sous-agent long" and "sous-agent fini" launch a subagent in the background as Claude Code 2.1.300 does (ESC-13): its
// own thread asks a permission (`can_use_tool` with its `agent_id`, request "req_subperm"); "sous-agent long" then runs
// until a `stop_task` of its task ("fakelong-agent", logged to <log>.control.jsonl), "sous-agent fini" hands back its
// report (`handback_report`) once the permission is answered.
// "sous-agent sans modèle" launches a subagent in the foreground whose call names no model (its definition chooses) and
// whose message comes from claude-haiku-5-5, then hands back its result.
// "lire-fichier <path>" in the message makes it ask the permission to `Read` that file, "modifier-fichier <path>" to `Edit` it
// (the rest of the first line is the path); answered, it says « Read : allow » (or deny, Edit…) and ends the turn.
// "commande de fond 20s", "surveillance de fond 20s" and "commande premier plan 20s" run a command, a `Monitor` and a
// command that goes to the background from the foreground, as Claude Code 2.1.301 does (ESC-14): see bgRun (the output
// grows by a line every 200 ms, answers `get_task_output`, ends by itself after that long or by a `stop_task`).
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
// Asked for a pull request (<commits>), it answers a title (« Le travail de l’agent, proposé par le faux claude », with
// " (en)" in English) and the commits it read as the description.
// Asked to name a task (<tache>, <task>), it answers with its first two words. Asked in English (its
// question and its role both), a name ends with "-en", a direct commit's message with " (en)"; with
// a question and a role that disagree, "mixed" says so in the same places. A task that says [muet] gets
// no name (it fails), one that says [lent] gets it 3 s late.
// "envoie des fichiers" (ESC-171): it writes two PNG images and a PDF in a temporary folder of its own folder
// (`.impeccable/tmp/`), and sends them to the user by SendUserFile, their paths written with `/` as Claude Code does
// under Windows; with [proactif] the call says it sends them of itself.

import { execFileSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import readline from 'node:readline';
import zlib from 'node:zlib';

const argv = process.argv.slice(2);
// `--version`, as Claude Code prints it, not logged (the app asks it before an agent starts, not a launch of one):
// FAKE_CLAUDE_VERSION plays another version (an older one than the tested ones is refused by the app).
if (argv[0] === '--version') {
  process.stdout.write(`${process.env.FAKE_CLAUDE_VERSION || '2.1.301'} (Claude Code)\n`);
  process.exit(0);
}
// `update`, as Escouade runs it apart from its agents (ESC-133), not logged with the launches: FAKE_CLAUDE_UPDATE_LOG
// (by default <log>.update.jsonl) gets {started, pid, autoUpdater} at once, then {done} once FAKE_CLAUDE_UPDATE_MS (0
// by default) went by; it then prints FAKE_CLAUDE_UPDATE_SAYS (« Claude Code is up to date (2.1.301) » by default)
// and ends with FAKE_CLAUDE_UPDATE_EXIT (0 by default).
if (argv[0] === 'update') {
  const defaultLog =
    process.env.FAKE_CLAUDE_LOG || path.join(os.tmpdir(), `fake-claude-${process.cwd().replace(/[^a-zA-Z0-9]/g, '_')}.jsonl`);
  const log = process.env.FAKE_CLAUDE_UPDATE_LOG || `${defaultLog}.update.jsonl`;
  const write = (line) => fs.appendFileSync(log, JSON.stringify(line) + '\n');
  write({ started: true, pid: process.pid, autoUpdater: process.env.DISABLE_AUTOUPDATER ?? null });
  // Waited for here, nothing below run: the rest of this file is the other commands.
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, Number(process.env.FAKE_CLAUDE_UPDATE_MS || 0));
  write({ done: true });
  fs.writeSync(1, `${process.env.FAKE_CLAUDE_UPDATE_SAYS || 'Claude Code is up to date (2.1.301)'}\n`);
  process.exit(Number(process.env.FAKE_CLAUDE_UPDATE_EXIT || 0));
}
const logFile = process.env.FAKE_CLAUDE_LOG || path.join(os.tmpdir(), `fake-claude-${process.cwd().replace(/[^a-zA-Z0-9]/g, '_')}.jsonl`);
fs.appendFileSync(
  logFile,
  JSON.stringify({
    argv,
    cwd: process.cwd(),
    pid: process.pid,
    proxy: process.env.HTTPS_PROXY ?? null,
    tls: process.env.NODE_TLS_REJECT_UNAUTHORIZED ?? null,
    configDir: process.env.CLAUDE_CONFIG_DIR ?? null,
    todoTools: process.env.CLAUDE_CODE_ENABLE_TODO_TOOLS ?? null,
    autoUpdater: process.env.DISABLE_AUTOUPDATER ?? null,
  }) + '\n',
);

// The account's own folder, if it has one (set empty: as if it were not).
const configDir = process.env.CLAUDE_CONFIG_DIR || null;
// The refresh of an out-of-date sign-in (ESC-162), while a <dir>/fake-refresh file is there: each process tells
// <dir>/fake-refresh.jsonl whether the sign-in of <dir>/.credentials.json was out of date as it started, and one that
// found it so writes a new one in date, the number of ms the file says later (300 by default), before it answers
// anything, as Claude Code refreshes it.
// A process that talks to the API alone: not `--version`, `mcp` or `update`.
const session = argv.includes('--input-format') || argv.includes('-p');
if (session && configDir !== null && fs.existsSync(path.join(configDir, 'fake-refresh'))) {
  const credsFile = path.join(configDir, '.credentials.json');
  let oauth = null;
  try {
    oauth = JSON.parse(fs.readFileSync(credsFile, 'utf8')).claudeAiOauth ?? null;
  } catch {}
  const expired = !!oauth?.accessToken && typeof oauth.expiresAt === 'number' && oauth.expiresAt < Date.now();
  fs.appendFileSync(path.join(configDir, 'fake-refresh.jsonl'), JSON.stringify({ pid: process.pid, expired, at: Date.now(), argv }) + '\n');
  if (expired) {
    const ms = Number(fs.readFileSync(path.join(configDir, 'fake-refresh'), 'utf8').trim() || '300');
    const renewed = {
      ...oauth,
      accessToken: `${oauth.accessToken}-r`,
      refreshToken: `${oauth.refreshToken ?? ''}-r`,
      expiresAt: Date.now() + 3_600_000,
    };
    // Before anything is answered, as Claude Code does (an `initialize` waits for it).
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
    fs.writeFileSync(credsFile, JSON.stringify({ claudeAiOauth: renewed }));
  }
}
// Under the folder it started in, wherever it goes then (a worktree it went into).
const startCwd = process.cwd();
const sessionFile = (id) => path.join(configDir, 'projects', startCwd.replace(/[^a-zA-Z0-9]/g, '-'), `${id}.jsonl`);
// Claude Code finds a session to resume in any project folder of the account (a copy made in a
// worktree of its own resumes the original's from another one).
const sessionKept = (id) => {
  const projects = path.join(configDir, 'projects');
  const folders = fs.existsSync(projects) ? fs.readdirSync(projects) : [];
  return folders.some((f) => fs.existsSync(path.join(projects, f, `${id}.jsonl`)));
};
// Out of quota, for as long as the file is there.
const limited = () => configDir !== null && fs.existsSync(path.join(configDir, 'fake-limit'));
// The sign-in no longer refreshed (ESC-162): by the file, then for good in this process.
let oauthDead = false;
const oauthLost = () => {
  if (oauthDead) return true;
  const file = configDir === null ? null : path.join(configDir, 'fake-oauth-lost');
  if (file === null || !fs.existsSync(file)) return false;
  const left = Number(fs.readFileSync(file, 'utf8').trim() || 'NaN');
  if (left <= 1) fs.rmSync(file, { force: true });
  else if (!Number.isNaN(left)) fs.writeFileSync(file, String(left - 1));
  oauthDead = true;
  return true;
};
// Failing with an error of the API, for as long as the file is there.
const failing = () => configDir !== null && fs.existsSync(path.join(configDir, 'fake-error'));
// The alias haiku as `initialize` lists it, when <dir>/fake-haiku says which: « 5-5 » as from 2.1.293 on (Haiku 5.5, with
// the mode auto and its efforts), « 4-5 » as before (Haiku 4.5, without either); else Haiku 5.5 with the mode auto but no
// effort (as the rule of Escouade has it).
const haikuListed = () => {
  const file = configDir && path.join(configDir, 'fake-haiku');
  const which = file && fs.existsSync(file) ? fs.readFileSync(file, 'utf8').trim() : '';
  if (which === '5-5')
    return [{ value: 'haiku', resolvedModel: 'claude-haiku-5-5', displayName: 'Haiku 5.5', supportsAutoMode: true, ...efforts }];
  if (which === '4-5') return [{ value: 'haiku', resolvedModel: 'claude-haiku-4-5-20251001', displayName: 'Haiku 4.5' }];
  return [{ value: 'haiku', resolvedModel: 'claude-haiku-5-5', displayName: 'Haiku 5.5', supportsAutoMode: true }];
};
const efforts = { supportsEffort: true, supportedEffortLevels: ['low', 'medium', 'high', 'xhigh', 'max'] };
// The models `initialize` lists, as Claude Code 2.1.302 does (abridged): an alias, the model it runs, its name in `/model`,
// its efforts, whether it takes the mode auto. The account's <dir>/fake-models.json (<FAKE_CLAUDE_HOME> for Principal)
// changes them: {"add": [models listed after these], "remove": [values not listed]} (ESC-124: a model more, one less).
const modelsListed = () => {
  const listed = [
    { value: 'default', resolvedModel: 'claude-opus-5-5', displayName: 'Default (recommended)', supportsAutoMode: true, ...efforts },
    { value: 'opus', resolvedModel: 'claude-opus-5-5', displayName: 'Opus 5.5', supportsAutoMode: true, ...efforts },
    { value: 'fable', resolvedModel: 'claude-fable-5-1', displayName: 'Fable 5.1', supportsAutoMode: true, ...efforts },
    { value: 'sonnet', resolvedModel: 'claude-sonnet-5-5', displayName: 'Sonnet 5.5', supportsAutoMode: true, ...efforts },
    ...haikuListed(),
    { value: 'claude-haiku-4-5-20251001', resolvedModel: 'claude-haiku-4-5-20251001', displayName: 'Haiku 4.5' },
  ];
  const dir = configDir || process.env.FAKE_CLAUDE_HOME;
  const file = dir && path.join(dir, 'fake-models.json');
  if (!file || !fs.existsSync(file)) return listed;
  const change = JSON.parse(fs.readFileSync(file, 'utf8'));
  return [...listed.filter((m) => !(change.remove ?? []).includes(m.value)), ...(change.add ?? [])];
};

// `claude mcp add|remove|get|list`: the MCP servers of the user scope, which Escouade declares in each
// Claude account (« Claude et les agents peuvent piloter Escouade »). Kept where Claude Code keeps them, in the `mcpServers`
// of <CLAUDE_CONFIG_DIR>/.claude.json (an account's own folder, or the app's when it has one), else of
// <FAKE_CLAUDE_HOME>/.claude.json (Principal, whose real file is ~/.claude.json). It refuses to run with
// neither variable set: it never touches the real home. Like Claude Code: `add` of a name already there
// fails (exit 1, "already exists"), `remove` and `get` of one that is absent too; `--header` takes every
// argument that follows it (so Escouade puts it last). While a <folder>/fake-mcp-fail file is there, `add`
// and `remove` fail with its text; while a <folder>/fake-mcp-hang file is there, they never answer; while a
// <folder>/fake-mcp-raced file is there, the entry is removed behind their back; while a <folder>/fake-mcp-slow
// file is there, they wait as many milliseconds as it says. Every call is in the launch log above (its argv).
async function mcpCommand(args) {
  const dir = process.env.CLAUDE_CONFIG_DIR || process.env.FAKE_CLAUDE_HOME;
  if (!dir) {
    process.stderr.write('fake claude: set FAKE_CLAUDE_HOME or CLAUDE_CONFIG_DIR, `claude mcp` never touches the real home\n');
    process.exit(2);
  }
  const file = path.join(dir, '.claude.json');
  const fail = (text, code = 1) => {
    process.stderr.write(`${text}\n`);
    process.exit(code);
  };
  const read = () => {
    if (!fs.existsSync(file)) return {};
    try {
      return JSON.parse(fs.readFileSync(file, 'utf8'));
    } catch {
      return fail(`fake claude: ${file} is not JSON`);
    }
  };
  const [sub, ...rest] = args;
  const options = {};
  const positional = [];
  for (let i = 0; i < rest.length; i++) {
    const a = rest[i];
    if (!a.startsWith('-')) {
      positional.push(a);
      continue;
    }
    const name = { '-s': '--scope', '-t': '--transport', '-H': '--header', '-e': '--env' }[a] ?? a;
    // One value, but for the two options Claude Code makes variadic: everything up to the next option.
    const values = [rest[++i]];
    if (name === '--header' || name === '--env') while (i + 1 < rest.length && !rest[i + 1].startsWith('-')) values.push(rest[++i]);
    options[name] = [...(options[name] ?? []), ...values];
  }
  const scope = options['--scope']?.[0] ?? 'local';
  if ((sub === 'add' || sub === 'remove') && scope !== 'user') fail(`fake claude: only the user scope is kept (got ${scope})`);
  const [name, url] = positional;
  const slow = path.join(dir, 'fake-mcp-slow');
  if ((sub === 'add' || sub === 'remove') && fs.existsSync(slow))
    await new Promise((r) => setTimeout(r, Number(fs.readFileSync(slow, 'utf8')) || 1000));
  // While a <folder>/fake-mcp-raced file is there, someone else removes the entry just before `add` and
  // `remove` look at it.
  if ((sub === 'add' || sub === 'remove') && fs.existsSync(path.join(dir, 'fake-mcp-raced')) && read().mcpServers?.[name]) {
    const json = read();
    delete json.mcpServers[name];
    fs.writeFileSync(file, JSON.stringify(json, null, 2));
  }
  const servers = read().mcpServers ?? {};
  const refused = path.join(dir, 'fake-mcp-fail');
  if ((sub === 'add' || sub === 'remove') && fs.existsSync(path.join(dir, 'fake-mcp-hang'))) setInterval(() => {}, 1000);
  else if ((sub === 'add' || sub === 'remove') && fs.existsSync(refused))
    fail(fs.readFileSync(refused, 'utf8').trim() || 'fake claude: refused');
  else if (sub === 'add') {
    if (!name || !url) fail('error: missing required argument');
    if (name in servers) fail(`MCP server ${name} already exists in user config`);
    const headers = Object.fromEntries(
      (options['--header'] ?? []).map((h) => [h.slice(0, h.indexOf(':')).trim(), h.slice(h.indexOf(':') + 1).trim()]),
    );
    const json = read();
    json.mcpServers = {
      ...servers,
      [name]: { type: options['--transport']?.[0] ?? 'stdio', url, ...(Object.keys(headers).length ? { headers } : {}) },
    };
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(file, JSON.stringify(json, null, 2));
    process.stdout.write(`Added HTTP MCP server ${name} with URL: ${url} to user config\nFile modified: ${file}\n`);
  } else if (sub === 'remove') {
    if (!(name in servers)) fail(`No MCP server found with name: ${name}`);
    const json = read();
    delete json.mcpServers[name];
    fs.writeFileSync(file, JSON.stringify(json, null, 2));
    process.stdout.write(`Removed MCP server ${name} from user config\nFile modified: ${file}\n`);
  } else if (sub === 'get') {
    if (!(name in servers)) fail(`No MCP server found with name: ${name}`);
    const s = servers[name];
    process.stdout.write(`${name}:\n  Scope: User config\n  Status: ✔ Connected\n  Type: ${s.type}\n  URL: ${s.url}\n`);
  } else if (sub === 'list') {
    const lines = Object.entries(servers).map(([n, s]) => `${n}: ${s.url} (${s.type.toUpperCase()}) - ✔ Connected`);
    process.stdout.write(
      lines.length
        ? `Checking MCP server health...\n\n${lines.join('\n')}\n`
        : 'No MCP servers configured. Use `claude mcp add` to add a server.\n',
    );
  } else fail(`fake claude: unknown mcp command ${sub}`, 2);
}

if (argv[0] === 'mcp') {
  mcpCommand(argv.slice(1));
} else if (argv.includes('-p') && argv.some((a) => a.includes('[sourd]'))) {
  // One-shot mode (`-p --output-format json`), used by the app to name agents.
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
    // « Affiner… » (ESC-196): the workflows it is given, answered in the schema: each step described, a way
    // back from the last to the first (3 times at most). A file that says [invalide] gets prose, one that says
    // [invente] gets a step with no file, one on a file it read (« Devinée »), a way to a step that is not there
    // and a workflow of the project it does not have; one that says [muet] gets nothing (the model fails).
    if (input.includes('<workflows-json>')) {
      if (input.includes('[muet]')) {
        process.stderr.write('model unavailable\n');
        process.exit(1);
      }
      if (input.includes('[invalide]')) {
        process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result: 'Voici ce que je pense : tout va bien.' }));
        return;
      }
      const given = JSON.parse(input.match(/<workflows-json>\n([\s\S]*?)\n<\/workflows-json>/)[1]);
      const file = input.match(/^=== (.+)$/m)?.[1] ?? '';
      const workflows = given.map((w) => {
        const steps = w.steps.map((s) => ({ ...s, description: `Précisé par le faux claude : ${s.name}` }));
        const transitions = [{ from: steps[steps.length - 1].name, to: steps[0].name, condition: 'si rouge', max: 3 }];
        if (input.includes('[invente]')) {
          steps.push({
            name: 'Devinée',
            command: '/devinee',
            description: 'Une étape portée par un fichier.',
            path: file,
            agents: ['inventé'],
            reads: [],
            writes: [],
          });
          steps.push({
            name: 'Fantôme',
            command: '/fantome',
            description: 'Sans aucun fichier.',
            path: 'absent.md',
            agents: [],
            reads: [],
            writes: [],
          });
          transitions.push({ from: steps[0].name, to: 'Absent', condition: null, max: null });
        }
        return { id: w.id, steps, transitions, exit: null };
      });
      if (input.includes('[invente]')) workflows.push({ id: 'inconnu|x', steps: [], transitions: [] });
      const result = `Voici.\n\n\`\`\`json\n${JSON.stringify({ workflows })}\n\`\`\``;
      process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result }));
      return;
    }
    // « Planifier » (ESC-262): one ticket per page changed (« Implémenter : <title> »), each after the one before, the
    // first with the decisions; then again the first (twice in the list) and each ticket of the board whose title says
    // [doublon] (one the board has): Escouade leaves those out. A page that says [muet]: the model fails.
    if (input.includes('<changed-page ') || input.includes('<decision number=')) {
      const changed = [...input.matchAll(/<changed-page path="([^"]*)" title="([^"]*)">\n([\s\S]*?)\n<\/changed-page>/g)];
      if (changed.some((c) => c[3].includes('[muet]'))) {
        process.stderr.write('model unavailable\n');
        process.exit(1);
      }
      // A decision by its ref: its number (« 0003 »), or a proposal's id.
      const decisions = [...input.matchAll(/<decision ref="([^"]+)"/g)].map((m) => (/^\d+$/.test(m[1]) ? Number(m[1]) : m[1]));
      const tickets = changed.map((c, i) => ({
        title: `Implémenter : ${c[2]}`,
        description: `D’après la page ${c[1]} de la spec.`,
        criteria: [`${c[2]} : fait comme la page le dit`, 'Tests passés'],
        after: i ? [String(i)] : [],
        pages: [c[1], 'inconnu/page'],
        decisions: i ? [] : decisions,
      }));
      if (!tickets.length && decisions.length)
        tickets.push({ title: 'Appliquer les décisions', description: '', criteria: ['Fait'], after: [], pages: [], decisions });
      if (tickets.length) tickets.push({ ...tickets[0], title: tickets[0].title.toUpperCase() });
      const board = input.match(/<board>\n([\s\S]*?)\n?<\/board>/)?.[1] ?? '';
      for (const line of board.split('\n')) {
        const m = line.match(/^\S+ \[\w+\] (.*?)(?: — .*)?$/);
        if (m && m[1].includes('[doublon]')) tickets.push({ title: m[1], description: '', criteria: [], after: [], pages: [] });
      }
      const result = `Planifié.\n\n\`\`\`json\n${JSON.stringify({ tickets })}\n\`\`\``;
      process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result }));
      return;
    }
    // A page of the spec compared to the code (ESC-263): a gap when its text says [écart], on the first file of code of the
    // folder it reads (else its first file) and one outside of it; none otherwise. [muet] in it: the model fails.
    const compared = input.match(/<spec-page path="([^"]*)"[^>]*>\n([\s\S]*?)\n<\/spec-page>/);
    if (compared) {
      if (compared[2].includes('[muet]')) {
        process.stderr.write('model unavailable\n');
        process.exit(1);
      }
      const files = fs
        .readdirSync(process.cwd(), { withFileTypes: true })
        .filter((e) => e.isFile() && !e.name.startsWith('.'))
        .map((e) => e.name)
        .sort();
      const first = files.find((f) => /\.(ts|rs|js|py)$/.test(f)) ?? files[0];
      const gaps = compared[2].includes('[écart]')
        ? [
            {
              text: 'La page dit quatre échéances ; le code en accepte encore trois au plus.',
              code: [first, '../dehors.txt'].filter(Boolean),
            },
          ]
        : [];
      const result = `Comparé.\n\n\`\`\`json\n${JSON.stringify({ gaps })}\n\`\`\``;
      process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result }));
      return;
    }
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
    // A pull request's title and description (ESC-64): from the commits it read.
    if (input.includes('<commits>')) {
      const commits = input.match(/<commits>\n([\s\S]*?)<\/commits>/)?.[1].trim() ?? '';
      const asked = tongue(input.startsWith('Write the title'));
      const title =
        asked === 'fr' ? 'Le travail de l’agent, proposé par le faux claude' : `The agent’s work, proposed by the fake claude (${asked})`;
      process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result: `${title}\n\n${commits}` }));
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
    const answer = () => process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', result: named }));
    // A task that says [muet] gets no name (the model fails), one that says [lent] gets it 3 s late.
    if (task.includes('[muet]')) {
      process.stderr.write('model unavailable\n');
      process.exit(1);
    }
    if (task.includes('[lent]')) setTimeout(answer, 3000);
    else answer();
  });
} else if (argv.includes('--settings') && !argv.includes('stream-json')) {
  // A stream has settings too when Escouade's server is kept out of it (`deniedMcpServers`).
  tuiSession();
} else {
  startSession();
}

// Claude Code's own interface (« Vue Claude Code »: no -p, its hooks in --settings). It needs
// CLAUDE_CONFIG_DIR. Each line it reads (pasted text, then Enter) is a prompt: it calls the hooks
// of the settings (HTTP, with the token of ESCOUADE_HOOK_TOKEN), shows `fake-turn <prompt>` on its
// screen when a turn starts, and writes its transcript as
// Claude Code does (see tests/fixtures/tui-transcript.jsonl). "permission" in the prompt asks for
// Bash first and goes on once the next Enter comes; "lent" makes the turn wait for Escape, which
// interrupts it; "/exit" ends the process. A pasted path to an image file is the image, as Claude
// Code takes it (« [Image #1] »): its path and content (base64) are added to <dir>/fake-tui-images.jsonl.
// Like the real one (2.1.301) it sends no SessionStart hook (Claude Code skips its HTTP hooks for that
// event). Once it is up (past its welcome screen and the box about the folder) it registers itself as
// Claude Code does, in <CLAUDE_CONFIG_DIR>/sessions/<pid>.json ({pid, sessionId, cwd, startedAt, version,
// kind: "interactive", entrypoint: "cli", status}), a file it removes when it ends; while <log>.tui-welcome
// is there it is not up (a welcome screen: no registry file, no Remote Control either), and goes on when the
// file is removed. Started with --remote-control=<name>, once it is up it opens Remote Control: a
// `bridge-session` and a `bridge_status` entry with the link in its transcript, and the `bridgeSessionId`
// of its registry file, as Claude Code does. While <log>.tui-rc-hold is there it does not open it (as an
// account with no subscription, or a network that refuses), and does when the file is removed; while
// <log>.tui-rc-inactive is there the `bridge_status` it writes has no link ("/remote-control is not active")
// and the registry has no `bridgeSessionId`; while <log>.tui-rc-registry-only is there the link is told in the
// registry alone; while <log>.tui-rc-exit is there
// it closes 200 ms after it started (code 1). Each Ctrl+C (\x03) it reads is added, with its time (ms) and the
// prompt line it found, to <log>.quit.jsonl; on a line that holds something it empties it, and the second one on an
// empty line ends it (code 0), as Claude Code quits on two, unless <log>.tui-ignore-quit is there: then it goes on.
// Its terminal is raw before its first screen; while <log>.tui-slow-start is there, the rest of its start waits half a
// second after that screen.
// As Claude Code does, it writes no transcript before its first message (a session that never got one cannot be resumed), and
// a `--resume=<id>` of a session absent from every <dir>/projects/* folder (or named `missing…`) fails as an unknown one does:
// "No conversation found with session ID: <id>" on stderr, exit code 1, a moment after (the pseudo-console flushes its
// output). While a <dir>/fake-no-conversation file is there, every `--resume` fails so, the session known or not (a
// transcript that Claude Code does not see); while a <dir>/fake-tui-crash file is there, it writes
// "fake-claude-tui: crashed" on stderr and exits with code 1 at once, whatever it was asked.
// Like Claude Code's interface (a raw-mode terminal that echoes nothing itself), it draws the prompt line again at each key
// it reads: `\r ESC[2K > ` and what is typed so far; a typed character is added to it, Backspace (\x7f) takes the last one off,
// Ctrl+U (\x15) clears the line, a submitted prompt leaves an empty one. Inside a bracketed paste it draws nothing until the
// paste ends, and then the line once.
function tuiSession() {
  const settings = JSON.parse(fs.readFileSync(argv[argv.indexOf('--settings') + 1], 'utf8'));
  const flag = (name) => argv.find((a) => a.startsWith(`${name}=`))?.slice(name.length + 1);
  const side = (kind) => logFile.replace(/\.jsonl$/, '') + `.${kind}`;
  // It quits as Claude Code does, once what it wrote has reached the terminal.
  const quit = (text) => {
    try {
      fs.writeSync(2, text);
    } catch {
      process.stderr.write(text);
    }
    setTimeout(() => process.exit(1), 150);
  };
  if (configDir && fs.existsSync(path.join(configDir, 'fake-tui-crash'))) {
    quit('fake-claude-tui: crashed\r\n');
    return;
  }
  const resume = flag('--resume');
  if (
    resume &&
    (resume.startsWith('missing') || (configDir && (fs.existsSync(path.join(configDir, 'fake-no-conversation')) || !sessionKept(resume))))
  ) {
    quit(`No conversation found with session ID: ${resume}\r\n`);
    return;
  }
  const sessionId = flag('--session-id') ?? resume ?? `sess-${process.pid}`;
  const file = sessionFile(sessionId);
  const model = argv[argv.indexOf('--model') + 1] ?? 'sonnet';
  const ts = () => new Date().toISOString();
  const write = (entry) => fs.appendFileSync(file, JSON.stringify({ ...entry, sessionId, timestamp: ts() }) + '\n');
  const hook = async (name, extra = {}) => {
    const h = settings.hooks[name]?.[0]?.hooks?.[0];
    if (!h) return;
    const headers = { 'content-type': 'application/json' };
    for (const [k, v] of Object.entries(h.headers ?? {}))
      headers[k] = v.replace('$ESCOUADE_HOOK_TOKEN', process.env.ESCOUADE_HOOK_TOKEN ?? '');
    const body = { session_id: sessionId, transcript_path: file, cwd: process.cwd(), hook_event_name: name, ...extra };
    try {
      await fetch(h.url, { method: 'POST', headers, body: JSON.stringify(body) });
    } catch {}
  };
  let n = 0;
  const assistant = (content) => {
    n += 1;
    write({
      type: 'assistant',
      uuid: `tui-${process.pid}-${n}`,
      isSidechain: false,
      message: {
        id: `msg-${process.pid}-${n}`,
        model,
        role: 'assistant',
        content,
        usage: { input_tokens: 100, output_tokens: 20, cache_read_input_tokens: 1000 },
      },
    });
  };
  let waiting = null;
  // What its registry file says of it (set below).
  let status = () => {};
  let line = '';
  // As Claude Code's interface: every key as it comes (Escape alone, Enter as \r), and the prompt line drawn again at each.
  // Raw before its first screen (ESC-125): a key typed as soon as that shows would else reach a terminal still cooked, which
  // turns Enter into \n (no prompt submitted), however long the rest of its start takes.
  if (process.stdin.isTTY) process.stdin.setRawMode(true);
  process.stdin.setEncoding('utf8');
  // No space: ConPTY draws a run of spaces as a cursor move (ESC[nC). Pastes come bracketed
  // (ESC[200~ … ESC[201~), as Claude Code asks for them.
  process.stdout.write('\x1b[?2004hfake-claude-tui\r\n');
  // With `tui-slow-start`, the rest of its start waits half a second, as on a loaded runner (ESC-125).
  if (fs.existsSync(side('tui-slow-start'))) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 500);
  // With `tui-paste-late`, as the real one under Windows while it starts (ESC-12): it is listed in the registry, then
  // turns bracketed paste off for a while. Without it, a paste is only keys: its line break is Enter.
  let pasteMode = true;
  if (fs.existsSync(side('tui-paste-late'))) {
    pasteMode = false;
    process.stdout.write('\x1b[?2004l');
    setTimeout(() => {
      pasteMode = true;
      process.stdout.write('\x1b[?2004h');
    }, 1500);
  }
  // The session's file is made with its first message, not before.
  let begun = false;
  const begin = () => {
    if (begun) return;
    begun = true;
    fs.mkdirSync(path.dirname(file), { recursive: true });
    write({ type: 'permission-mode', permissionMode: argv[argv.indexOf('--permission-mode') + 1] ?? 'default' });
  };
  // The size of its terminal, as Claude Code's interface reads it to draw (ink: process.stdout.rows): each one it had is added
  // to <dir>/fake-tui-size.jsonl. Read from the console itself, not from the 'resize' event, which a pseudo-console may not send.
  let size = '';
  const reportSize = () => {
    const [cols, rows] = process.stdout.getWindowSize?.() ?? [process.stdout.columns, process.stdout.rows];
    if (`${cols}x${rows}` === size) return;
    size = `${cols}x${rows}`;
    // The account's folder may not be there yet: the session's file is made with its first message.
    fs.mkdirSync(configDir, { recursive: true });
    fs.appendFileSync(path.join(configDir, 'fake-tui-size.jsonl'), JSON.stringify({ cols, rows }) + '\n');
  };
  reportSize();
  setInterval(reportSize, 50).unref();
  const remote = flag('--remote-control');
  if (remote !== undefined && fs.existsSync(side('tui-rc-exit'))) setTimeout(() => process.exit(1), 200);
  const turn = async (prompt) => {
    if (prompt === '/exit') process.exit(0);
    begin();
    // What its screen shows of the turn that starts (the terminal's output the app keeps).
    process.stdout.write(`fake-turn ${prompt}\r\n`);
    status('busy');
    await hook('UserPromptSubmit', { prompt });
    // "/compact" as Claude Code 2.1.302 writes it (ESC-12): the text typed, the command, then its output; no hook, no
    // `turn_duration`. "/compact echec": its failure, as a `local_command` entry.
    if (prompt.startsWith('/compact')) {
      const promptId = `p-${process.pid}-${n}`;
      const args = prompt.slice('/compact'.length).trim();
      write({ type: 'user', uuid: `u-${process.pid}-${n++}`, promptId, message: { role: 'user', content: prompt } });
      write({
        type: 'user',
        uuid: `c-${process.pid}-${n++}`,
        promptId,
        isMeta: true,
        message: {
          role: 'user',
          content: '<local-command-caveat>The command below was run directly in Claude Code.</local-command-caveat>',
        },
      });
      write({
        type: 'user',
        uuid: `m-${process.pid}-${n++}`,
        promptId,
        message: {
          role: 'user',
          content: `<command-name>/compact</command-name>\n            <command-message>compact</command-message>\n            <command-args>${args}</command-args>`,
        },
      });
      if (args === 'echec') {
        write({
          type: 'system',
          subtype: 'local_command',
          content:
            '<local-command-stderr>Error during compaction: API Error: Output blocked by content filtering policy</local-command-stderr>',
          commandOutcome: { kind: 'failed' },
        });
      } else {
        write({ type: 'system', subtype: 'compact_boundary', content: 'Conversation compacted', compactMetadata: { trigger: 'manual' } });
        write({
          type: 'user',
          uuid: `s-${process.pid}-${n++}`,
          isCompactSummary: true,
          message: { role: 'user', content: 'This session is being continued…' },
        });
        write({
          type: 'user',
          uuid: `o-${process.pid}-${n++}`,
          message: {
            role: 'user',
            content: '<local-command-stdout>\x1b[2mCompacted (ctrl+o to see full summary)\x1b[22m</local-command-stdout>',
          },
        });
      }
      return status('idle');
    }
    write({ type: 'user', uuid: `u-${process.pid}-${n++}`, origin: { kind: 'human' }, message: { role: 'user', content: prompt } });
    if (prompt.includes('permission')) {
      const input = { command: 'echo tui' };
      const id = `tool-${process.pid}-${n}`;
      assistant([{ type: 'tool_use', id, name: 'Bash', input }]);
      await hook('PreToolUse', { tool_name: 'Bash', tool_input: input });
      await hook('PermissionRequest', { tool_name: 'Bash', tool_input: input });
      await new Promise((resolve) => (waiting = resolve));
      write({
        type: 'user',
        uuid: `r-${id}`,
        message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: id, content: 'tui' }] },
        toolUseResult: { stdout: 'tui', stderr: '' },
      });
      await hook('PostToolUse', { tool_name: 'Bash', tool_input: input });
    }
    if (prompt.includes('lent')) {
      const stopped = await new Promise((resolve) => (waiting = resolve));
      if (stopped === 'escape') {
        write({
          type: 'user',
          uuid: `i-${process.pid}-${n++}`,
          message: { role: 'user', content: [{ type: 'text', text: '[Request interrupted by user]' }] },
        });
        return status('idle');
      }
    }
    // Ending with "pense": it thinks until Escape, which cancels the turn as the real one does before its first answer (ESC-12):
    // nothing in the transcript, no hook, its registry says idle, and the prompt back in its line (before what was typed
    // meanwhile).
    if (prompt.endsWith('pense')) {
      await new Promise((resolve) => (waiting = resolve));
      line = prompt + line;
      draw();
      return status('idle');
    }
    // "commande de fond Ns": a `Bash` with `run_in_background`, as a real 2.1.306 interface writes it, Remote Control or not
    // (ESC-138): its result with `backgroundTaskId` at once, the end of the turn, then N seconds later the notification it
    // replays to Claude and the turn it starts on it.
    const bgTui = prompt.match(/commande de fond (\d+)s/);
    if (bgTui) {
      const id = `tool-${process.pid}-${n}`;
      const task = `b${process.pid}${n}`;
      const description = 'Wait until PR 148 checks finish';
      const input = { command: 'gh pr checks 148 --watch', description, run_in_background: true };
      assistant([{ type: 'tool_use', id, name: 'Bash', input }]);
      await hook('PreToolUse', { tool_name: 'Bash', tool_input: input });
      const out = path.join(os.tmpdir(), `${task}.output`);
      write({
        type: 'user',
        uuid: `r-${id}`,
        message: {
          role: 'user',
          content: [
            {
              type: 'tool_result',
              tool_use_id: id,
              content: `Command running in background with ID: ${task}. Output is being written to: ${out}. You will be notified when it completes.`,
            },
          ],
        },
        toolUseResult: { stdout: '', stderr: '', interrupted: false, isImage: false, noOutputExpected: false, backgroundTaskId: task },
      });
      await hook('PostToolUse', { tool_name: 'Bash', tool_input: input });
      assistant([{ type: 'text', text: 'lancé' }]);
      await hook('Stop', { stop_hook_active: false });
      write({ type: 'system', subtype: 'turn_duration', durationMs: 12, isSidechain: false });
      status('idle');
      setTimeout(
        async () => {
          write({
            type: 'user',
            uuid: `n-${id}`,
            origin: { kind: 'task-notification', producer: 'session-task' },
            message: {
              role: 'user',
              content: `<task-notification>\n<task-id>${task}</task-id>\n<tool-use-id>${id}</tool-use-id>\n<output-file>${out}</output-file>\n<status>completed</status>\n<summary>Background command "${description}" completed (exit code 0)</summary>\n</task-notification>`,
            },
          });
          status('busy');
          assistant([{ type: 'text', text: 'Les checks sont finis.' }]);
          await hook('Stop', { stop_hook_active: false });
          write({ type: 'system', subtype: 'turn_duration', durationMs: 12, isSidechain: false });
          status('idle');
        },
        Number(bgTui[1]) * 1000,
      );
      return;
    }
    // "tard": its registry says it idle a second before its transcript ends the turn.
    if (prompt.includes('tard')) {
      status('idle');
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
    assistant([{ type: 'text', text: `Fini : ${prompt}` }]);
    await hook('Stop', { stop_hook_active: false });
    write({ type: 'system', subtype: 'turn_duration', durationMs: 12, isSidechain: false });
    status('idle');
  };
  let pasteStart = null;
  let images = 0;
  let ctrlC = 0;
  const draw = () => process.stdout.write(`\r\x1b[2K> ${line}`);
  // What the keys typed do to the line: a character is added, Backspace takes the last one off, Ctrl+U clears it.
  const edit = (keys) => {
    for (const c of keys) {
      if (c === '\x7f' || c === '\b') line = line.slice(0, -1);
      else if (c === '\x15') line = '';
      else line += c;
    }
  };
  process.stdin.on('data', (chunk) => {
    for (const part of chunk.split(/(\x1b\[20[01]~|\r|\x1b|\x03)/)) {
      // Ctrl+C empties a line that holds something, as Claude Code does; on an empty one, the second quits.
      if (part === '\x03') {
        fs.appendFileSync(side('quit.jsonl'), JSON.stringify({ at: Date.now(), line }) + '\n');
        if (line) {
          line = '';
          draw();
        } else if (++ctrlC >= 2 && !fs.existsSync(side('tui-ignore-quit'))) process.exit(0);
        continue;
      }
      if (!pasteMode && (part === '\x1b[200~' || part === '\x1b[201~')) continue;
      if (part === '\x1b[200~') pasteStart = line.length;
      if (part === '\x1b[201~' && pasteStart !== null && pasteStart <= line.length) {
        const file = line
          .slice(pasteStart)
          .trim()
          .replace(/^"(.*)"$/, '$1');
        if (/\.(png|jpe?g|gif|webp)$/i.test(file) && fs.existsSync(file)) {
          const data = fs.readFileSync(file).toString('base64');
          fs.mkdirSync(configDir, { recursive: true });
          fs.appendFileSync(path.join(configDir, 'fake-tui-images.jsonl'), JSON.stringify({ path: file, data }) + '\n');
          line = line.slice(0, pasteStart) + `[Image #${++images}]`;
        }
        pasteStart = null;
      }
      if (part === '\x1b[201~') {
        // The paste is over (also one a line break in it cut short): the line, drawn once.
        pasteStart = null;
        draw();
      }
      if (part === '\x1b[200~' || part === '\x1b[201~' || part === '') continue;
      if (part === '\r' && pasteStart !== null) {
        // A line break pasted (\r, as a terminal pastes it) stays in the line; a \n pasted is dropped, as Claude Code
        // does under Windows.
        line += '\n';
      } else if (part === '\x1b') {
        if (waiting) [waiting, (waiting = null)][0]('escape');
      } else if (part === '\r') {
        if (waiting) {
          [waiting, (waiting = null)][0]('enter');
        } else if (line.trim()) {
          const prompt = line.trim();
          line = '';
          draw();
          turn(prompt);
        }
      } else if (pasteStart !== null) line += part.replaceAll('\n', '');
      else {
        edit(part);
        draw();
      }
    }
  });
  // Up and reading its input: registered, as Claude Code 2.1.301 does once it is past its welcome
  // screen and the box about the folder (the one signal an interface no window shows is ready by),
  // then Remote Control opened, if it was started with it.
  const registry = configDir ? path.join(configDir, 'sessions', `${process.pid}.json`) : null;
  const registered = {
    pid: process.pid,
    sessionId,
    cwd: process.cwd(),
    startedAt: Date.now(),
    version: '2.1.301',
    kind: 'interactive',
    entrypoint: 'cli',
    status: 'idle',
    statusUpdatedAt: Date.now(),
  };
  let listed = null;
  const register = (extra = {}) => {
    if (!registry) return;
    listed = { ...listed, ...extra };
    fs.mkdirSync(path.dirname(registry), { recursive: true });
    fs.writeFileSync(registry, JSON.stringify({ ...registered, ...listed }));
  };
  // Busy during a turn, idle after, as Claude Code says in its registry file (once it is listed there).
  status = (now) => {
    registered.status = now;
    registered.statusUpdatedAt = Date.now();
    if (listed) register();
  };
  process.on('exit', () => {
    if (registry) fs.rmSync(registry, { force: true });
  });
  const until = async (kind) => {
    while (fs.existsSync(side(kind))) await new Promise((resolve) => setTimeout(resolve, 50));
  };
  (async () => {
    await until('tui-welcome');
    register();
    if (remote === undefined) return;
    await until('tui-rc-hold');
    // Remote Control makes the session's file, as Claude Code does, before any message.
    begin();
    const url = `https://claude.ai/code/session_fake_tui_${process.pid}`;
    if (fs.existsSync(side('tui-rc-inactive'))) {
      write({ type: 'system', subtype: 'bridge_status', content: '/remote-control is not active' });
      return;
    }
    const bridgeSessionId = `cse_fake_tui_${process.pid}`;
    register({ bridgeSessionId });
    if (fs.existsSync(side('tui-rc-registry-only'))) return;
    write({ type: 'bridge-session', bridgeSessionId });
    write({ type: 'system', subtype: 'bridge_status', content: `/remote-control is active · ${url}`, url });
  })();
}

function startSession() {
  if (configDir && fs.existsSync(path.join(configDir, 'fake-exit'))) {
    process.stderr.write(fs.readFileSync(path.join(configDir, 'fake-exit'), 'utf8'));
    process.exit(1);
  }
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
  let floodTimer = null;
  let asked = false;
  // The spec kept once ([spec:<project>], ESC-263).
  let specKept = false;
  // The subagent of "sous-agent long" while it runs, until a `stop_task` of its task: { task, call, description }.
  let longAgent = null;
  const replay = argv.includes('--replay-user-messages');
  const appended = argv.includes('--append-system-prompt') ? (argv[argv.indexOf('--append-system-prompt') + 1] ?? '') : '';
  // What a copy of an agent is told of its folder, what every agent is told of the shared memory (ESC-255) or a discussion
  // of the Docs tab of its role (ESC-261) is no ticket's protocol: it plays a plain agent.
  const copied = [
    'Cette conversation a été copiée',
    'This conversation was copied',
    'Le projet a une mémoire partagée',
    'The project has a memory shared',
    // A discussion of the Docs tab (ESC-261): its role, no ticket.
    'Tu es l’agent rédacteur de spec',
    'Tu es l’agent produit',
    'You are the spec writer',
    'You are the product agent',
  ];
  const sys = copied.some((c) => appended.startsWith(c)) ? '' : appended;
  let remoteSent = false;
  let entries = 0;
  let slowClose = false;

  const out = (o) => process.stdout.write(JSON.stringify(o) + '\n');
  const ok = (id, response = {}) => out({ type: 'control_response', response: { subtype: 'success', request_id: id, response } });
  // While <log>.<kind>-slow is there, holding a number of milliseconds, `later` runs `then` that
  // long after (kind: 'initialize', 'remote'): a request whose answer takes its time.
  const later = (kind, then) => {
    let ms = 0;
    try {
      ms = Number(fs.readFileSync(logFile.replace(/\.jsonl$/, '') + `.${kind}-slow`, 'utf8')) || 0;
    } catch {}
    return ms ? setTimeout(then, ms) : then();
  };

  // An entry of the session, kept in the account's folder when it has one.
  function keep(entry) {
    if (!configDir) return;
    const file = sessionFile(sessionId);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.appendFileSync(file, JSON.stringify({ ...entry, sessionId, cwd: process.cwd() }) + '\n');
  }

  // The quota windows it tells: full while the account is out of quota; ending from now once [quota-rythme] was asked,
  // by this process or, through <dir>/fake-paced, by another one of the account (a reading of its quota is a process of
  // its own).
  let pacedHere = false;
  const pacedFile = () => (configDir === null ? null : path.join(configDir, 'fake-paced'));
  const isPaced = () => pacedHere || (pacedFile() !== null && fs.existsSync(pacedFile()));
  const pace = () => {
    pacedHere = true;
    if (pacedFile()) fs.writeFileSync(pacedFile(), '');
  };
  // While a <dir>/fake-overage file is there, the account answers as extra usage (ESC-147), its 5-hour window full.
  const overageOn = () => configDir !== null && fs.existsSync(path.join(configDir, 'fake-overage'));
  // While a <dir>/fake-pct file is there, the 5-hour window says the percentage it holds, until in an hour (ESC-148);
  // <dir>/fake-week-pct, the weekly window, until in a day.
  const pctOf = (name = 'fake-pct') => {
    const file = configDir === null ? null : path.join(configDir, name);
    return file && fs.existsSync(file) ? Number(fs.readFileSync(file, 'utf8')) / 100 : null;
  };
  const windows = () => {
    const now = Math.floor(Date.now() / 1000);
    const pct = pctOf();
    return {
      five_hour: overageOn()
        ? { utilization: 1, resetsAt: now + 3600 }
        : pct !== null
          ? { utilization: pct, resetsAt: now + 3600 }
          : { utilization: limited() ? 1 : isPaced() ? 0.5 : 0.12, resetsAt: isPaced() ? now + 3 * 3600 : 1790558400 },
      seven_day:
        pctOf('fake-week-pct') !== null
          ? { utilization: pctOf('fake-week-pct'), resetsAt: now + 24 * 3600 }
          : { utilization: 0.34, resetsAt: isPaced() ? now + 84 * 3600 : 1790805600 },
    };
  };

  // `parent`: the call of the subagent that makes this message (null: the main thread).
  function assistant(block, id = `msg_${process.pid}_${++msg}`, parent = null) {
    const uuid = `entry-${process.pid}-${++entries}`;
    const message = { id, role: 'assistant', content: [block] };
    out({ type: 'assistant', message, parent_tool_use_id: parent, session_id: sessionId, uuid });
    if (!parent) keep({ type: 'assistant', uuid, message });
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
    const overage = overageOn();
    out({
      type: 'rate_limit_event',
      rate_limit_info: overage
        ? { status: 'allowed_warning', isUsingOverage: true, overageStatus: 'allowed', unifiedWindows: windows() }
        : { unifiedWindows: windows() },
    });
  }

  // AskUserQuestion; once answered, `then` goes on with the turn (else a plain reply ends it). `questions`: its own
  // (else one question of two choices).
  function askQuestion(then = null, questions = null) {
    const tuid = `toolu_q${msg}`;
    assistant({ type: 'tool_use', id: tuid, name: 'AskUserQuestion', input: {} });
    const input = questions
      ? { questions }
      : {
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

  // Escouade's tools, called as Claude Code would through the server its --mcp-config file names.
  async function escouadeTool(name, args) {
    const file = argv[argv.indexOf('--mcp-config') + 1];
    const server = JSON.parse(fs.readFileSync(file, 'utf8')).mcpServers.escouade;
    let session = null;
    const post = async (body) => {
      const headers = { ...server.headers, 'Content-Type': 'application/json', Accept: 'application/json, text/event-stream' };
      if (session) headers['Mcp-Session-Id'] = session;
      const r = await fetch(server.url, { method: 'POST', headers, body: JSON.stringify(body) });
      session = r.headers.get('mcp-session-id') ?? session;
      const t = await r.text();
      return t ? JSON.parse(t.startsWith('{') ? t : (t.match(/^data: (.*)$/m)?.[1] ?? 'null')) : null;
    };
    await post({
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'fake-claude', version: '1' } },
    });
    await post({ jsonrpc: '2.0', method: 'notifications/initialized' });
    const done = await post({ jsonrpc: '2.0', id: 2, method: 'tools/call', params: { name, arguments: args } });
    if (done?.error || done?.result?.isError) throw new Error(`${name}: ${JSON.stringify(done)}`);
    return done?.result;
  }

  async function keepSpec(project, key) {
    await escouadeTool('write_spec_page', {
      project,
      path: 'paiement/echeances',
      title: 'Échéances',
      body: `# Échéances\n\nQuatre échéances au plus (${key}). [écart]`,
    });
    await escouadeTool('propose_decision', {
      project,
      title: `Quatre échéances (${key})`,
      context: 'Les clients demandent une échéance de plus.',
      decision: 'Quatre échéances au plus.',
      consequences: 'L’échéancier change.',
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
    const facts = /faits-workflow:\s*(\[[^\n]*\])/.exec(all);
    if (facts) {
      workflowFacts(JSON.parse(facts[1]), false, true).catch((e) => process.stderr.write(`faits-workflow: ${e.stack}\n`));
      return;
    }
    if (all.includes('[lent]')) {
      streamText('Je commence…');
      slowTimer = setTimeout(() => result(), 30_000);
      return;
    }
    // [spec:<project>] (ESC-263): a ticket that changes how the product behaves keeps the spec, as its protocol asks,
    // once: the page paiement/echeances written and a decision left by Escouade's docs tools.
    const specProject = all.match(/\[spec:([^\]]+)\]/)?.[1];
    if (specProject && !specKept) {
      specKept = true;
      keepSpec(specProject, key)
        .catch((e) => process.stderr.write(`spec: ${e.stack}\n`))
        .finally(() => ticketTurn(text));
      return;
    }
    const file = `${key.toLowerCase()}.txt`;
    // [dans:api]: in that folder of its own (a repository of a project of several, ESC-169), each one named.
    const into = [...all.matchAll(/\[dans:([^\]]+)\]/g)].map((m) => m[1]);
    if (!all.includes('[rien]'))
      for (const dir of into.length ? into : ['.']) fs.writeFileSync(path.join(process.cwd(), dir, file), `Boucle ${loop}\n`);
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

  // ---------- "flood" ----------

  // A busy agent for `ms`: ten deltas of 40 characters (one every 15 ms) make a message, a tool call comes after it
  // and its output (~1.5 KB) on the next tick, and so on. One timer paces it; an interrupt clears it (`floodTimer`).
  function flood(ms) {
    clearInterval(floodTimer);
    const end = Date.now() + ms;
    const words = 'lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore ';
    const output = Array.from({ length: 24 }, (_, i) => `ligne ${String(i + 1).padStart(2, '0')} : ${'abcdefghij'.repeat(5)}`).join('\n');
    const ev = (event) => out({ type: 'stream_event', event, parent_tool_use_id: null, session_id: sessionId });
    let id = null;
    let text = '';
    let call = null;
    let calls = 0;
    floodTimer = setInterval(() => {
      if (Date.now() >= end) {
        clearInterval(floodTimer);
        result();
        return;
      }
      if (call) {
        toolResult(call, output, { tool_use_result: { stdout: output, stderr: '', interrupted: false } });
        call = null;
        return;
      }
      if (id === null) {
        id = `msg_${process.pid}_${++msg}`;
        text = '';
        ev({
          type: 'message_start',
          message: { id, usage: { input_tokens: 10, cache_read_input_tokens: 1000, cache_creation_input_tokens: 0 } },
        });
        ev({ type: 'content_block_start', index: 0, content_block: { type: 'text', text: '' } });
      }
      const from = (text.length * 7 + calls) % (words.length - 40);
      const delta = words.slice(from, from + 40);
      text += delta;
      ev({ type: 'content_block_delta', index: 0, delta: { type: 'text_delta', text: delta } });
      if (text.length < 400) return;
      assistant({ type: 'text', text }, id);
      ev({ type: 'content_block_stop', index: 0 });
      ev({ type: 'message_stop' });
      id = null;
      call = `toolu_f${process.pid}_${++calls}`;
      const command = `npm test -- --shard ${calls}`;
      assistant({ type: 'tool_use', id: call, name: 'Bash', input: { command, description: command } });
    }, 15);
  }

  // ---------- "flood-real" ----------

  // A busy agent as the superpowers skills drive one, for `ms`: a plan of 40 tasks (TaskCreate), then task after task its
  // TaskUpdate, its own messages and tool calls and an implementer's and a reviewer's subagent (frames with a
  // `parent_tool_use_id`), the results of 2 to 30 KB. Every message streams in small deltas, every step is a timer: ~50
  // stream lines a second, ~8 of which change the agent's view. An interrupt clears the timer: the chain below is dropped.
  async function floodReal(ms) {
    clearTimeout(floodTimer);
    const end = Date.now() + ms;
    const over = new Error('flood-real over');
    const sleep = (t) => new Promise((resolve, reject) => (Date.now() >= end ? reject(over) : (floodTimer = setTimeout(resolve, t))));
    const rnd = (a, b) => a + Math.floor(Math.random() * (b - a + 1));
    const pick = (list) => list[rnd(0, list.length - 1)];
    const prose =
      'lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua '.repeat(
        30,
      );
    const said = (n) => {
      const from = rnd(0, 100);
      return prose.slice(from, from + n);
    };
    const big = Array.from({ length: 320 }, (_, i) => `${String(i + 1).padStart(4, '0')}: ${said(90)}`).join('\n');
    const output = () => big.slice(0, rnd(2 * 1024, 30 * 1024));
    const ev = (parent, event) => out({ type: 'stream_event', event, parent_tool_use_id: parent, session_id: sessionId });
    const system = (o) => out({ type: 'system', session_id: sessionId, ...o });
    const dir = process.cwd();
    const files = Array.from({ length: 12 }, (_, i) => `src/module${i + 1}.ts`);
    let calls = 0;

    // One assistant message in the thread of `parent` (null: the main one): `chars` of text in small deltas, then the call of
    // the tool `name` (none: the message ends). Returns the call's id.
    const message = async (parent, chars, name, input) => {
      const id = `msg_${process.pid}_${++msg}`;
      const call = `toolu_r${process.pid}_${++calls}`;
      const usage = { input_tokens: rnd(3, 30), cache_read_input_tokens: rnd(20000, 90000), cache_creation_input_tokens: rnd(0, 900) };
      ev(parent, { type: 'message_start', message: { id, usage } });
      ev(parent, { type: 'content_block_start', index: 0, content_block: { type: 'text', text: '' } });
      let text = '';
      while (text.length < chars) {
        const delta = said(rnd(15, 45));
        text += delta;
        ev(parent, { type: 'content_block_delta', index: 0, delta: { type: 'text_delta', text: delta } });
        await sleep(rnd(12, 28));
      }
      assistant({ type: 'text', text }, id, parent);
      ev(parent, { type: 'content_block_stop', index: 0 });
      if (name) {
        const json = JSON.stringify(input);
        const cut = json.length >> 1;
        ev(parent, { type: 'content_block_start', index: 1, content_block: { type: 'tool_use', id: call, name } });
        ev(parent, { type: 'content_block_delta', index: 1, delta: { type: 'input_json_delta', partial_json: json.slice(0, cut) } });
        ev(parent, { type: 'content_block_delta', index: 1, delta: { type: 'input_json_delta', partial_json: json.slice(cut) } });
        assistant({ type: 'tool_use', id: call, name, input }, id, parent);
        ev(parent, { type: 'content_block_stop', index: 1 });
      }
      ev(parent, {
        type: 'message_delta',
        delta: { stop_reason: name ? 'tool_use' : 'end_turn' },
        usage: { output_tokens: Math.ceil(text.length / 4) + 50 },
      });
      ev(parent, { type: 'message_stop' });
      return call;
    };

    // A message that calls a tool, and the tool's result a moment later.
    const run = async (parent, chars, name, input, content, extra = {}) => {
      const call = await message(parent, chars, name, input);
      await sleep(rnd(60, 250));
      toolResult(call, content, { parent_tool_use_id: parent, ...extra });
    };

    // A tool of the agent's work: a Read, a Grep, an Edit or a Bash, and its result.
    const work = (parent) => {
      const chars = parent ? rnd(100, 400) : rnd(200, 600);
      const file = path.join(dir, pick(files));
      const content = output();
      switch (rnd(0, 3)) {
        case 0:
          return run(parent, chars, 'Read', { file_path: file }, content);
        case 1:
          return run(parent, chars, 'Grep', { pattern: said(12).trim(), path: dir }, content);
        case 2:
          return run(parent, chars, 'Edit', { file_path: file, old_string: said(120), new_string: said(160) }, content, {
            tool_use_result: {
              filePath: file,
              structuredPatch: [{ oldStart: 10, oldLines: 3, newStart: 10, newLines: 4, lines: [' a', '-b', '+c', '+d'] }],
            },
          });
        default: {
          const command = `npm test -- --shard ${rnd(1, 9)}`;
          return run(parent, chars, 'Bash', { command, description: command }, content, {
            tool_use_result: { stdout: content, stderr: '', interrupted: false },
          });
        }
      }
    };

    // A subagent in the foreground: launched by the main thread, its messages and tools in its own thread, its result.
    const subagent = async (n, review) => {
      const description = `${review ? 'Relire' : 'Implémenter'} la tâche ${n}`;
      const prompt = `Lis .superpowers/sdd/demo/task-${n}-brief.md et fais ce qu'il demande.\n${said(rnd(600, 2500))}`;
      const agent = { description, subagent_type: 'general-purpose', model: review ? 'sonnet' : 'haiku', prompt };
      const id = await message(null, rnd(150, 400), 'Agent', agent);
      system({
        subtype: 'task_started',
        task_id: `fakeflood-${id}`,
        tool_use_id: id,
        description,
        subagent_type: agent.subagent_type,
        is_backgrounded: false,
        spawn_depth: 1,
        task_type: 'local_agent',
        prompt,
        run_id: `run-${id}`,
      });
      const tools = rnd(3, 7);
      for (let i = 0; i < tools; i++) await work(id);
      await message(id, rnd(100, 300), null);
      const content = [{ type: 'text', text: `${description} : fait. ${said(400)}` }];
      toolResult(id, content, {
        tool_use_result: {
          status: 'completed',
          agentId: `fakeflood-${id}`,
          content,
          totalToolUseCount: tools,
          totalDurationMs: tools * 400,
          totalTokens: tools * 1800,
        },
      });
    };

    const titles = Array.from({ length: 40 }, (_, i) => `Tâche ${i + 1} : ${said(30).trim()}`);
    const update = (n, from, to) =>
      run(null, rnd(40, 120), 'TaskUpdate', { taskId: String(n), status: to }, `Updated task #${n} status`, {
        tool_use_result: { success: true, taskId: String(n), updatedFields: ['status'], statusChange: { from, to } },
      });
    try {
      for (;;) {
        for (const [i, subject] of titles.entries()) {
          const input = { subject, description: said(rnd(100, 400)), activeForm: `Fait la tâche ${i + 1}` };
          await run(null, rnd(20, 60), 'TaskCreate', input, `Task #${i + 1} created successfully: ${subject}`, {
            tool_use_result: { task: { id: String(i + 1), subject } },
          });
        }
        for (let n = 1; n <= titles.length; n++) {
          await update(n, 'pending', 'in_progress');
          await work(null);
          await subagent(n, false);
          for (let i = rnd(1, 2); i > 0; i--) await work(null);
          await subagent(n, true);
          await work(null);
          await update(n, 'in_progress', 'completed');
        }
      }
    } catch (e) {
      if (e !== over) throw e;
    }
    streamText('Plan terminé');
    result();
  }

  // ---------- "plan superpowers" ----------

  let planCut = false;

  // A pause of a few dozen ms between frames, so that the app sees them one by one; it ends the run
  // when an interrupt came meanwhile.
  const beat = (ms = 25) =>
    new Promise((resolve, reject) => setTimeout(() => (planCut ? reject(new Error('interrupted')) : resolve()), ms));

  async function planRun(text) {
    planCut = false;
    const withList = !text.includes('sans liste');
    const cutShort = text.includes('interrompu');
    const dir = process.cwd();
    const planRel = 'docs/superpowers/plans/2026-10-10-demo.md';
    const space = '.superpowers/sdd/2026-10-10-demo';
    const ledger = path.join(dir, space, 'progress.md');
    const titles = ['Écrire la fonction', 'Écrire les tests', 'Brancher dans l’interface'];
    const doing = ['Écrit la fonction', 'Écrit les tests', 'Branche dans l’interface'];
    const file = (rel) => path.join(dir, rel);
    const put = (rel, content) => {
      fs.mkdirSync(path.dirname(file(rel)), { recursive: true });
      fs.writeFileSync(file(rel), content);
    };
    // A call of the main thread and its result.
    const tool = async (id, name, input, content, extra = {}) => {
      assistant({ type: 'tool_use', id, name, input });
      await beat();
      toolResult(id, content, extra);
      await beat();
    };
    const bash = (id, command, content = 'ok') =>
      tool(id, 'Bash', { command, description: command }, content, {
        tool_use_result: { stdout: content, stderr: '', interrupted: false },
      });
    // A subagent in the foreground: its own calls, in its thread, then its result. False: the turn
    // was cut in the middle of it.
    const subagent = async (id, description, n, calls) => {
      const prompt = `Lis ${space}/task-${n}-brief.md et fais ce qu'il demande.`;
      const model = description.startsWith('Relire') ? 'sonnet' : 'haiku';
      assistant({ type: 'tool_use', id, name: 'Agent', input: { description, subagent_type: 'general-purpose', model, prompt } });
      await beat();
      for (const [i, [name, input]] of calls.entries()) {
        assistant({ type: 'tool_use', id: `${id}_${i}`, name, input }, undefined, id);
        await beat();
        if (cutShort && n === 1 && i === 1) return false;
        toolResult(`${id}_${i}`, 'ok', { parent_tool_use_id: id });
        await beat();
      }
      const content = [{ type: 'text', text: `${description} : fait.` }];
      const done = {
        status: 'completed',
        agentId: `agent-${id}`,
        content,
        totalToolUseCount: calls.length,
        totalDurationMs: 90,
        totalTokens: 1500,
      };
      toolResult(id, content, { tool_use_result: done });
      await beat();
      return true;
    };
    const system = (o) => out({ type: 'system', session_id: sessionId, ...o });

    streamText('J’écris le plan, puis je l’exécute tâche par tâche.');
    await beat();
    const plan = [
      '# Démo Implementation Plan',
      '',
      '> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development.',
      '',
      ...titles.flatMap((title, i) => [`### Task ${i + 1}: ${title}`, '', '- [ ] **Step 1: Écrire**', '- [ ] **Step 2: Vérifier**', '']),
    ].join('\n');
    put(planRel, plan);
    await tool('toolu_sp_plan', 'Write', { file_path: file(planRel), content: plan }, `File created successfully at: ${file(planRel)}`, {
      tool_use_result: { type: 'create', filePath: file(planRel), content: plan, structuredPatch: [] },
    });
    // What sdd-workspace and the controller make: the marker and the ledger.
    put(`${space}/plan-path`, `${planRel}\n`);
    put(`${space}/progress.md`, `# SDD ledger — plan: ${planRel}\n`);
    await bash('toolu_sp_ws', `sdd-workspace ${planRel}`, space);
    if (withList) {
      for (const [i, title] of titles.entries()) {
        await tool(
          `toolu_sp_tc${i + 1}`,
          'TaskCreate',
          { subject: title, description: `Task ${i + 1} du plan`, activeForm: doing[i] },
          `Task #${i + 1} created successfully: ${title}`,
          { tool_use_result: { task: { id: String(i + 1), subject: title } } },
        );
      }
    }
    const status = (n, from, to) => ({
      tool_use_result: { success: true, taskId: String(n), updatedFields: ['status'], statusChange: { from, to } },
    });
    try {
      for (const [i, title] of titles.entries()) {
        const n = i + 1;
        if (withList) {
          await tool(
            `toolu_sp_ts${n}`,
            'TaskUpdate',
            { taskId: String(n), status: 'in_progress' },
            `Updated task #${n} status`,
            status(n, 'pending', 'in_progress'),
          );
        }
        put(`${space}/task-${n}-brief.md`, `### Task ${n}: ${title}\n\n- [ ] **Step 1: Écrire**\n`);
        await bash(`toolu_sp_st${n}`, `task-start ${planRel} ${n}`, `${space}/task-${n}-brief.md`);
        if (n === 3) {
          // The question the third task waits on.
          await new Promise((resolve) => askQuestion(resolve));
          await beat();
        }
        const read = ['Read', { file_path: file('src/slugify.ts') }];
        const edit = ['Edit', { file_path: file('src/slugify.ts'), old_string: 'a', new_string: 'b' }];
        const test = ['Bash', { command: 'npm test', description: 'npm test' }];
        if (!(await subagent(`toolu_sp_im${n}`, `Implémenter la tâche ${n}`, n, [read, edit, test]))) {
          result({ isError: true, subtype: 'error_during_execution' });
          return;
        }
        await subagent(`toolu_sp_rv${n}`, `Relire la tâche ${n}`, n, [read, ['Bash', { command: 'git diff', description: 'git diff' }]]);
        if (n === 2) {
          // A subagent that goes on after the turn, as Claude Code 2.1.296 launches every subagent (measured):
          // no `run_in_background` in the call, `async_launched` in its result, `task_started` between the two.
          // And a workflow that ends within the turn.
          const bg = {
            description: 'Surveiller la suite de tests',
            subagent_type: 'general-purpose',
            model: 'haiku',
            prompt: 'Surveille les tests.',
          };
          assistant({ type: 'tool_use', id: 'toolu_sp_bg', name: 'Agent', input: bg });
          await beat();
          system({
            subtype: 'task_started',
            task_id: 'fakebg-agent',
            tool_use_id: 'toolu_sp_bg',
            description: bg.description,
            subagent_type: bg.subagent_type,
            is_backgrounded: true,
            spawn_depth: 1,
            task_type: 'local_agent',
            prompt: bg.prompt,
            run_id: 'fakerun-bg',
          });
          await beat();
          toolResult('toolu_sp_bg', 'Async agent launched successfully.\nagentId: fakebg-agent (internal ID)', {
            tool_use_result: {
              isAsync: true,
              status: 'async_launched',
              agentId: 'fakebg-agent',
              description: bg.description,
              resolvedModel: 'claude-haiku-5-5',
              prompt: bg.prompt,
              outputFile: 'fakebg-agent.output',
              canReadOutputFile: true,
              canContinueAgent: true,
            },
          });
          await beat();
          // What it does, told as it goes (measured: one `task_progress` per tool it uses).
          system({
            subtype: 'task_progress',
            task_id: 'fakebg-agent',
            tool_use_id: 'toolu_sp_bg',
            description: 'Running npm test',
            subagent_type: bg.subagent_type,
            usage: { total_tokens: 1200, tool_uses: 2, duration_ms: 300 },
            last_tool_name: 'Bash',
          });
          await beat();
          assistant({ type: 'tool_use', id: 'toolu_sp_wf', name: 'Workflow', input: { workflow: 'revue-finale' } });
          await beat();
          system({
            subtype: 'task_started',
            task_id: 'fakewf1',
            tool_use_id: 'toolu_sp_wf',
            description: 'Revue finale',
            task_type: 'local_workflow',
            workflow_name: 'revue-finale',
          });
          await beat();
          for (const [phase, tokens, uses] of [
            ['Relecture : sécurité', 300, 2],
            ['Relecture : style', 640, 5],
          ]) {
            const usage = { total_tokens: tokens, tool_uses: uses, duration_ms: uses * 20 };
            system({
              subtype: 'task_progress',
              task_id: 'fakewf1',
              tool_use_id: 'toolu_sp_wf',
              description: phase,
              usage,
              last_tool_name: phase.split(' : ')[1],
            });
            await beat();
          }
          system({
            subtype: 'task_notification',
            task_id: 'fakewf1',
            tool_use_id: 'toolu_sp_wf',
            status: 'completed',
            summary: 'Workflow "revue-finale" completed',
            usage: { total_tokens: 640, tool_uses: 5, duration_ms: 100 },
          });
          toolResult('toolu_sp_wf', 'Workflow revue-finale completed');
          await beat();
        }
        fs.appendFileSync(ledger, `Task ${n}: complete (commits a1b2c3${n}..d4e5f6${n}, tests: npm test → 12 passed, review clean)\n`);
        await bash(`toolu_sp_td${n}`, `task-done ${planRel} ${n} d4e5f6${n} -- npm test`, `recorded: Task ${n}`);
        if (withList) {
          await tool(
            `toolu_sp_tu${n}`,
            'TaskUpdate',
            { taskId: String(n), status: 'completed' },
            `Updated task #${n} status`,
            status(n, 'in_progress', 'completed'),
          );
        }
      }
    } catch (e) {
      // An interrupt came: the turn already ended with its answer.
      if (e.message === 'interrupted') return;
      throw e;
    }
    // The skills end a run by removing the plan's workspace, once the final review is clean.
    fs.rmSync(file(space), { recursive: true, force: true });
    await bash('toolu_sp_rm', `rm -rf ${space}`, '');
    streamText('Plan terminé');
    result();
    // The background subagent ends after the turn, and Claude Code tells it by itself.
    setTimeout(() => {
      system({
        subtype: 'task_updated',
        task_id: 'fakebg-agent',
        run_id: 'fakerun-bg',
        patch: { status: 'completed', end_time: Date.now() },
      });
      system({
        subtype: 'task_notification',
        task_id: 'fakebg-agent',
        run_id: 'fakerun-bg',
        tool_use_id: 'toolu_sp_bg',
        status: 'completed',
        output_file: 'fakebg-agent.output',
        summary: 'Agent "Surveiller la suite de tests" completed',
        usage: { total_tokens: 2200, tool_uses: 6, duration_ms: 800 },
        uuid: `bgagent-${msg}`,
      });
    }, 400);
  }

  // ---------- "/cost", "/cost echec", "/loop …" (ESC-154) ----------

  /** `text`, a `/` command, as Claude Code writes the command it runs: `<command-message>`, `<command-name>`, `<command-args>`. */
  function commandTags(text) {
    const m = text.match(/^\/([\w:-]+)\s*([\s\S]*)$/);
    if (!m) return null;
    return `<command-message>${m[1]}</command-message>\n<command-name>/${m[1]}</command-name>\n<command-args>${m[2].trim()}</command-args>`;
  }

  /** A message Claude Code passes on by itself (`--replay-user-messages`), with a uuid of its own. */
  function replayed(content, extra = {}) {
    out({
      type: 'user',
      message: { role: 'user', content },
      parent_tool_use_id: null,
      uuid: `replay-${process.pid}-${++msg}`,
      isReplay: true,
      session_id: sessionId,
      ...extra,
    });
  }

  // A local command, run by Claude Code itself: its output replayed, its lines and their ANSI codes as it writes them
  // (« /cost echec »: its error output), then the end of its turn.
  function localCommand(text) {
    replayed(
      '<local-command-caveat>Caveat: The messages below were generated by the user while running local commands. DO NOT respond to these messages or otherwise consider them in your response unless the user explicitly asks you to.</local-command-caveat>',
      { isMeta: true },
    );
    if (text.includes('echec')) replayed('<local-command-stderr>Erreur de /cost : \u001b[31mindisponible\u001b[39m</local-command-stderr>');
    else
      replayed(
        '<local-command-stdout>Total cost:            $0.05\n\u001b[2mTotal duration (API):  1.2s\u001b[22m\nUsage:                 100 input, 20 output</local-command-stdout>',
      );
    result();
  }

  // A dynamic `/loop` (2.1.309, measured): `ScheduleWakeup`, the turn ends; at the wake-up, Claude Code runs the command
  // again, replayed with no origin (a human's) and a uuid of its own, and the loop stops (`stop: true`).
  let woken = false;
  function loopTurn(text) {
    const args = text.slice('/loop'.length).trim();
    const tuid = `toolu_w${++msg}`;
    if (!woken) {
      woken = true;
      const at = Date.now() + 1500;
      assistant({
        type: 'tool_use',
        id: tuid,
        name: 'ScheduleWakeup',
        input: { delaySeconds: 60, reason: 'suivre le ticket', prompt: text },
      });
      toolResult(tuid, 'Next wakeup scheduled.', { tool_use_result: { scheduledFor: at, clampedDelaySeconds: 60, wasClamped: false } });
      streamText('Je reviens dans une minute.');
      result();
      setTimeout(() => {
        replayed(commandTags(text));
        loopTurn(text);
      }, 1500);
      return;
    }
    assistant({ type: 'tool_use', id: tuid, name: 'ScheduleWakeup', input: { stop: true, reason: 'fini' } });
    toolResult(tuid, 'Loop stopped.', {
      tool_use_result: { scheduledFor: 0, clampedDelaySeconds: 0, wasClamped: false, stopped: true, cancelledWakeups: 0 },
    });
    streamText(`Le ticket est revenu (${args}).`);
    result();
  }

  // ---------- "envoie des fichiers" (ESC-171) ----------

  /** A PNG of `w` × `h` pixels of one colour, made here (zlib), that a browser draws. */
  function png(w, h, [r, g, b]) {
    const crcTable = Array.from({ length: 256 }, (_, n) => {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      return c >>> 0;
    });
    const crc = (buf) => {
      let c = 0xffffffff;
      for (const x of buf) c = crcTable[(c ^ x) & 0xff] ^ (c >>> 8);
      return (c ^ 0xffffffff) >>> 0;
    };
    const chunk = (type, data) => {
      const len = Buffer.alloc(4);
      len.writeUInt32BE(data.length);
      const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
      const sum = Buffer.alloc(4);
      sum.writeUInt32BE(crc(body));
      return Buffer.concat([len, body, sum]);
    };
    const ihdr = Buffer.alloc(13);
    ihdr.writeUInt32BE(w, 0);
    ihdr.writeUInt32BE(h, 4);
    ihdr[8] = 8;
    ihdr[9] = 2;
    const row = Buffer.concat([Buffer.from([0]), Buffer.alloc(w * 3).map((_, i) => [r, g, b][i % 3])]);
    const raw = Buffer.concat(Array.from({ length: h }, () => row));
    return Buffer.concat([
      Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
      chunk('IHDR', ihdr),
      chunk('IDAT', zlib.deflateSync(raw)),
      chunk('IEND', Buffer.alloc(0)),
    ]);
  }

  function sendFiles(text) {
    const dir = path.join(process.cwd(), '.impeccable', 'tmp');
    fs.mkdirSync(dir, { recursive: true });
    const files = [
      ['vue-face.png', png(160, 100, [214, 120, 60])],
      ['vue-dessus.png', png(120, 120, [60, 130, 210])],
      ['contrat.pdf', Buffer.from('%PDF-1.4\n1 0 obj << >> endobj\ntrailer << >>\n%%EOF\n')],
    ].map(([name, bytes]) => {
      const file = path.join(dir, name);
      fs.writeFileSync(file, bytes);
      return file.replace(/\\/g, '/');
    });
    const id = `toolu_send${msg}`;
    const input = {
      files,
      caption: 'Les deux vues et le contrat.',
      status: text.includes('[proactif]') ? 'proactive' : 'normal',
      display: 'render',
    };
    assistant({ type: 'tool_use', id, name: 'SendUserFile', input });
    toolResult(
      id,
      `${files.length} files delivered to user.\n${files.map((f) => `${f} → file_uuid: 00000000-0000-0000-0000-00000000000${files.indexOf(f)}`).join('\n')}`,
    );
    streamText('Je t’ai envoyé les vues et le contrat.');
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

  // ---------- "sorties piégées" (ESC-290) ----------

  // A Bash call that read a binary file and a minified one: NUL and control characters, a terminal's escapes, a lone
  // half of a UTF-16 pair, then a line of 5 MB. What the page must show without drawing it raw nor whole.
  function trappedOutputs() {
    const wav = Buffer.alloc(4096);
    for (let i = 0; i < wav.length; i++) wav[i] = (i * 37 + 11) % 256;
    Buffer.from('RIFF\0\0\0\0WAVEfmt ', 'latin1').copy(wav);
    const binary = 'cat \x1b[31m' + wav.toString('latin1') + '\x1b[0m fin \ud83c';
    const id = 'toolu_trap_' + process.pid;
    assistant({ type: 'tool_use', id, name: 'Bash', input: { command: 'cat "bank 1.bnk" minified.json' } });
    toolResult(id, binary + '\n' + 'x'.repeat(5_000_000));
    streamText('Lu.');
    result();
  }

  // ---------- "faits-workflow" ----------

  // The facts of an agent in a workflow, one turn: see the head of this file.
  async function workflowFacts(list, fails, hold = false) {
    const wait = (ms) => new Promise((r) => setTimeout(r, ms));
    let n = 0;
    for (const f of list) {
      const id = `toolu_wf${msg}_${++n}`;
      if (f.say) {
        streamText(f.say);
      } else if (f.agent) {
        assistant({ type: 'tool_use', id, name: 'Agent', input: { description: f.agent, subagent_type: f.agent, prompt: 'go' } });
        await wait(30);
        toolResult(id, 'fait');
      } else if (f.skill) {
        assistant({ type: 'tool_use', id, name: 'Skill', input: { skill: f.skill } });
        toolResult(id, 'Launching skill');
      } else if (f.write) {
        const file = path.resolve(process.cwd(), f.write);
        fs.mkdirSync(path.dirname(file), { recursive: true });
        fs.appendFileSync(
          file,
          `${f.text ?? 'écrit par le faux agent'}
`,
        );
        assistant({ type: 'tool_use', id, name: 'Write', input: { file_path: file, content: f.text ?? '' } });
        toolResult(id, `File written: ${f.write}`);
      } else if (f.bash) {
        assistant({ type: 'tool_use', id, name: 'Bash', input: { command: f.bash, description: f.bash } });
        toolResult(id, 'ok');
      } else if (f.wait) {
        await wait(f.wait);
      }
      await wait(20);
    }
    if (hold) {
      // Under way, as [lent]: an interrupt ends it.
      slowTimer = setTimeout(() => result(), 600_000);
      return;
    }
    streamText(fails ? 'Échec simulé.' : 'Fait.');
    result({ isError: fails, subtype: fails ? 'error_during_execution' : 'success' });
  }

  // ---------- "sous-agent long", "sous-agent fini" ----------

  // A subagent in the background, as Claude Code 2.1.300 runs one (measured, docs/PROTOCOL.md): its call, its
  // `task_started`, its launch's result, and the agent's turn ends there. Then its own thread (its frames carry the id of
  // the call and the `agent_id` of its task): a text, then a command it asks permission for (`can_use_tool` with
  // `agent_id`). "sous-agent long" then runs until a `stop_task` of its task (see stopTask); "sous-agent fini", once the
  // permission is answered, hands back its report (`task_notification` with `handback_report`), and Claude Code opens a
  // turn of its own.
  function subagentRun(long) {
    const call = `toolu_sub${++msg}`;
    const task = long ? 'fakelong-agent' : 'fakedone-agent';
    const input = long
      ? {
          description: 'Surveiller le serveur',
          subagent_type: 'Explore',
          model: 'haiku',
          prompt: 'Surveille le serveur de dev\net dis-moi quand il répond.',
        }
      : {
          description: 'Chercher la cause',
          subagent_type: 'general-purpose',
          model: 'sonnet',
          prompt: 'Cherche pourquoi les PDF sont refusés.',
        };
    const sysFrame = (o) => out({ type: 'system', session_id: sessionId, ...o });
    assistant({ type: 'tool_use', id: call, name: 'Agent', input });
    sysFrame({
      subtype: 'task_started',
      task_id: task,
      tool_use_id: call,
      description: input.description,
      subagent_type: input.subagent_type,
      is_backgrounded: true,
      spawn_depth: 1,
      task_type: 'local_agent',
      prompt: input.prompt,
    });
    toolResult(call, `Async agent launched successfully.\nagentId: ${task} (internal ID)`, {
      tool_use_result: {
        isAsync: true,
        status: 'async_launched',
        agentId: task,
        description: input.description,
        resolvedModel: `claude-${input.model}-5-5`,
        prompt: input.prompt,
        canContinueAgent: true,
      },
    });
    streamText('Sous-agent lancé.');
    result();
    setTimeout(() => {
      const thread = (block) =>
        out({
          type: 'assistant',
          message: { id: `msg_${process.pid}_${++msg}`, role: 'assistant', content: [block] },
          parent_tool_use_id: call,
          agent_id: task,
          session_id: sessionId,
        });
      thread({ type: 'text', text: long ? 'Je regarde le serveur.' : 'Je lis le filtre des pièces jointes.' });
      const tuid = `${call}_bash`;
      const cmd = long
        ? { command: 'curl localhost:3000', description: 'Interroge le serveur' }
        : { command: 'grep -n pdf src/filter.ts', description: 'Cherche le filtre' };
      thread({ type: 'tool_use', id: tuid, name: 'Bash', input: cmd });
      sysFrame({
        subtype: 'task_progress',
        task_id: task,
        tool_use_id: call,
        description: `Running ${cmd.description}`,
        usage: { total_tokens: 1800, tool_uses: 1, duration_ms: 300 },
        last_tool_name: 'Bash',
      });
      pendingAnswer = { id: 'req_subperm', tuid, kind: 'subagent', call, task, long, thread, sysFrame };
      out({
        type: 'control_request',
        request_id: 'req_subperm',
        request: {
          subtype: 'can_use_tool',
          tool_name: 'Bash',
          display_name: 'Bash',
          input: cmd,
          description: cmd.description,
          tool_use_id: tuid,
          agent_id: task,
        },
      });
      if (long) longAgent = { task, call, description: input.description, sysFrame };
    }, 150);
  }

  // The answer to the permission a subagent asked for: its thread goes on.
  function subagentAnswered(p, r) {
    const allowed = r.behavior === 'allow';
    out({
      type: 'user',
      message: {
        role: 'user',
        content: [
          { type: 'tool_result', tool_use_id: p.tuid, content: allowed ? 'HTTP 200' : `Refusé : ${r.message}`, is_error: !allowed },
        ],
      },
      parent_tool_use_id: p.call,
      agent_id: p.task,
      session_id: sessionId,
    });
    if (p.long) {
      p.thread({ type: 'text', text: allowed ? 'Le serveur répond, je continue de surveiller.' : 'Je ne peux pas interroger le serveur.' });
      return;
    }
    const report =
      '**Cause trouvée** : le filtre de `src/filter.ts` refuse les PDF.\n\nCorrection proposée : ajouter `application/pdf` à la liste.';
    p.thread({ type: 'text', text: 'Trouvé : le filtre refuse les PDF.' });
    p.thread({ type: 'tool_use', id: `${p.call}_hb`, name: 'SubagentHandback', input: { message: report } });
    p.sysFrame({ subtype: 'task_updated', task_id: p.task, patch: { status: 'completed', end_time: Date.now() } });
    p.sysFrame({
      subtype: 'task_notification',
      task_id: p.task,
      tool_use_id: p.call,
      status: 'completed',
      summary: "This agent's report was delivered to you as a message.",
      usage: { total_tokens: 4200, tool_uses: 3, duration_ms: 2500 },
      handback: 'send',
      handback_report: { text: report },
      uuid: `notif-${p.call}`,
    });
    // Claude Code opens a turn of its own with the report.
    out({ type: 'system', subtype: 'init', session_id: sessionId, model, cwd: process.cwd(), permissionMode: 'default' });
    streamText('Rapport reçu.');
    result();
  }

  // ---------- "commande de fond", "surveillance de fond", "commande premier plan" ----------

  // Tasks Claude Code runs for the agent, as 2.1.301 does it (measured, docs/PROTOCOL.md, ESC-14). "commande de fond 20s"
  // runs a `Bash` command in the background ("Ticker": a line « tick N » every 200 ms for 20 s, 6 by default); in the
  // order of a real run: its call, `background_tasks_changed` (the level, first), `task_started` (`local_bash`), the
  // result (`backgroundTaskId`, « Output is being written to: <file> »), the end of the turn. It ends by itself
  // (`task_updated`, `task_notification` with its words, the level, then a turn of Claude Code's own) or by a `stop_task`
  // (killed, stopped, the level, the answer `{}`, a turn). "surveillance de fond" is a `Monitor` ("Watch loop output",
  // lines « watch N », `taskId` in its result: also a `local_bash` task) and "commande premier plan" a command that
  // runs in the foreground (`is_backgrounded: false`, no level, the turn waits) until the `background_tasks` request
  // of its call sends it to the background (the level, `task_updated`, its result « manually backgrounded »). The
  // output file (<log>.tasks/tasks/<task>.output) is written as the output grows, and `get_task_output` answers its end,
  // 8 KiB at most, for a task running or over; for any other id, the error Claude Code gives.
  const bgTasks = new Map();
  let bgCount = 0;
  const bgDir = () => path.join(`${logFile.replace(/\.jsonl$/, '')}.tasks`, 'tasks');
  const sysOut = (o) => out({ type: 'system', session_id: sessionId, ...o });

  // The level: every task in the background now (the subagent of "sous-agent long" too).
  function bgLevel() {
    const tasks = [...bgTasks.values()]
      .filter((t) => t.background && !t.ended)
      .map((t) => ({ task_id: t.id, run_id: `r-${t.id}`, task_type: 'local_bash', description: t.description }));
    if (longAgent)
      tasks.push({
        task_id: longAgent.task,
        run_id: `r-${longAgent.task}`,
        task_type: 'local_agent',
        subagent_type: 'Explore',
        description: longAgent.description,
      });
    sysOut({ subtype: 'background_tasks_changed', tasks });
  }

  function bgAppend(t, text) {
    t.content += text;
    fs.appendFileSync(t.file, text);
  }

  function bgRun(kind, text) {
    const monitor = kind === 'monitor';
    const foreground = kind === 'foreground';
    const n = ++bgCount;
    const id = `${monitor ? 'fakemon' : foreground ? 'fakefg' : 'fakebg'}${n}`;
    const call = `toolu_bg${n}_${++msg}`;
    const description = monitor ? 'Watch loop output' : foreground ? 'Foreground loop' : 'Ticker';
    const input = monitor
      ? { command: 'for i in $(seq 1 6); do echo watch $i; sleep 3; done', description, timeout_ms: 60000 }
      : foreground
        ? { command: 'for i in $(seq 1 12); do echo fg $i; sleep 2; done; echo fgdone', description }
        : { command: 'for i in $(seq 1 10); do echo tick $i; sleep 2; done', description, run_in_background: true };
    const seconds = Number(/(\d+)s/.exec(text)?.[1] ?? 6);
    fs.mkdirSync(bgDir(), { recursive: true });
    const t = {
      id,
      call,
      description,
      monitor,
      background: !foreground,
      ended: false,
      file: path.join(bgDir(), `${id}.output`),
      content: '',
      lines: 0,
      word: monitor ? 'watch' : foreground ? 'fg' : 'tick',
      until: Date.now() + seconds * 1000,
    };
    fs.writeFileSync(t.file, '');
    bgTasks.set(id, t);
    assistant({ type: 'tool_use', id: call, name: monitor ? 'Monitor' : 'Bash', input });
    // The level comes before the task.
    if (t.background) bgLevel();
    sysOut({
      subtype: 'task_started',
      task_id: id,
      run_id: `r-${id}`,
      tool_use_id: call,
      description,
      is_backgrounded: t.background,
      task_type: 'local_bash',
    });
    t.timer = setInterval(() => {
      bgAppend(t, `${t.word} ${++t.lines}\n`);
      if (Date.now() >= t.until) bgEnds(t);
    }, 200);
    if (foreground) return;
    if (monitor) {
      toolResult(
        call,
        `Monitor started (task ${id}, expires in 1m unless the source ends first; you get one notice at expiry — re-arm if you still need the watch). You will be notified on each event.`,
        {
          tool_use_result: { taskId: id, timeoutMs: 60000, persistent: false },
        },
      );
    } else {
      toolResult(
        call,
        `Command running in background with ID: ${id}. Output is being written to: ${t.file}. You will be notified when it completes. To check interim output, use Read.`,
        {
          tool_use_result: { stdout: '', stderr: '', interrupted: false, isImage: false, noOutputExpected: false, backgroundTaskId: id },
        },
      );
    }
    streamText('Lancé en arrière-plan.');
    result();
  }

  // A command or a monitor ends by itself, with the exit code 0.
  function bgEnds(t) {
    if (t.ended) return;
    clearInterval(t.timer);
    t.ended = true;
    bgAppend(t, '\n[exited with code 0]\n');
    sysOut({ subtype: 'task_updated', task_id: t.id, run_id: `r-${t.id}`, patch: { status: 'completed', end_time: Date.now() } });
    if (!t.background) {
      // In the foreground: the agent was waiting for it, its result goes on with the turn.
      sysOut({
        subtype: 'task_notification',
        task_id: t.id,
        run_id: `r-${t.id}`,
        tool_use_id: t.call,
        status: 'completed',
        output_file: '',
        summary: t.description,
      });
      toolResult(t.call, t.content, { tool_use_result: { stdout: t.content, stderr: '', interrupted: false } });
      streamText('La commande est finie.');
      return result();
    }
    sysOut({
      subtype: 'task_notification',
      task_id: t.id,
      run_id: `r-${t.id}`,
      tool_use_id: t.call,
      status: 'completed',
      output_file: t.file,
      summary: t.monitor ? `Monitor "${t.description}" stream ended` : `Background command "${t.description}" completed (exit code 0)`,
      uuid: `bgend-${t.id}`,
    });
    bgLevel();
    // Claude Code opens a turn of its own about it.
    out({ type: 'system', subtype: 'init', session_id: sessionId, model, cwd: process.cwd(), permissionMode: 'default' });
    streamText('La tâche de fond est finie.');
    result();
  }

  // `stop_task` on a command or a monitor running in the background, as Claude Code 2.1.301 answers it (measured): the
  // task killed, its notification « stopped » (its words are its description), the level, the answer, a turn of its own.
  function bgStop(id, taskId) {
    const t = bgTasks.get(taskId);
    if (!t || t.ended || !t.background) return false;
    clearInterval(t.timer);
    t.ended = true;
    bgAppend(t, '\n[killed]\n');
    sysOut({ subtype: 'task_updated', task_id: t.id, run_id: `r-${t.id}`, patch: { status: 'killed', end_time: Date.now() } });
    sysOut({
      subtype: 'task_notification',
      task_id: t.id,
      run_id: `r-${t.id}`,
      tool_use_id: t.call,
      status: 'stopped',
      output_file: t.file,
      summary: t.description,
      uuid: `bgstop-${t.id}`,
    });
    bgLevel();
    ok(id);
    out({ type: 'system', subtype: 'init', session_id: sessionId, model, cwd: process.cwd(), permissionMode: 'default' });
    streamText('La tâche de fond a été arrêtée.');
    result();
    return true;
  }

  // `get_task_output`: the end of the output, 8 KiB at most, of a command or a monitor, running or over.
  function bgOutput(id, taskId) {
    const t = bgTasks.get(taskId);
    if (!t) {
      return out({
        type: 'control_response',
        response: {
          subtype: 'error',
          request_id: id,
          error: 'get_task_output: no shell or Monitor task with that task_id in this session',
        },
      });
    }
    const bytes = Buffer.from(t.content, 'utf8');
    const tail = bytes.length > 8192 ? bytes.subarray(bytes.length - 8192) : bytes;
    ok(id, { output: tail.toString('utf8'), total_bytes: bytes.length, truncated: bytes.length > 8192 });
  }

  // `background_tasks { tool_use_id }` (what Ctrl+B does): the command of that call, in the foreground, goes to the
  // background: the level that has it, `task_updated` (`is_backgrounded`), its result, and the turn ends.
  function bgSend(id, call) {
    const t = [...bgTasks.values()].find((x) => x.call === call && !x.background && !x.ended);
    if (!t) {
      return out({
        type: 'control_response',
        response: { subtype: 'error', request_id: id, error: `No foreground task for the tool call ${call}` },
      });
    }
    t.background = true;
    bgLevel();
    sysOut({ subtype: 'task_updated', task_id: t.id, run_id: `r-${t.id}`, patch: { is_backgrounded: true } });
    toolResult(
      t.call,
      `Command was manually backgrounded by user with ID: ${t.id}. Output is being written to: ${t.file}. If it is still running after 30m in the background, it will be killed.`,
      {
        tool_use_result: { stdout: '', stderr: '', interrupted: false, backgroundTaskId: t.id, backgroundedByUser: true },
      },
    );
    ok(id);
    streamText('FINISHED');
    result();
  }

  // `stop_task`, as Claude Code 2.1.300 answers it (measured): the task killed and its notification « stopped », then
  // the answer, then the end of the subagent's thread; Claude Code opens a turn of its own about it. A task it does not
  // run is an error. A permission the subagent still waits for is cancelled first (measured on 2.1.300 too).
  function stopTask(id, task) {
    if (bgStop(id, task)) return;
    if (!longAgent || longAgent.task !== task) {
      return out({ type: 'control_response', response: { subtype: 'error', request_id: id, error: `No task found with ID: ${task}` } });
    }
    const { call, description, sysFrame } = longAgent;
    longAgent = null;
    if (pendingAnswer?.kind === 'subagent') {
      out({ type: 'control_cancel_request', request_id: pendingAnswer.id });
      pendingAnswer = null;
    }
    sysFrame({ subtype: 'task_updated', task_id: task, patch: { status: 'killed', end_time: Date.now() } });
    sysFrame({
      subtype: 'task_notification',
      task_id: task,
      tool_use_id: call,
      status: 'stopped',
      output_file: `${task}.output`,
      summary: description,
      uuid: `stopped-${task}`,
    });
    ok(id);
    out({
      type: 'user',
      message: { role: 'user', content: [{ type: 'text', text: '[Request interrupted by user]' }] },
      parent_tool_use_id: call,
      agent_id: task,
      session_id: sessionId,
    });
    out({ type: 'system', subtype: 'init', session_id: sessionId, model, cwd: process.cwd(), permissionMode: 'default' });
    streamText('Le sous-agent a été arrêté.');
    result();
  }

  // The folders Claude Code reads without asking: its own and those of --add-dir (as the app gives them).
  const readable = [process.cwd(), ...argv.filter((a) => a.startsWith('--add-dir=')).map((a) => a.slice('--add-dir='.length))];
  const within = (file, dir) => {
    const rel = path.relative(dir, file);
    return !!rel && !rel.startsWith('..') && !path.isAbsolute(rel);
  };

  // Files attached by path (« Fichier joint : <nom>, <chemin> »), read with the Read tool as Claude would: one outside the
  // folders above asks for a permission first (which the app answers by itself for a file it attached by its origin).
  function readAttached(files, sizes = []) {
    const next = files.shift();
    if (!next) {
      streamText(`Lu : ${sizes.join(', ')}`);
      return result();
    }
    const tuid = `toolu_r${msg}-${sizes.length}`;
    const done = (allowed) => {
      let size = 'refusé';
      if (allowed) {
        try {
          size = `${next.name}=${fs.statSync(next.file).size}`;
        } catch {
          size = `${next.name}=introuvable`;
        }
      }
      toolResult(tuid, allowed ? size : 'Refusé');
      readAttached(files, [...sizes, allowed ? size : `${next.name}=refusé`]);
    };
    assistant({ type: 'tool_use', id: tuid, name: 'Read', input: { file_path: next.file } });
    if (readable.some((dir) => within(next.file, dir))) return done(true);
    pendingAnswer = { id: `req_read${msg}-${sizes.length}`, tuid, kind: 'read', then: (r) => done(r.behavior === 'allow') };
    out({
      type: 'control_request',
      request_id: pendingAnswer.id,
      request: { subtype: 'can_use_tool', tool_name: 'Read', input: { file_path: next.file }, tool_use_id: tuid },
    });
  }

  // A sentence said in a voice conversation (ESC-97): the answer ends with its passage to be read aloud, between
  // <spoken> and </spoken>, as the instruction at the end of the message asks; « base » asks a question first, and
  // the passage says the option chosen.
  const PROJECT_QUESTIONS = [
    {
      question: 'Quelle base de données ?',
      header: 'Base',
      multiSelect: false,
      options: [
        { label: 'PostgreSQL', description: 'Relationnelle' },
        { label: 'SQLite', description: 'Embarquée' },
      ],
    },
    {
      question: 'Quels tests ?',
      header: 'Tests',
      multiSelect: true,
      options: [
        { label: 'Unitaires', description: '' },
        { label: 'E2E', description: '' },
        { label: 'Lint', description: '' },
      ],
    },
    {
      question: 'Quel nom pour le projet ?',
      header: 'Nom',
      multiSelect: false,
      options: [
        { label: 'web-shop', description: '' },
        { label: 'admin', description: '' },
      ],
    },
  ];

  function voiceTurn(text) {
    const said = text.slice(0, text.indexOf('<escouade-voice>')).trim();
    const answer = (choice = '') => {
      const chose = choice ? ` Tu as choisi ${choice}.` : '';
      streamText(
        `J'ai bien noté : « ${said} ».${chose}\n\n- détail technique : \`src/app.rs\`, ligne 42\n\n<spoken>C'est noté, je m'en occupe.${chose}</spoken>`,
      );
      result();
    };
    // « projet » (ESC-131): a request of three questions, a simple one, one of several choices, one answered freely.
    if (said.toLowerCase().includes('projet'))
      return askQuestion((answers) => answer(Object.values(answers ?? {}).join(' ; ')), PROJECT_QUESTIONS);
    if (said.toLowerCase().includes('base')) return askQuestion((answers) => answer(Object.values(answers ?? {}).join(', ')));
    answer();
  }

  // As EnterWorktree: a worktree of the repository on a new branch, made once, where it works from now on.
  function enterWorktree(branch) {
    const top = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim();
    const dir = path.join(top, '.claude', 'worktrees', branch.replace(/[^a-zA-Z0-9]/g, '-'));
    if (!fs.existsSync(dir)) execFileSync('git', ['worktree', 'add', '-q', '-b', branch, dir], { cwd: top, stdio: 'ignore' });
    process.chdir(dir);
  }

  // ---------- The website's screenshots: turns written in $FAKE_CLAUDE_DEMO ----------

  // A user message that is a key of the JSON file $FAKE_CLAUDE_DEMO (tests/site-shots) plays its steps, a few tens of
  // milliseconds apart: { thinking }, { text } (streamed markdown), { tool: 'Edit', file, old, new } (made in the file of the
  // agent's folder, its patch computed from it), { tool: 'Write', file, content }, { tool: 'Bash', command, description,
  // output }, { tool: 'Read', file, lines }, { tool: 'TodoWrite', todos }, { send: [files], caption } (SendUserFile, files of
  // the agent's folder), { question: [questions] } (waits for the answer, then goes on), { hold: true } (the turn stays at
  // work: its last tool call on the card), { usage } (the session's totals so far). The turn ends with a result unless it
  // holds.
  let demoTurns = null;
  function demoTurn(text) {
    const file = process.env.FAKE_CLAUDE_DEMO;
    if (!file) return null;
    demoTurns ??= JSON.parse(fs.readFileSync(file, 'utf8'));
    return demoTurns[text.trim()] ?? null;
  }

  function playDemo(steps) {
    const step = steps[0];
    if (!step) return result();
    const next = () => setTimeout(() => playDemo(steps.slice(1)), 60);
    const call = (name, input) => {
      const id = `toolu_demo${++msg}`;
      assistant({ type: 'tool_use', id, name, input });
      return id;
    };
    const abs = (f) => path.join(process.cwd(), f);
    if (step.usage) {
      usage.inputTokens = step.usage.input;
      usage.outputTokens = step.usage.output;
      usage.cacheReadInputTokens = step.usage.cache;
      usage.costUSD = step.usage.cost - 0.05;
      return next();
    }
    if (step.thinking) assistant({ type: 'thinking', thinking: step.thinking, signature: 'sig-demo' });
    else if (step.text) streamText(step.text);
    else if (step.tool === 'Edit') {
      const before = fs.readFileSync(abs(step.file), 'utf8');
      const at = before.indexOf(step.old);
      const after = before.slice(0, at) + step.new + before.slice(at + step.old.length);
      fs.writeFileSync(abs(step.file), after);
      const line = before.slice(0, at).split('\n').length;
      const lines = [...step.old.split('\n').map((l) => `-${l}`), ...step.new.split('\n').map((l) => `+${l}`)];
      const oldLines = step.old.split('\n').length;
      const newLines = step.new.split('\n').length;
      const id = call('Edit', { file_path: abs(step.file), old_string: step.old, new_string: step.new });
      toolResult(id, `The file ${abs(step.file)} has been updated.`, {
        tool_use_result: { filePath: abs(step.file), structuredPatch: [{ oldStart: line, oldLines, newStart: line, newLines, lines }] },
      });
    } else if (step.tool === 'Write') {
      fs.mkdirSync(path.dirname(abs(step.file)), { recursive: true });
      fs.writeFileSync(abs(step.file), step.content);
      const id = call('Write', { file_path: abs(step.file), content: step.content });
      toolResult(id, `File created successfully at: ${abs(step.file)}`);
    } else if (step.tool === 'Bash') {
      const id = call('Bash', { command: step.command, description: step.description ?? '' });
      if (!step.hold) toolResult(id, step.output ?? '', { tool_use_result: { stdout: step.output ?? '', stderr: '', interrupted: false } });
    } else if (step.tool === 'Read') {
      const id = call('Read', { file_path: abs(step.file) });
      toolResult(id, `${step.lines} lines`, {
        tool_use_result: { type: 'text', file: { filePath: abs(step.file), numLines: step.lines } },
      });
    } else if (step.tool === 'TodoWrite') {
      const id = call('TodoWrite', { todos: step.todos });
      toolResult(id, 'Todos have been modified successfully.');
    } else if (step.send) {
      const files = step.send.map((f) => abs(f).replace(/\\/g, '/'));
      const id = call('SendUserFile', { files, caption: step.caption ?? '', status: 'normal', display: 'render' });
      toolResult(id, `${files.length} files delivered to user.`);
    } else if (step.question) {
      const tuid = call('AskUserQuestion', {});
      pendingAnswer = { id: 'req_question', tuid, kind: 'question', then: () => playDemo(steps.slice(1)) };
      out({
        type: 'control_request',
        request_id: 'req_question',
        request: {
          subtype: 'can_use_tool',
          tool_name: 'AskUserQuestion',
          input: { questions: step.question },
          tool_use_id: tuid,
          requires_user_interaction: true,
        },
      });
      return;
    }
    if (step.hold) return;
    next();
  }

  function onUser(text) {
    out({ type: 'system', subtype: 'init', session_id: sessionId, model, cwd: process.cwd(), permissionMode: 'default' });
    const demo = demoTurn(text);
    if (demo) return playDemo(demo);
    const attached = [...text.matchAll(/^(?:Fichier joint : |Attached file: )(.+), (.+)$/gm)].map((m) => ({
      name: m[1],
      file: m[2].trim(),
    }));
    if (attached.length) return readAttached(attached);
    if (text.includes('<escouade-voice>')) return voiceTurn(text);
    if (text.includes('[ferme-lentement]')) slowClose = true;
    if (text.includes('[quota-rythme]')) pace();
    const entering = text.match(/\[worktree ([\w./-]+)\]/);
    if (entering) enterWorktree(entering[1]);
    if (text.includes('crash')) process.exit(3);
    if (text.startsWith('/cost')) return localCommand(text);
    if (text.startsWith('/loop')) return loopTurn(text);
    if (text.includes('grandchild')) {
      // Like a dev server started by the Bash tool: must die with the agent's process tree. Node ends
      // a child that is not detached with its parent (on Windows, through a job of its own); with
      // [detached] it lives on after the process has ended, in the agent's job still.
      const child = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60000)'], {
        stdio: 'ignore',
        detached: text.includes('[detached]'),
      });
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
    if (oauthLost()) {
      // As Claude Code 2.1.309 tells it (its `authentication_failed`): a failed turn, no rate limit.
      const said = 'Failed to authenticate: OAuth session expired and could not be refreshed';
      out({
        type: 'assistant',
        error: 'authentication_failed',
        message: { id: `msg_oauth${msg}`, role: 'assistant', content: [{ type: 'text', text: said }] },
        parent_tool_use_id: null,
        session_id: sessionId,
      });
      out({ type: 'result', subtype: 'success', is_error: true, duration_ms: 100, session_id: sessionId, result: said });
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
    if (text.includes('envoie des fichiers')) return sendFiles(text);
    if (text.includes('sorties piégées')) return trappedOutputs();
    if (sys) return ticketTurn(text);
    const facts = /faits-workflow(-erreur)?:\s*(\[[\s\S]*\])/.exec(text);
    if (facts) {
      workflowFacts(JSON.parse(facts[2]), Boolean(facts[1])).catch((e) => {
        process.stderr.write(`faits-workflow: ${e.stack}\n`);
        result({ isError: true, subtype: 'error_during_execution' });
      });
      return;
    }
    if (text.includes('plan superpowers')) {
      planRun(text).catch((e) => {
        // An interrupt came at any moment of the run: the turn already ended with its answer.
        if (e.message === 'interrupted') return;
        process.stderr.write(`plan superpowers: ${e.stack}\n`);
        result({ isError: true, subtype: 'error_during_execution' });
      });
      return;
    }
    // A command, a monitor in the background, a command that goes there from the foreground (see bgRun).
    const bgKind = /commande de fond|background command/.test(text)
      ? 'command'
      : /surveillance de fond|background monitor/.test(text)
        ? 'monitor'
        : /commande premier plan|foreground command/.test(text)
          ? 'foreground'
          : null;
    if (bgKind) return bgRun(bgKind, text);
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
    // Escouade asks the agent to pass a message on to its subagent (« Lui écrire »): it does, with SendMessage.
    const passOn =
      /(?:Transmets ce message au sous-agent|Pass this message on to the subagent) .*?\(agentId (\S+)\)[^\n]*\n\n([\s\S]*)$/.exec(text);
    if (passOn) {
      const tuid = `toolu_m${msg}`;
      assistant({ type: 'tool_use', id: tuid, name: 'SendMessage', input: { to: passOn[1], message: passOn[2] } });
      toolResult(tuid, `Message sent to ${passOn[1]}.`);
      streamText('Message transmis au sous-agent.');
      result();
      return;
    }
    if (text.includes('sous-agent long') || text.includes('sous-agent fini')) return subagentRun(text.includes('sous-agent long'));
    if (text.includes('sous-agent sans modèle')) {
      // In the foreground, its call naming no model: its message tells the one it runs on.
      const tuid = `toolu_m${msg}`;
      assistant({
        type: 'tool_use',
        id: tuid,
        name: 'Agent',
        input: { description: 'Lister les fichiers', subagent_type: 'Explore', prompt: 'Liste src' },
      });
      out({
        type: 'assistant',
        message: { id: `msg_m${msg}`, role: 'assistant', model: 'claude-haiku-5-5', content: [{ type: 'text', text: 'Deux fichiers.' }] },
        parent_tool_use_id: tuid,
        session_id: sessionId,
        uuid: `entry-m${msg}`,
      });
      const content = [{ type: 'text', text: 'Deux fichiers.' }];
      toolResult(tuid, content, {
        tool_use_result: {
          status: 'completed',
          agentId: 'fakemodel',
          content,
          totalToolUseCount: 0,
          totalDurationMs: 300,
          totalTokens: 900,
        },
      });
      streamText('Rapport reçu.');
      result();
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
    // « Mettre la spec à jour » (ESC-261): the spec writer proposes the page from the code, its [écart] gone.
    const gapPage = text.match(/\(\.escouade\/spec\/(.+?)\.md\) (?:ne dit pas ce que fait le code|doesn’t say what the code does)/)?.[1];
    const writerOf = appended.match(/(?:du projet « ([^»]+) »|of the project “([^”]+)”)/);
    if (gapPage && writerOf) {
      const project = (writerOf[1] ?? writerOf[2]).trim();
      (async () => {
        const read = await escouadeTool('read_spec_page', { project, path: gapPage });
        const old = JSON.parse(read.content[0].text).body;
        await escouadeTool('write_spec_page', {
          project,
          path: gapPage,
          body: `${old.replace(/\s*\[écart\]/g, '')}\n\nMis à jour d’après le code par le faux claude.`,
        });
      })()
        .then(() => streamText('Page proposée d’après le code.'))
        .catch((e) => {
          process.stderr.write(`mise à jour: ${e.stack}\n`);
          streamText(`Échec : ${e.message}`);
        })
        .finally(() => result());
      return;
    }
    // [proposer:<project>] (ESC-261): what a discussion proposes for the spec, by Escouade's docs tools.
    const proposeIn = text.match(/\[proposer:([^\]]+)\]/)?.[1];
    if (proposeIn) {
      (async () => {
        await escouadeTool('write_spec_page', {
          project: proposeIn,
          path: 'espace-adherent/relances',
          title: 'Relances',
          subject: 'Espace adhérent',
          body: 'Trois relances par e-mail : 30 et 7 jours avant l’échéance, puis 3 jours après.\nUn adhérent peut couper les relances depuis son espace.',
        });
        const tree = await escouadeTool('get_spec_tree', { project: proposeIn });
        if (tree?.content?.[0]?.text?.includes('notifications/general'))
          await escouadeTool('write_spec_page', {
            project: proposeIn,
            path: 'notifications/general',
            body: 'Les relances d’adhésion sont planifiées chaque nuit.',
          });
        await escouadeTool('propose_decision', {
          project: proposeIn,
          title: 'Relances par e-mail seulement',
          context: 'Les SMS demandent un consentement à part.',
          decision: 'Pas de SMS.',
          consequences: 'À revoir si le renouvellement baisse.',
          pages: ['espace-adherent/relances'],
        });
      })()
        .then(() => streamText('Proposé dans la spec.'))
        .catch((e) => {
          process.stderr.write(`proposer: ${e.stack}\n`);
          streamText(`Échec : ${e.message}`);
        })
        .finally(() => result());
      return;
    }
    if (text.includes('lire-fichier ') || text.includes('modifier-fichier ')) {
      // A file the agent wants to read (`Read`) or to change (`Edit`): Claude Code asks the permission, unless the
      // host answers it by itself.
      const read = text.includes('lire-fichier ');
      const tool = read ? 'Read' : 'Edit';
      const file = new RegExp(`${read ? 'lire' : 'modifier'}-fichier (.+)`).exec(text)[1].trim();
      const tuid = `toolu_f${msg}`;
      const input = read ? { file_path: file } : { file_path: file, old_string: 'a', new_string: 'b' };
      assistant({ type: 'tool_use', id: tuid, name: tool, input });
      pendingAnswer = { id: `req_f${msg}`, tuid, kind: 'file', tool };
      out({
        type: 'control_request',
        request_id: `req_f${msg}`,
        request: { subtype: 'can_use_tool', tool_name: tool, input, tool_use_id: tuid },
      });
      return;
    }
    if (text.includes('exit plan')) {
      // The plan mode's end: ExitPlanMode asks for the plan to be approved (a permission).
      const tuid = `toolu_x${msg}`;
      const input = { plan: '# Plan\n\n1. Écrire le test\n2. Faire passer le test' };
      assistant({ type: 'tool_use', id: tuid, name: 'ExitPlanMode', input });
      pendingAnswer = { id: 'req_plan', tuid, kind: 'permission', plan: true };
      out({
        type: 'control_request',
        request_id: 'req_plan',
        request: {
          subtype: 'can_use_tool',
          tool_name: 'ExitPlanMode',
          input,
          tool_use_id: tuid,
          permission_suggestions: [{ type: 'setMode', mode: 'acceptEdits', destination: 'session' }],
        },
      });
      return;
    }
    // Two questions at once (ESC-75: answered from the list one after the other).
    if (text.includes('deux questions'))
      return askQuestion(null, [
        {
          question: 'Quelle base de données ?',
          header: 'Base',
          multiSelect: false,
          options: [
            { label: 'PostgreSQL', description: '' },
            { label: 'SQLite', description: '' },
          ],
        },
        {
          question: 'Quel ORM ?',
          header: 'ORM',
          multiSelect: false,
          options: [
            { label: 'Prisma', description: '' },
            { label: 'Drizzle', description: '' },
          ],
        },
      ]);
    // Four questions, the first one long with a very long choice (ESC-249: the card held to the sidebar's width).
    if (text.includes('questions longues'))
      return askQuestion(null, [
        {
          question: 'La page de démo doit-elle appeler le vrai Purse (sandbox) ou un Purse simulé, le temps que les clés arrivent ?',
          header: 'Purse',
          multiSelect: false,
          options: [
            { label: 'Simulé + bascule sandbox (Recommandé), par une variable d’environnement lue au démarrage', description: '' },
            { label: 'Purse simulé uniquement', description: '' },
            { label: 'Purse sandbox uniquement', description: '' },
            { label: 'Autre', description: '' },
          ],
        },
        ...['Quel port ?', 'Quelle langue ?', 'Quel thème ?'].map((question, i) => ({
          question,
          header: `Q${i + 2}`,
          multiSelect: false,
          options: [
            { label: 'Oui', description: '' },
            { label: 'Non', description: '' },
          ],
        })),
      ]);
    // A long question with several answers: answered in the conversation, the card saying it (ESC-249).
    if (text.includes('question multiple'))
      return askQuestion(null, [
        {
          question: 'Quels environnements la page de démo doit-elle couvrir, entre la sandbox, la préproduction et la production ?',
          header: 'Envs',
          multiSelect: true,
          options: [
            { label: 'Sandbox', description: '' },
            { label: 'Préproduction', description: '' },
          ],
        },
      ]);
    // A permission with a long summary, still answered from the list (ESC-249).
    if (text.includes('permission longue')) {
      const tuid = `toolu_p${msg}`;
      const input = {
        command: 'npx playwright test tests/e2e/checkout-sdk-surcharge-avec-la-bascule-sandbox.spec.ts --project=chromium --reporter=line',
        description: 'Lance les tests de la page de démo',
      };
      assistant({ type: 'tool_use', id: tuid, name: 'Bash', input });
      pendingAnswer = { id: 'req_perm', tuid, kind: 'permission' };
      out({
        type: 'control_request',
        request_id: 'req_perm',
        request: { subtype: 'can_use_tool', tool_name: 'Bash', input, tool_use_id: tuid },
      });
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
    if (text.includes('flood-real')) {
      // "flood-real 90s" lasts 90 seconds, a plain "flood-real" 60.
      floodReal(Number(/flood-real (\d+)s/.exec(text)?.[1] ?? 60) * 1000);
      return;
    }
    if (text.includes('flood')) {
      // "flood 90s" lasts 90 seconds, a plain "flood" 60.
      flood(Number(/flood (\d+)s/.exec(text)?.[1] ?? 60) * 1000);
      return;
    }
    if (text.includes('slow')) {
      streamText('Je commence…');
      // "slow 3s" ends after 3 seconds, a plain "slow" after 30.
      slowTimer = setTimeout(() => result(), Number(/slow (\d+)s/.exec(text)?.[1] ?? 30) * 1000);
      return;
    }
    if (text.includes('raisonnement')) {
      // A turn that reasons before it answers: a thinking block, then its reply.
      assistant({ type: 'thinking', thinking: 'Je pèse les deux pistes avant de répondre.', signature: 'sig-fake' });
      streamText('Réponse réfléchie.');
      result();
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
    if (p.kind === 'subagent') return subagentAnswered(p, r);
    if (p.kind === 'read') return p.then(r);
    if (p.kind === 'question') {
      const answers = r.updatedInput?.answers ?? {};
      toolResult(p.tuid, `Réponses : ${JSON.stringify(answers)}`, { tool_use_result: { answers } });
      if (p.then) return p.then(answers);
      streamText(`Choix retenu : ${Object.values(answers).join(', ')}`);
    } else if (p.kind === 'file') {
      toolResult(p.tuid, r.behavior === 'allow' ? 'contenu du fichier' : `Refusé : ${r.message}`);
      streamText(`${p.tool} : ${r.behavior}`);
    } else if (r.behavior === 'allow' && p.plan) {
      // As Claude Code: out of Plan in the mode its answer sets (`setMode`), else `default` (ESC-277).
      const mode = (r.updatedPermissions ?? []).find((u) => u.type === 'setMode')?.mode ?? 'default';
      out({ type: 'system', subtype: 'status', status: null, permissionMode: mode, session_id: sessionId });
      toolResult(p.tuid, 'Plan approuvé');
      streamText(mode === 'acceptEdits' ? 'Plan approuvé (édits acceptés).' : 'Plan approuvé.');
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
          return later('initialize', () =>
            ok(m.request_id, {
              commands: [
                { name: 'compact', description: 'Compacte le contexte', argumentHint: '' },
                { name: 'review', description: 'Revue de code', argumentHint: '[pr]' },
              ],
              models: modelsListed(),
              account: {},
            }),
          );
        case 'get_usage':
          // Once [quota-rythme] was asked, the windows it tells, read again as the endpoint writes them.
          if (isPaced()) {
            const w = windows();
            const said = (x) => ({ utilization: x.utilization * 100, resets_at: new Date(x.resetsAt * 1000).toISOString() });
            return ok(m.request_id, { rate_limits: { five_hour: said(w.five_hour), seven_day: said(w.seven_day) } });
          }
          return ok(m.request_id, {
            rate_limits: {
              five_hour: { utilization: limited() ? 100 : 12, resets_at: '2026-09-28T01:20:00+00:00' },
              seven_day: { utilization: 34, resets_at: '2026-09-30T22:00:00+00:00' },
            },
          });
        case 'interrupt':
          clearTimeout(slowTimer);
          clearInterval(floodTimer);
          planCut = true;
          // A command in the foreground is stopped with the turn; the ones in the background go on.
          for (const t of bgTasks.values())
            if (!t.background && !t.ended) {
              clearInterval(t.timer);
              t.ended = true;
            }
          if (sys.includes('[fin-d-abord]')) {
            // The app reads the turn's end before the interrupt's answer reaches its caller.
            result({ isError: true, subtype: 'error_during_execution' });
            return ok(m.request_id, { still_queued: [] });
          }
          ok(m.request_id, { still_queued: [] });
          return result({ isError: true, subtype: 'error_during_execution' });
        case 'remote_control':
          return remoteControl(m.request_id, r);
        case 'rename_session':
          // Logged to <log>.rename.jsonl: the title it is given.
          fs.appendFileSync(logFile.replace(/\.jsonl$/, '') + '.rename.jsonl', JSON.stringify(r) + '\n');
          return ok(m.request_id);
        case 'stop_task':
          fs.appendFileSync(logFile.replace(/\.jsonl$/, '') + '.control.jsonl', JSON.stringify(r) + '\n');
          return stopTask(m.request_id, r.task_id);
        case 'get_task_output':
          fs.appendFileSync(logFile.replace(/\.jsonl$/, '') + '.control.jsonl', JSON.stringify(r) + '\n');
          return bgOutput(m.request_id, r.task_id);
        case 'background_tasks':
          fs.appendFileSync(logFile.replace(/\.jsonl$/, '') + '.control.jsonl', JSON.stringify(r) + '\n');
          return bgSend(m.request_id, r.tool_use_id);
        default:
          return ok(m.request_id);
      }
    }
    if (m.type === 'user') {
      fs.appendFileSync(logFile.replace(/\.jsonl$/, '') + '.stdin.jsonl', JSON.stringify(m) + '\n');
      keep({ type: 'user', uuid: m.uuid, message: m.message });
      // Real CLI: echoes stdin messages (same uuid) with --replay-user-messages; a `/` command as the command it runs
      // (2.1.309, ESC-154).
      if (replay) {
        const echoed = typeof m.message.content === 'string' ? commandTags(m.message.content) : null;
        const message = echoed ? { ...m.message, content: echoed } : m.message;
        out({ type: 'user', message, parent_tool_use_id: null, uuid: m.uuid, isReplay: true, session_id: sessionId });
      }
      onUser(typeof m.message.content === 'string' ? m.message.content : m.message.content.map((b) => b.text ?? '').join(' '));
    }
  });

  // Remote Control: requests are logged to <log>.control.jsonl. With $FAKE_CLAUDE_REMOTE_MESSAGE,
  // a message "sent from claude.ai" arrives once it is on (replayed with origin human, then run).
  // As Claude Code does, a session opened with keep_session_on_exit stays on claude.ai when Remote
  // Control is turned off; one opened with keep_session_on_exit false is archived then (its bridge
  // id is added to <log>.archived.jsonl).
  let remoteOn = null;
  function remoteControl(id, r) {
    fs.appendFileSync(logFile.replace(/\.jsonl$/, '') + '.control.jsonl', JSON.stringify(r) + '\n');
    if (!r.enabled) {
      if (remoteOn && remoteOn.keep === false)
        fs.appendFileSync(logFile.replace(/\.jsonl$/, '') + '.archived.jsonl', JSON.stringify({ bridge: remoteOn.bridge }) + '\n');
      remoteOn = null;
      ok(id);
      return out({ type: 'system', subtype: 'bridge_state', state: 'disconnected', session_id: sessionId });
    }
    const bridge = r.reattach_session_id ?? `cse_fake_${process.pid}`;
    if (!remoteOn) remoteOn = { bridge, keep: r.keep_session_on_exit !== false };
    later('remote', () => {
      out({ type: 'system', subtype: 'bridge_state', state: 'ready', session_id: sessionId });
      ok(id, { session_url: `https://claude.ai/code/session_${bridge.slice(4)}`, bridge_session_id: bridge, bridge_epoch: 1 });
      out({ type: 'system', subtype: 'bridge_state', state: 'connected', bridge_epoch: 1, session_id: sessionId });
    });
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
