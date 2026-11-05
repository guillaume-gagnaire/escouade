# Example plugins

Five complete plugins to read, copy and install: a theme, a Discord webhook with no code, Slack and a team’s rules in TypeScript, and Goose, a harness described with no code. They are Escouade’s own plugins, on its store; each one passes the conformance bench on every change.

## Where to find them

They are in the `plugins/` folder of Escouade’s repository, one folder per plugin (`plugins/slack/`…), each readable on its own with its manifest, its README, its LICENSE (MIT), its CHANGELOG and its tests: https://github.com/guillaume-gagnaire/escouade/tree/main/plugins. The store publishes them as `dev.escouade.high-contrast`, `dev.escouade.discord`, `dev.escouade.slack`, `dev.escouade.team-rules` and `dev.escouade.goose`, taken from these folders at their tags `plugins/<folder>/v1.2.3` (see [Publishing a plugin](plugins/publishing)).

To make one yours, copy its folder and give it an id of your own (`io.github.<your-login>.slack`): `dev.escouade.*` is kept for Escouade’s own plugins, and the store takes it from Escouade’s repository alone.

| Example | What it shows | Kind |
| --- | --- | --- |
| High contrast | A theme, dark and light, and a palette of project colors | Declarative |
| Discord | Tickets to review and agents waiting, in a Discord channel | Declarative |
| Slack | The same in Slack, in each project’s channel, with a secret in the keychain | TypeScript program |
| Team rules | The agents’ commands allowed or denied by a file of the repository | TypeScript program |
| Goose | An agent that speaks ACP, added as a harness | Declarative |
| Linear | A source of the Kanban, on the kit | Rust module |

The code shown here is the repository’s: a test of the site checks it has not changed since.

