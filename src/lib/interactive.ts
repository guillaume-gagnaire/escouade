// The interactive commands of a harness (ESC-401): those its own interface opens a screen for, which do nothing useful
// typed to the process Escouade drives. Typed in the composer or picked in its « / » menu, one never goes to the agent as
// a message: it opens what Escouade has for it, else it goes to the harness's own view, typed there.
//
// What Claude Code 2.1.319 does with each, typed to its stream-json process (tried on 2026-11-09 with a `claude -p
// --input-format stream-json` in an empty CLAUDE_CONFIG_DIR, and read in the commands `initialize` lists):
//
// | Command                              | In stream-json                                  | In Escouade                                   |
// | ------------------------------------ | ----------------------------------------------- | --------------------------------------------- |
// | /login                               | « /login isn't available in this environment. » | the agent's account signs in (Réglages' gesture), then the agent goes on |
// | /logout                              | « isn't available in this environment »         | asked, then the account signs out (`claude auth logout`) |
// | /model (alone)                       | the current model and a usage line              | the composer's model menu                     |
// | /effort (alone)                      | a usage line                                    | the composer's effort menu                    |
// | /plan [text]                         | « isn't available in this environment »         | Plan mode, then the text sent                 |
// | /usage, /extra-usage, /usage-credits | the session's cost / not available              | Réglages › Comptes et harness (quotas)        |
// | /remote-control                      | « isn't available in this environment »         | the agent's Remote Control turned on          |
// | /config, /mcp (alone)                | the keys it takes / the servers and a usage line| the Claude Code view, typed there             |
// | /permissions, /hooks, /plugin, /skills, /memory, /status, /help, /ide, /vim, /theme, /terminal-setup, /statusline, /keybindings, /privacy-settings, /install-github-app, /feedback, /bug, /release-notes, /upgrade, /add-dir, /export, /rewind, /tasks, /bashes, /copy, /btw, /sandbox, /advisor, /import, /voice, /ultrareview, /web-setup, /workflows, /background, /schedule, /mobile, /desktop, /chrome, /powerup, /passes, /tui | « isn't available in this environment » | the Claude Code view, typed there |
// | /resume, /branch, /fork, /teleport   | « isn't available in this environment »         | not offered: the agent keeps its session (one line) |
// | /exit                                | « isn't available in this environment »         | not offered: nothing to quit (one line)       |
// | /focus                               | « /focus isn't available here yet »             | the Claude Code view, typed there             |
//
// Not interactive there, so left as they are (their answer shows in the conversation): /clear, /compact, /context,
// /cost, /stats, /color, /rename, /output-style, /autocompact, /goal, /reload-plugins, /agents (its wizard is gone: it
// says so), /model <name>, /effort <level>, /config key=value, /mcp <action>, and every skill or command of the user's.
//
// The other harnesses (no view of Escouade's for their own screens yet, their commands not tried one by one): /login,
// /logout, /model and /effort alone only, where their gesture of Escouade exists; their other commands go as they did.

import type { SettingsPage } from './settings-pages';
import type { EngineId } from './types';

/** Where an interactive command leads. */
export type Opens =
  | { kind: 'login' }
  | { kind: 'logout' }
  | { kind: 'menu'; menu: 'model' | 'effort' }
  | { kind: 'plan'; rest: string }
  | { kind: 'settings'; page: SettingsPage }
  | { kind: 'remote' }
  /** Its harness's view, the command typed there. */
  | { kind: 'view'; command: string }
  /** Not offered from Escouade: said in one line, never sent. */
  | { kind: 'kept'; why: 'session' | 'exit' };

type Target = 'login' | 'logout' | 'model' | 'effort' | 'plan' | 'usage' | 'remote' | 'view' | 'session' | 'exit';

interface Entry {
  name: string;
  target: Target;
  /** Interactive alone only: with arguments, its process runs it (`/model opus`). */
  bare?: boolean;
}

const e =
  (target: Target, bare = false) =>
  (name: string) => ({ name, target, bare });

