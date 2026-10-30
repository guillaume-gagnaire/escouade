# Example plugins

Four complete plugins to copy, read and install as they are: a theme, a Discord webhook with no code, and Slack and a team’s rules in TypeScript. Each one passes the conformance bench on every change.

## Where to find them

They are in the `plugin-examples/` folder of Escouade’s repository, one folder per plugin, each readable on its own and under the MIT license: https://github.com/guillaume-gagnaire/escouade/tree/main/plugin-examples

| Example | What it shows | Kind |
| --- | --- | --- |
| High contrast | A theme, dark and light, and a palette of project colors | Declarative |
| Discord webhook | Tickets to review and agents waiting, in a Discord channel | Declarative |
| Slack | The same in Slack, in each project’s channel, with a secret in the keychain | TypeScript program |
| Team rules | The agents’ commands allowed or denied by a file of the repository | TypeScript program |

The code shown here is the repository’s: a test of the site checks it has not changed since. Gemini CLI ships with Escouade; the example of a harness described by a plugin will come with that extension point.

## Trying them

In **Plugins**, “In development”, pick an example’s folder: it is loaded in place and read again when it changes. Enter its secrets in the “Settings” tab of its page.

To check them as the repository’s CI does: the TypeScript examples pass their tests and their typecheck, their `dist/plugin.mjs` is compared to the build of `src/`, then each one passes `escouade-plugin validate` and the bench (see [Testing a plugin](plugins/testing)).

```sh
npx tsc -p sdk/plugin/tsconfig.json
ESCOUADE_PLUGIN_BIN=/path/to/escouade-plugin node plugin-examples/ci.mjs
```

## High contrast

A theme with no code: values for the window’s tokens, dark and light, and a palette for project colors. Escouade filters each value before it sets it; a token left out keeps Escouade’s. See [Themes](plugins/themes).

`plugin-examples/high-contrast/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.exemple.high-contrast",
  "name": { "fr": "Contraste élevé", "en": "High contrast" },
  "version": "1.0.0",
  "description": {
    "fr": "Un thème à fort contraste, clair et sombre : texte pur, bordures nettes, couleurs d'état franches.",
    "en": "A high-contrast theme, light and dark: pure text, sharp borders, plain status colors."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "escouade": { "api": "1", "min": "2.1.0" },
  "contributes": {
    "themes": [
      {
        "id": "high-contrast",
        "name": { "fr": "Contraste élevé", "en": "High contrast" },
        "dark": {
          "--frame": "#000000",
          "--panel": "color-mix(in oklch, var(--p) 4%, #050505)",
          "--raise": "#141414",
          "--raise-2": "#1f1f1f",
          "--well": "#000000",
          "--line": "#8a8a8a",
          "--line-2": "#b0b0b0",
          "--line-3": "#d8d8d8",
          "--text": "#ffffff",
          "--soft": "#f0f0f0",
          "--muted": "#d0d0d0",
          "--dim": "#b8b8b8",
          "--p-ink": "#ffd400",
          "--ring": "#ffd400",
          "--run": "#4dff88",
          "--wait": "#ffb000",
          "--done": "#7ab8ff",
          "--add": "#4dff88",
          "--del": "#ff6b6b"
        },
        "light": {
          "--frame": "#ffffff",
          "--panel": "#ffffff",
          "--raise": "#f2f2f2",
          "--raise-2": "#e6e6e6",
          "--well": "#ffffff",
          "--line": "#5c5c5c",
          "--line-2": "#3d3d3d",
          "--line-3": "#1f1f1f",
          "--text": "#000000",
          "--soft": "#0a0a0a",
          "--muted": "#262626",
          "--dim": "#3d3d3d",
          "--p-ink": "#0040c0",
          "--ring": "#0040c0",
          "--run": "#006b2e",
          "--wait": "#8a4b00",
          "--done": "#0040c0",
          "--add": "#006b2e",
          "--del": "#b00020"
        },
        "projectColors": ["#ffd400", "#00e0ff", "#ff5cf0", "#7cff4f", "#ff8a00", "#b48cff"]
      }
    ]
  }
}
```

Pick it in Settings › Application, “Colors” row.

## Discord webhook