> [!NOTE]
> Slack and Team rules are `run.js` plugins since their 2.0.0: Escouade runs their `dist/plugin.mjs` inside the app, in WASM, with no Node on the machine (see [the protocol](plugins/api#a-program-in-wasm)). They need Escouade 2.2.0 or later (`escouade.min`); their 1.x versions, started through Node, stay on the store for the versions before. Node only serves their authors, to build and test them.

## Trying them

Install them from the store, in **Plugins**. To change one: “In development”, pick an example’s folder; it is loaded in place and read again when it changes. Enter its secrets in the “Settings” tab of its page.

To check them as the repository’s CI does: each one passes its tests (and, in TypeScript, its typecheck; its `dist/plugin.mjs` is compared to the build of `src/`), then `escouade-plugin validate` and the bench, with `--first-party` for their reserved ids (see [Testing a plugin](plugins/testing)).

```sh
npx tsc -p sdk/plugin/tsconfig.json
ESCOUADE_PLUGIN_BIN=/path/to/escouade-plugin node plugins/ci.mjs
```

## High contrast

A theme with no code: values for the window’s tokens, dark and light, and a palette for project colors. Escouade filters each value before it sets it; a token left out keeps Escouade’s. See [Themes](plugins/themes).

`plugins/high-contrast/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.high-contrast",
  "name": { "fr": "Contraste élevé", "en": "High contrast" },
  "version": "1.1.0",
  "description": {
    "fr": "Un thème à fort contraste, clair et sombre : texte pur, bordures nettes, couleurs d'état franches.",
    "en": "A high-contrast theme, light and dark: pure text, sharp borders, plain status colors."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "repository": "https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/high-contrast",
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

## Discord

Two declared webhooks, no program: a ticket moved to “To review”, and an agent waiting for your answer. The Discord webhook’s address is a secret: it stays in the system’s keychain and never shows in the journal. See [Events](plugins/events#a-webhook-without-code).

`plugins/discord/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.discord",
  "name": "Discord",
  "version": "1.1.0",
  "description": {
    "fr": "Les tickets à tester et les agents qui attendent, dans un salon Discord : un webhook, sans code.",
    "en": "Tickets to test and agents waiting, in a Discord channel: a webhook, no code."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "repository": "https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/discord",
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
        "body": {
          "content": "🧪 **{{ticket.key}}** to test: {{ticket.title}} ({{project.name}})",
          "allowed_mentions": { "parse": [] }
        }
      },
      {
        "url": "{{secrets.webhook}}",
        "events": ["agent.status"],
        "when": { "data.status": "waiting" },
        "body": {
          "content": "✋ **{{agent.name}}** waits for you ({{project.name}})",
          "allowed_mentions": { "parse": [] }
        }
      }
    ]
  }
}
```

In Discord, create the webhook in the channel’s settings (Integrations › Webhooks), then paste its address in the plugin’s “Settings” tab. `when` lets through only the move to “To review” and the `waiting` status; the other moves and statuses say nothing. An empty `allowed_mentions`: an `@everyone` in the title of a ticket from elsewhere (a public issue) shows without notifying the channel.

## Slack

A TypeScript program, written with the [SDK](plugins/sdk). It posts in each project’s channel: the one `channels.json` gives, else `escouade-<project name>`. The bot’s token (`xoxb-…`) is asked of Escouade on each fact (`secrets/get`), never written in a file.

`plugins/slack/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.slack",
  "name": "Slack",
  "version": "2.0.0",
  "description": {
    "fr": "Les tickets à tester et les agents qui attendent, dans le salon Slack de chaque projet.",
    "en": "Tickets to test and agents waiting, in each project's Slack channel."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "repository": "https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/slack",
  "escouade": { "api": "1", "min": "2.2.0" },
  "run": { "js": "dist/plugin.mjs" },
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

`plugins/slack/channels.json`

```json
{
  "Escouade": "escouade-dev"
}
```

`plugins/slack/src/plugin.ts`

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

/** A text from elsewhere (a ticket's title may come from a public issue), as Slack asks: `<!channel>` or `<@U…>` in it
 * is shown, never a mention that notifies. */
export const escaped = (text: string) => text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

/** What a fact says in Slack, or nothing for the facts it keeps quiet about. */
export function textOf(fact: Fact): string | null {
  if (fact.name === 'ticket.column' && fact.data.to === 'review') {
    return `:test_tube: *${escaped(fact.ticket?.key ?? '')}* to test: ${escaped(fact.ticket?.title ?? '')}`;
  }
  if (fact.name === 'agent.status' && fact.data.status === 'waiting') {
    return `:raised_hand: *${escaped(fact.agent?.name ?? 'An agent')}* waits for you`;
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

`plugins/slack/src/main.ts`

```ts
// The program Escouade starts (run in escouade-plugin.json): speaks the protocol on stdin and stdout.
import { slack } from './plugin.ts';

void slack().start();
```

In Slack, create an app with a bot that has the `chat:write` scope, invite it to the channels, then paste its token in the plugin’s “Settings” tab. Without a token, the journal says so and nothing is sent. A ticket’s title is escaped (`&`, `<`, `>`) as Slack asks: a `<!channel>` from a public issue shows without notifying anyone.

Its tests drive its handlers against the SDK’s fake Escouade and a fake Slack, with no network (`npm test`). `npm run build` bundles `src/` into `dist/plugin.mjs`, which `run.js` names: committed, it is the bundle the store compiles to WASM, without `npm install`. `start()` speaks to Escouade’s WASM host, or on standard input and output under Node (its tests, the bench). Its `fetch` to `slack.com/api/chat.postMessage` goes through Escouade, which lets it reach `slack.com` and its subdomains only, over https; `channels.json`, a JSON file at the root of its folder, is given to it at its start, and `readFileSync` reads it as under Node.

## Team rules

A TypeScript program that decides permissions before they reach you (see [Permission rules](plugins/permission-rules)). Its rules are in `rules.json`, a file of the plugin’s repository that the team reviews like its code. A line that holds a denied command is denied, even after a `&&` or a `&`; a line made only of allowed commands is allowed, unless it holds a substitution (`$(…)`, backquotes, `$VAR`), a redirection (`>`, `<`), an escape, which would slip something else behind an allowed start, or an `--output` (with it, `git diff` and `git log` write a file); the rest goes to the user. A rule compares the start of a command: `deny` is a safety net (`sudo rm -rf` slips past it and goes to the user), and `allow` allows any arguments.

`plugins/team-rules/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.team-rules",
  "name": { "fr": "Règles de l'équipe", "en": "Team rules" },
  "version": "2.0.0",
  "description": {
    "fr": "Autorise ou refuse les commandes des agents selon les règles de l'équipe, écrites dans rules.json.",
    "en": "Allows or denies the agents' commands by the team's rules, written in rules.json."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "repository": "https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/team-rules",
  "escouade": { "api": "1", "min": "2.2.0" },
  "run": { "js": "dist/plugin.mjs" },
  "activation": ["event:request.permission"],
  "permissions": { "decide": true }
}
```

`plugins/team-rules/rules.json`

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

`plugins/team-rules/src/plugin.ts`

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

/** An option that makes a command write a file of its own choosing: `git diff --output=~/.bashrc`, `git log --output …`
 * (git takes no abbreviation of it). A redirection by another name: a line that holds it, its quotes taken out as the
 * shell takes them out, is never allowed. */
const WRITES = /(^|\s)--output(=|\s|$)/;
const writes = (line: string) => WRITES.test(line.replace(/["']/g, ''));

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
  const plain = !HIDDEN.test(line) && !writes(line);
  if (plain && commands.length > 0 && commands.every((c) => rules.allow.some((rule) => startsWith(c, rule)))) {
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

`plugins/team-rules/src/main.ts`

```ts
// The program Escouade starts (run in escouade-plugin.json): speaks the protocol on stdin and stdout.
import { teamRules } from './plugin.ts';

void teamRules().start();
```

The rules are read once, at start: a decision has 500 ms. In WASM, `rules.json` is given to it at its start with the other JSON files at the root of its folder; it starts nothing and reaches no network (it has no `permissions.network`). An edit or a file read is not a command, and goes to the user.

## Linear

A source of the Kanban in Rust: the kit’s `Hosts` answers the protocol, and the plugin gives its kind and its client. The `npm create escouade-plugin -- --kind rust` template, shorter, starts from a blank page; Linear shows a real service. It is read in [the Rust kit](plugins/rust#writing-a-source), and its code is in [`plugins/linear`](https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/linear).
`plugins/linear/src/lib.rs` (excerpt)

```rust
impl SourceKind for LinearSource {
    fn id(&self) -> &str {
        ID
    }
    fn name(&self) -> &str {
        S
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn TicketSource> {
        let base = account.api.clone().filter(|a| !a.trim().is_empty());
        Box::new(Linear::new(http, account, base.as_deref().unwrap_or(API)))
    }
}
```


## Goose

A harness without code: Goose, Block’s open source agent, described over ACP. The ACP plugin shipped with Escouade launches it and shows its conversation; the manifest says its modes, its mode that changes nothing, its model option and the names of its tools. Goose writes its files itself: no agent of a private project starts on it. See [A described ACP harness](plugins/harnesses).

`plugins/goose/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.goose",
  "name": "Goose",
  "version": "1.0.0",
  "description": {
    "fr": "Goose, l'agent open source de Block, comme harness : décrit par ACP, sans code.",
    "en": "Goose, Block's open source agent, as a harness: described over ACP, no code."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "repository": "https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/goose",
  "escouade": { "api": "1", "min": "2.1.0" },
  "contributes": {
    "harnesses": [
      {
        "id": "goose",
        "name": "Goose",
        "speaker": "Goose",
        "writer": "Goose",
        "locate": { "names": ["goose"] },
        "tested": { "oldest": "1.57.0", "newest": "1.57.0" },
        "caps": {
          "modes": [
            {
              "id": "default",
              "meaning": {
                "fr": "Goose demande avant chaque outil (son mode approve). Il écrit les fichiers lui-même, sans les faire passer par Escouade.",
                "en": "Goose asks before each tool (its approve mode). It writes files by itself, without passing them through Escouade."
              }
            },
            {
              "id": "acceptEdits",
              "meaning": {
                "fr": "Goose ne demande que pour ce qu'il juge sensible (son mode smart_approve) ; Escouade accepte ses écritures dans le dossier de l'agent.",
                "en": "Goose asks only for what it deems sensitive (its smart_approve mode); Escouade accepts its writes in the agent's folder."
              }
            },
            {
              "id": "bypassPermissions",
              "meaning": {
                "fr": "Goose ne demande rien (son mode auto) : commandes, fichiers, tout le disque. Pas de bac à sable.",
                "en": "Goose asks nothing (its auto mode): commands, files, the whole disk. No sandbox."
              }
            }
          ],
          "readOnly": true
        },
        "accountVar": "GOOSE_PATH_ROOT",
        "install": {
          "windows": "Invoke-WebRequest -Uri https://raw.githubusercontent.com/aaif-goose/goose/main/download_cli.ps1 -OutFile download_cli.ps1; ./download_cli.ps1",
          "unix": "curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash"
        },
        "updateCommand": "goose update",
        "acp": {
          "args": ["acp"],
          "fs": false,
          "modes": { "default": "approve", "acceptEdits": "smart_approve", "bypassPermissions": "auto" },
          "readOnlyMode": "chat",
          "modelOption": "model",
          "questionsInText": true,
          "tools": { "shell": "Bash", "write": "Write", "edit": "Edit", "text_editor": "Edit", "read": "Read", "tree": "Glob" }
        }
      }
    ]
  }
}
```
