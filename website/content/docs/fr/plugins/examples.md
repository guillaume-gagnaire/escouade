# Les plugins d’exemple

Cinq plugins complets à lire, copier et installer : un thème, un webhook Discord sans code, Slack et les règles d’une équipe en TypeScript, et Goose, un harness décrit sans code. Ce sont des plugins d’Escouade, sur son store ; chacun passe le banc de conformité à chaque changement.

## Où les trouver

Ils sont dans le dossier `plugins/` du dépôt d’Escouade, un sous-dossier par plugin (`plugins/slack/`…), chacun lisible seul avec son manifeste, son README, sa LICENSE (MIT), son CHANGELOG et ses tests : https://github.com/guillaume-gagnaire/escouade/tree/main/plugins. Le store les publie sous les identifiants `dev.escouade.high-contrast`, `dev.escouade.discord`, `dev.escouade.slack`, `dev.escouade.team-rules` et `dev.escouade.goose`, pris de ces dossiers à leurs étiquettes `plugins/<dossier>/v1.2.3` (voir [Publier un plugin](plugins/publishing)).

Pour en faire le tien, copie son dossier et donne-lui un identifiant à toi (`io.github.<ton-login>.slack`) : `dev.escouade.*` est réservé aux plugins d’Escouade, et le store ne l’accepte que du dépôt d’Escouade.

| Exemple | Ce qu’il montre | Genre |
| --- | --- | --- |
| Contraste élevé | Un thème, sombre et clair, et une palette de couleurs de projet | Déclaratif |
| Discord | Les tickets à tester et les agents qui attendent, dans un salon Discord | Déclaratif |
| Slack | La même chose dans Slack, dans le salon de chaque projet, avec un secret au trousseau | Programme TypeScript |
| Règles de l’équipe | Les commandes des agents autorisées ou refusées selon un fichier du dépôt | Programme TypeScript |
| Goose | Un agent qui parle ACP, ajouté comme harness | Déclaratif |
| Linear | Une source du Kanban, sur le kit | Module Rust |

Le code montré ici est celui du dépôt : un test du site vérifie qu’il n’a pas changé depuis.