Two declared webhooks, no program: a ticket moved to “To review”, and an agent waiting for your answer. The Discord webhook’s address is a secret: it stays in the system’s keychain and never shows in the journal. See [Events](plugins/events#a-webhook-without-code).

`plugin-examples/discord-webhook/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.exemple.discord",
  "name": "Discord",
  "version": "1.0.0",
  "description": {
    "fr": "Les tickets à tester et les agents qui attendent, dans un salon Discord : un webhook, sans code.",
    "en": "Tickets to test and agents waiting, in a Discord channel: a webhook, no code."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "escouade": { "api": "1", "min": "2.1.0" },
  "permissions": {
    "events": ["ticket.column", "agent.status"],
    "network": ["discord.com"],
    "secrets": ["webhook"]
  },
  "contributes": {
    "secrets": [{ "key": "webhook", "title": { "fr": "Adresse du webhook Discord", "en": "Discord webhook URL" } }],
    "webhooks": [
      {
        "url": "{{secrets.webhook}}",
        "events": ["ticket.column"],
        "when": { "data.to": "review" },
        "body": { "content": "🧪 **{{ticket.key}}** to test: {{ticket.title}} ({{project.name}})" }
      },
      {
        "url": "{{secrets.webhook}}",
        "events": ["agent.status"],
        "when": { "data.status": "waiting" },
        "body": { "content": "✋ **{{agent.name}}** waits for you ({{project.name}})" }
      }
    ]
  }
}
```

In Discord, create the webhook in the channel’s settings (Integrations › Webhooks), then paste its address in the plugin’s “Settings” tab. `when` lets through only the move to “To review” and the `waiting` status; the other moves and statuses say nothing.

## Slack

A TypeScript program, written with the [SDK](plugins/sdk). It posts in each project’s channel: the one `channels.json` gives, else `escouade-<project name>`. The bot’s token (`xoxb-…`) is asked of Escouade on each fact (`secrets/get`), never written in a file.

`plugin-examples/slack/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.exemple.slack",
  "name": "Slack",
  "version": "1.0.0",
  "description": {
    "fr": "Les tickets à tester et les agents qui attendent, dans le salon Slack de chaque projet.",
    "en": "Tickets to test and agents waiting, in each project's Slack channel."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "escouade": { "api": "1", "min": "2.1.0" },
  "run": { "command": "slack", "windows": "slack.cmd" },
  "activation": ["event:ticket.column", "event:agent.status"],
  "permissions": {
    "events": ["ticket.column", "agent.status"],
    "network": ["slack.com"],
    "secrets": ["token"]
  },
  "contributes": {
    "secrets": [{ "key": "token", "title": { "fr": "Jeton du bot Slack (xoxb-…)", "en": "Slack bot token (xoxb-…)" } }]
  }
}
```

`plugin-examples/slack/channels.json`

```json
{
  "Escouade": "escouade-dev"
}
```

`plugin-examples/slack/src/plugin.ts`

```ts
// Slack: the tickets to test and the agents waiting, in each project's channel. The bot's token is kept in the
// system's keychain (permissions.secrets), never in a file; the channel of a project comes from channels.json, or
// from the project's name (escouade-<name>).
import { readFileSync } from 'node:fs';
import { createPlugin, type Fact, type Plugin } from '@escouade-dev/plugin';

export interface Options {
  /** The channel of each project, by the project's name. */
  channels?: Record<string, string>;
}

/** channels.json, beside escouade-plugin.json (src/ and dist/ are one level below it). */
function readChannels(): Record<string, string> {
  try {
    return JSON.parse(readFileSync(new URL('../channels.json', import.meta.url), 'utf8'));
  } catch {
    return {};
  }
}

/** The channel of a project: the one channels.json gives it, else escouade-<its name>, as Slack writes names. */
export function channelOf(project: string, channels: Record<string, string>): string {
  const named = channels[project];
  if (named) return named.replace(/^#/, '');
  const slug = project
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return `escouade-${slug || 'project'}`.slice(0, 80);
}

/** What a fact says in Slack, or nothing for the facts it keeps quiet about. */
export function textOf(fact: Fact): string | null {
  if (fact.name === 'ticket.column' && fact.data.to === 'review') {
    return `:test_tube: *${fact.ticket?.key ?? ''}* to test: ${fact.ticket?.title ?? ''}`;
  }
  if (fact.name === 'agent.status' && fact.data.status === 'waiting') {
    return `:raised_hand: *${fact.agent?.name ?? 'An agent'}* waits for you`;
  }
  return null;
}

export function slack(options: Options = {}): Plugin {
  const channels = options.channels ?? readChannels();

  return createPlugin().onEvent(['ticket.column', 'agent.status'], async (fact, p) => {
    const text = textOf(fact);
    if (!text || !fact.project) return;
    const token = await p.secret('token');
    if (!token) return p.log.warn('no Slack token kept: nothing sent (Plugins › Slack › Settings)');
    // A bot token, never anything else, goes to Slack (and never into the journal).
    if (!token.startsWith('xoxb-')) return p.log.warn('the token kept is not a Slack bot token (xoxb-…): nothing sent');
    const channel = channelOf(fact.project.name, channels);
    try {
      const res = await fetch('https://slack.com/api/chat.postMessage', {
        method: 'POST',
        headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json; charset=utf-8' },
        body: JSON.stringify({ channel, text }),
      });
      const answer = (await res.json()) as { ok: boolean; error?: string };
      if (answer.ok) p.log.info(`posted in #${channel}`);
      else p.log.warn(`Slack refused the message in #${channel}: ${answer.error ?? res.status}`);
    } catch (e) {
      p.log.error(`Slack could not be reached: ${(e as Error).message}`);
    }
  });
}
```

`plugin-examples/slack/src/main.ts`

```ts
// The program Escouade starts (run in escouade-plugin.json): speaks the protocol on stdin and stdout.
import { slack } from './plugin.ts';