/** Claude Code's, as 2.1.319 has them (see the table above). */
const CLAUDE: Entry[] = [
  ...['login'].map(e('login')),
  ...['logout'].map(e('logout')),
  ...['model'].map(e('model', true)),
  ...['effort'].map(e('effort', true)),
  ...['plan'].map(e('plan')),
  ...['usage', 'extra-usage', 'usage-credits'].map(e('usage')),
  ...['remote-control'].map(e('remote')),
  ...['config', 'mcp'].map(e('view', true)),
  ...[
    'permissions',
    'hooks',
    'plugin',
    'skills',
    'memory',
    'status',
    'help',
    'ide',
    'vim',
    'theme',
    'terminal-setup',
    'statusline',
    'keybindings',
    'privacy-settings',
    'install-github-app',
    'feedback',
    'bug',
    'release-notes',
    'upgrade',
    'add-dir',
    'export',
    'rewind',
    'tasks',
    'bashes',
    'copy',
    'btw',
    'sandbox',
    'advisor',
    'import',
    'voice',
    'ultrareview',
    'web-setup',
    'workflows',
    'background',
    'schedule',
    'mobile',
    'desktop',
    'chrome',
    'powerup',
    'passes',
    'tui',
    'focus',
  ].map(e('view')),
  ...['resume', 'branch', 'fork', 'teleport'].map(e('session')),
  ...['exit'].map(e('exit')),
];

/** Every other harness's: what Escouade does for it whatever the harness (see the head of this file). */
const ANY: Entry[] = [
  ...['login'].map(e('login')),
  ...['logout'].map(e('logout')),
  ...['model'].map(e('model', true)),
  ...['effort'].map(e('effort', true)),
];

const entries = (engine: EngineId | null | undefined): Entry[] => ((engine ?? 'claude') === 'claude' ? CLAUDE : ANY);

/** The command a text is: its name and what follows it, when it starts with one (`/model opus` → model, opus). */
function command(text: string): { name: string; rest: string } | null {
  const m = text.trim().match(/^\/([\w:.-]+)(?:\s+([\s\S]*))?$/);
  return m ? { name: m[1].toLowerCase(), rest: (m[2] ?? '').trim() } : null;
}

/** Where `text`, typed to an agent of `engine`, leads, when it is one of its harness's interactive commands; else null. */
export function interactiveOf(engine: EngineId | null | undefined, text: string): Opens | null {
  const c = command(text);
  if (!c) return null;
  const hit = entries(engine).find((x) => x.name === c.name);
  if (!hit || (hit.bare && c.rest)) return null;
  switch (hit.target) {
    case 'login':
    case 'logout':
    case 'remote':
      return { kind: hit.target };
    case 'model':
    case 'effort':
      return { kind: 'menu', menu: hit.target };
    case 'plan':
      return { kind: 'plan', rest: c.rest };
    case 'usage':
      return { kind: 'settings', page: 'accounts' };
    case 'view':
      return { kind: 'view', command: text.trim() };
    case 'session':
    case 'exit':
      return { kind: 'kept', why: hit.target };
  }
}

/** Where a command of the « / » menu opens, as its mark says it. */
export type Where = 'account' | 'menu' | 'mode' | 'settings' | 'remote' | 'view' | 'kept';

/** The interactive commands of `engine` for the « / » menu, each with where it opens. */
export function interactiveCommands(engine: EngineId | null | undefined): { name: string; where: Where }[] {
  return entries(engine).map((x) => ({ name: x.name, where: whereOf(x.target) }));
}

function whereOf(target: Target): Where {
  switch (target) {
    case 'login':
    case 'logout':
      return 'account';
    case 'model':
    case 'effort':
      return 'menu';
    case 'plan':
      return 'mode';
    case 'usage':
      return 'settings';
    case 'remote':
      return 'remote';
    case 'view':
      return 'view';
    case 'session':
    case 'exit':
      return 'kept';
  }
}

/** The command each agent's harness view types once its terminal is up, by agent (`AgentTerminal`). */
const forView = new Map<string, string>();

/** `command` is typed in the agent's harness view once it is up. */
export function queueViewCommand(agentId: string, command: string) {
  forView.set(agentId, command);
}

/** The command waiting for the agent's harness view, taken (once). */
export function takeViewCommand(agentId: string): string | undefined {
  const c = forView.get(agentId);
  forView.delete(agentId);
  return c;
}