> [!NOTE]
> Slack et Règles de l’équipe sont en `run.js` depuis leur 2.0.0 : Escouade fait tourner leur `dist/plugin.mjs` dans l’app, en WASM, sans Node sur la machine (voir [le protocole](plugins/api#un-programme-en-wasm)). Il leur faut Escouade 2.2.0 ou plus récent (`escouade.min`) ; leurs versions 1.x, lancées par Node, restent au store pour les versions d’avant. Node ne sert qu’à leurs auteurs, pour les construire et les tester.

## Les essayer

Installe-les depuis le store, dans **Plugins**. Pour les modifier : « En développement », choisis le dossier d’un exemple ; il est chargé en place et relu quand il change. Ses secrets se saisissent dans l’onglet « Réglages » de sa fiche.

Pour les vérifier comme la CI du dépôt : chacun passe ses tests (et, en TypeScript, sa vérification de types ; son `dist/plugin.mjs` est comparé à la compilation de `src/`), puis `escouade-plugin validate` et le banc, avec `--first-party` pour leurs identifiants réservés (voir [Tester un plugin](plugins/testing)).

```sh
npx tsc -p sdk/plugin/tsconfig.json
ESCOUADE_PLUGIN_BIN=/chemin/vers/escouade-plugin node plugins/ci.mjs
```

## Contraste élevé

Un thème sans code : des valeurs pour les jetons de la fenêtre, en sombre et en clair, et une palette pour les couleurs de projet. Escouade filtre chaque valeur avant de la poser ; un jeton absent garde celle d’Escouade. Voir [Les thèmes](plugins/themes).

`plugins/high-contrast/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.high-contrast",
  "name": { "fr": "Contraste élevé", "en": "High contrast" },
  "version": "1.1.2",
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
  },
  "store": { "docs": "docs" }
}
```

Choisis-le dans Réglages › Général, rangée « Couleurs ».

## Discord

Deux webhooks déclarés, sans programme : un ticket passé dans « À tester », et un agent qui attend ta réponse. L’adresse du webhook Discord est un secret : elle reste dans le trousseau du système et n’apparaît jamais au journal. Voir [Les événements](plugins/events#un-webhook-sans-code).

`plugins/discord/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.discord",
  "name": "Discord",
  "version": "1.1.2",
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
  },
  "store": { "docs": "docs" }
}
```

Dans Discord, crée le webhook dans les réglages du salon (Intégrations › Webhooks), puis colle son adresse dans l’onglet « Réglages » du plugin. `when` ne laisse passer que l’arrivée dans « À tester » et le statut `waiting` ; les autres déplacements et statuts ne disent rien. `allowed_mentions` vide : un `@everyone` dans le titre d’un ticket venu d’ailleurs (une issue publique) s’affiche sans notifier le salon.

## Slack

Un programme en TypeScript, écrit avec le [SDK](plugins/sdk). Il poste dans le salon de chaque projet : celui que donne `channels.json`, sinon `escouade-<nom du projet>`. Le jeton du bot (`xoxb-…`) est demandé à Escouade à chaque fait (`secrets/get`), jamais écrit dans un fichier.

`plugins/slack/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.slack",
  "name": "Slack",
  "version": "2.0.2",
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
  },
  "store": { "docs": "docs" }
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

Dans Slack, crée une app avec un bot qui a le droit `chat:write`, invite-le dans les salons, puis colle son jeton dans l’onglet « Réglages » du plugin. Sans jeton, le journal le dit et rien ne part. Le titre d’un ticket est échappé (`&`, `<`, `>`) comme Slack le demande : un `<!channel>` venu d’une issue publique s’affiche sans notifier personne.

Ses tests jouent ses gestionnaires contre le faux Escouade du SDK et un faux Slack, sans réseau (`npm test`). `npm run build` réunit `src/` dans `dist/plugin.mjs`, que nomme `run.js` : commité, c’est le bundle que le store compile en WASM, sans `npm install`. `start()` parle à l’hôte WASM d’Escouade, ou sur l’entrée et la sortie standard sous Node (ses tests, le banc). Son `fetch` vers `slack.com/api/chat.postMessage` passe par Escouade, qui ne le laisse joindre que `slack.com` et ses sous-domaines, en https ; `channels.json`, un fichier JSON de la racine de son dossier, lui est donné au démarrage, et `readFileSync` le lit comme sous Node.

## Règles de l’équipe

Un programme en TypeScript qui décide des autorisations avant qu’elles n’arrivent jusqu’à toi (voir [Les règles d’autorisation](plugins/permission-rules)). Ses règles sont dans `rules.json`, un fichier du dépôt du plugin que l’équipe relit comme son code. Une ligne qui contient une commande refusée est refusée, même après un `&&` ou un `&` ; une ligne faite seulement de commandes permises est autorisée, sauf si elle contient une substitution (`$(…)`, accents graves, `$VAR`), une redirection (`>`, `<`), un échappement, qui feraient passer autre chose derrière un début permis, ou un `--output` (avec lui, `git diff` et `git log` écrivent un fichier) ; le reste va à l’utilisateur. Une règle compare le début d’une commande : `deny` est un filet (`sudo rm -rf` lui échappe et va à l’utilisateur), et `allow` permet tous les arguments.

`plugins/team-rules/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.team-rules",
  "name": { "fr": "Règles de l'équipe", "en": "Team rules" },
  "version": "2.0.1",
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
  "permissions": { "decide": true },
  "store": { "docs": "docs" }
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

Les règles sont lues une fois, au démarrage : une décision a 500 ms. En WASM, `rules.json` lui est donné au démarrage avec les autres fichiers JSON de la racine de son dossier ; il ne lance rien et n’atteint aucun réseau (il n’a pas de `permissions.network`). Une modification ou une lecture de fichier n’est pas une commande, et va à l’utilisateur.

## Linear

Une source du Kanban en Rust : `Hosts` du kit répond le protocole, et le plugin donne sa sorte et son client. Le modèle `npm create escouade-plugin -- --kind rust`, plus court, part d’une page blanche ; Linear montre un vrai service. Il est lu dans [le kit Rust](plugins/rust#ecrire-une-source), et son code est dans [`plugins/linear`](https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/linear).
`plugins/linear/src/lib.rs` (extrait)

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

Un harness sans code : Goose, l’agent open source de Block, décrit par ACP. Le plugin ACP livré avec Escouade le lance et montre sa conversation ; le manifeste dit ses modes, son mode qui ne change rien, son option de modèle et le nom de ses outils. Goose écrit ses fichiers lui-même : aucun agent d’un projet privé ne démarre sur lui. Voir [Un harness ACP décrit](plugins/harnesses).

`plugins/goose/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.goose",
  "name": "Goose",
  "version": "1.0.2",
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
  },
  "store": { "categories": ["harness"], "docs": "docs" }
}
```