void slack().start();
```

In Slack, create an app with a bot that has the `chat:write` scope, invite it to the channels, then paste its token in the plugin’s “Settings” tab. Without a token, the journal says so and nothing is sent.

Its tests drive its handlers against the SDK’s fake Escouade and a fake Slack, with no network (`npm test`). `npm run build` bundles `src/` into `dist/plugin.mjs`, which `slack` and `slack.cmd` start: Escouade runs the plugin as the repository holds it, without `npm install`.

## Team rules

A TypeScript program that decides permissions before they reach you (see [Permission rules](plugins/permission-rules)). Its rules are in `rules.json`, a file of the plugin’s repository that the team reviews like its code. A line that holds a denied command is denied, even after a `&&`; a line made only of allowed commands is allowed; the rest goes to the user.

`plugin-examples/team-rules/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.exemple.team-rules",
  "name": { "fr": "Règles de l'équipe", "en": "Team rules" },
  "version": "1.0.0",
  "description": {
    "fr": "Autorise ou refuse les commandes des agents selon les règles de l'équipe, écrites dans rules.json.",
    "en": "Allows or denies the agents' commands by the team's rules, written in rules.json."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "escouade": { "api": "1", "min": "2.1.0" },
  "run": { "command": "team-rules", "windows": "team-rules.cmd" },
  "activation": ["event:request.permission"],
  "permissions": { "decide": true }
}
```

`plugin-examples/team-rules/rules.json`

```json
{
  "deny": [
    "rm -rf",
    "git reset --hard",
    { "command": "git push --force", "message": "No force push in this team: push a new commit instead." },
    { "command": "git push -f", "message": "No force push in this team: push a new commit instead." }
  ],
  "allow": ["npm test", "npm run lint", "cargo test", "cargo nextest", "git status", "git diff", "git log"]
}
```

`plugin-examples/team-rules/src/plugin.ts`

```ts
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

/** The commands of a line: `cd app && rm -rf build` is two, each judged on its own. */
export function commandsOf(line: string): string[] {
  return line
    .split(/&&|\|\||[;|\n]/)
    .map((c) => c.trim().replace(/\s+/g, ' '))
    .filter(Boolean);
}

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
  if (commands.length > 0 && commands.every((c) => rules.allow.some((rule) => startsWith(c, rule)))) {
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
```

`plugin-examples/team-rules/src/main.ts`

```ts
// The program Escouade starts (run in escouade-plugin.json): speaks the protocol on stdin and stdout.
import { teamRules } from './plugin.ts';

void teamRules().start();
```

The rules are read once, at start: a decision has 500 ms. An edit or a file read is not a command, and goes to the user.
