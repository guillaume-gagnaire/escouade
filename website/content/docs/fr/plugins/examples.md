# Les plugins d’exemple

Quatre plugins complets à copier, lire et installer tels quels : un thème, un webhook Discord sans code, Slack et les règles d’une équipe en TypeScript. Chacun passe le banc de conformité à chaque changement.

## Où les trouver

Ils sont dans le dossier `plugin-examples/` du dépôt d’Escouade, un sous-dossier par plugin, chacun lisible seul et sous licence MIT : https://github.com/guillaume-gagnaire/escouade/tree/main/plugin-examples

| Exemple | Ce qu’il montre | Genre |
| --- | --- | --- |
| Contraste élevé | Un thème, sombre et clair, et une palette de couleurs de projet | Déclaratif |
| Webhook Discord | Les tickets à tester et les agents qui attendent, dans un salon Discord | Déclaratif |
| Slack | La même chose dans Slack, dans le salon de chaque projet, avec un secret au trousseau | Programme TypeScript |
| Règles de l’équipe | Les commandes des agents autorisées ou refusées selon un fichier du dépôt | Programme TypeScript |

Le code montré ici est celui du dépôt : un test du site vérifie qu’il n’a pas changé depuis. Gemini CLI, lui, est livré avec Escouade ; l’exemple d’un harness décrit par un plugin viendra avec ce point d’extension.

## Les essayer

Dans **Plugins**, « En développement », choisis le dossier d’un exemple : il est chargé en place et relu quand il change. Ses secrets se saisissent dans l’onglet « Réglages » de sa fiche.

Pour les vérifier comme la CI du dépôt : les exemples en TypeScript passent leurs tests et leur vérification de types, leur `dist/plugin.mjs` est comparé à la compilation de `src/`, puis chacun passe `escouade-plugin validate` et le banc (voir [Tester un plugin](plugins/testing)).

```sh
npx tsc -p sdk/plugin/tsconfig.json
ESCOUADE_PLUGIN_BIN=/chemin/vers/escouade-plugin node plugin-examples/ci.mjs
```

## Contraste élevé

Un thème sans code : des valeurs pour les jetons de la fenêtre, en sombre et en clair, et une palette pour les couleurs de projet. Escouade filtre chaque valeur avant de la poser ; un jeton absent garde celle d’Escouade. Voir [Les thèmes](plugins/themes).

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

Choisis-le dans Réglages › Application, rangée « Couleurs ».

## Webhook Discord

Deux webhooks déclarés, sans programme : un ticket passé dans « À tester », et un agent qui attend ta réponse. L’adresse du webhook Discord est un secret : elle reste dans le trousseau du système et n’apparaît jamais au journal. Voir [Les événements](plugins/events#un-webhook-sans-code).

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

Dans Discord, crée le webhook dans les réglages du salon (Intégrations › Webhooks), puis colle son adresse dans l’onglet « Réglages » du plugin. `when` ne laisse passer que l’arrivée dans « À tester » et le statut `waiting` ; les autres déplacements et statuts ne disent rien.

## Slack

Un programme en TypeScript, écrit avec le [SDK](plugins/sdk). Il poste dans le salon de chaque projet : celui que donne `channels.json`, sinon `escouade-<nom du projet>`. Le jeton du bot (`xoxb-…`) est demandé à Escouade à chaque fait (`secrets/get`), jamais écrit dans un fichier.

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

Dans Slack, crée une app avec un bot qui a le droit `chat:write`, invite-le dans les salons, puis colle son jeton dans l’onglet « Réglages » du plugin. Sans jeton, le journal le dit et rien ne part.

Ses tests jouent ses gestionnaires contre le faux Escouade du SDK et un faux Slack, sans réseau (`npm test`). `npm run build` compile `src/` dans `dist/plugin.mjs`, ce que lancent `slack` et `slack.cmd` : Escouade lance le plugin tel que le dépôt le contient, sans `npm install`.

## Règles de l’équipe

Un programme en TypeScript qui décide des autorisations avant qu’elles n’arrivent jusqu’à toi (voir [Les règles d’autorisation](plugins/permission-rules)). Ses règles sont dans `rules.json`, un fichier du dépôt du plugin que l’équipe relit comme son code. Une ligne qui contient une commande refusée est refusée, même après un `&&` ; une ligne faite seulement de commandes permises est autorisée ; le reste va à l’utilisateur.

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

Les règles sont lues une fois, au démarrage : une décision a 500 ms. Une modification ou une lecture de fichier n’est pas une commande, et va à l’utilisateur.
