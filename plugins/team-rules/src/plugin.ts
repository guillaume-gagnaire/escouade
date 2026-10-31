// Team rules: the agents' commands allowed or denied by rules.json, a file of the plugin's repository that the team
// keeps under review like its code. A command that holds a denied one is denied; a command made only of allowed ones is
// allowed; anything else goes to the user, as without the plugin. A refusal of any plugin wins.
import { readFileSync } from 'node:fs';
import { createPlugin, type Decision, type Plugin } from '@escouade-dev/plugin';

export interface Rules {
  /** Commands denied, by their start: a text, or a text and the message the agent is given. */
  deny: Array<string | { command: string; message?: string }>;
  /** Commands allowed, by their start. */
  allow: string[];
}

/** rules.json, beside escouade-plugin.json (src/ and dist/ are one level below it): read once, a decision has 500 ms. */
export function readRules(): Rules {
  return JSON.parse(readFileSync(new URL('../rules.json', import.meta.url), 'utf8'));
}

/** The commands of a line: `cd app && rm -rf build` is two, each judged on its own (`&` too, which sends one away). */
export function commandsOf(line: string): string[] {
  return line
    .split(/&&|\|\||[;|&\n]/)
    .map((c) => c.trim().replace(/\s+/g, ' '))
    .filter(Boolean);
}

/** What makes a line more than its commands: a substitution (`$(…)`, backquotes, `$VAR`), a redirection (`>`, `<`,
 * `<(…)`), an escape. A rule judges a command by its start, so a line that holds one of these is never allowed:
 * `git diff > ~/.bashrc` starts like `git diff`. */
const HIDDEN = /[`$<>\\]/;

/** Whether `command` starts with `rule`, word for word (`npm testing` is not `npm test`). */
const startsWith = (command: string, rule: string) => command === rule || command.startsWith(rule + ' ');

export function decide(rules: Rules, line: string): Decision {
  const commands = commandsOf(line);
  for (const rule of rules.deny) {
    const { command, message } = typeof rule === 'string' ? { command: rule, message: undefined } : rule;
    if (commands.some((c) => startsWith(c, command))) {
      return { decision: 'deny', message: message ?? `The team's rules forbid \`${command}\`.` };
    }
  }
  if (!HIDDEN.test(line) && commands.length > 0 && commands.every((c) => rules.allow.some((rule) => startsWith(c, rule)))) {
    return { decision: 'allow' };
  }
  return { decision: 'ask' };
}

export function teamRules(rules: Rules = readRules()): Plugin {
  return createPlugin().decidePermission(({ request }) => {
    const line = request.input?.command;
    // Only commands: an edit or a fetch goes to the user.
    if (request.kind !== 'command' || typeof line !== 'string') return { decision: 'ask' };
    return decide(rules, line);
  });
}
