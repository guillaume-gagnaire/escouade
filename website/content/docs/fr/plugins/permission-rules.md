# Les règles d’autorisation

Un plugin peut décider des autorisations que demandent tes agents avant qu’elles n’arrivent jusqu’à toi : autoriser une commande sûre, en refuser une dangereuse, et te laisser le reste. Les demandes qui t’interrompent le plus deviennent des règles, sans passer en mode Bypass.

C’est la permission la plus forte d’un plugin : elle est montrée **en rouge** à l’installation (« Décide des autorisations de tes agents »), chacune de ses décisions va à son journal, et aucun projet ne peut la donner : un `.escouade.yml` n’allume jamais un plugin.

## Comment Escouade décide

Quand un agent demande une autorisation (une commande, une modification de fichier, une page web, un outil MCP), quel que soit son harness :

1. Escouade la pose à chaque plugin qui a `permissions.decide`, allumé pour le projet de l’agent, **tous en même temps**.
2. Chacun a **500 ms** pour répondre : `allow`, `deny` avec un message, ou `ask`.
3. **Un refus l’emporte**, quel que soit l’ordre des réponses. Sinon, une autorisation suffit. Sinon (tous ont répondu `ask`, ou se sont tus, ont planté, ont répondu autre chose), la demande va à l’utilisateur comme sans plugin.

Pendant ce temps, la carte de la demande est déjà dans la conversation, mais Escouade ne te notifie pas : tu n’es appelé que si aucun plugin n’a tranché. Tu peux toujours répondre toi-même sur la carte ; ta réponse arrivée la première est gardée.

Un plan à approuver (`ExitPlanMode`) et une question de l’agent ne vont jamais aux plugins : ce sont les tiens.

## Ce que voit l’agent, ce que tu vois

La carte d’une demande tranchée par un plugin dit qui a répondu : « **Autorisé par Règles de l’équipe** via Escouade », ou « **Refusé par Règles de l’équipe** via Escouade — pas de rm -rf ici ».

L’agent apprend qui a refusé, jamais que c’est toi : il reçoit « Refusé par Règles de l’équipe via Escouade : pas de rm -rf ici », et peut chercher une autre voie.

## La demande

Escouade appelle `permissions/decide` (voir [Le protocole](plugins/api)) :

```json
{
  "request": {
    "id": "req_perm",
    "kind": "command",
    "tool": "Bash",
    "input": { "command": "rm -rf build", "description": "Supprime le dossier build" }
  },
  "project": { "id": "8c1f…", "name": "Escouade", "private": false },
  "ticket": { "id": "e02a…", "key": "ESC-180", "title": "Les règles d’autorisation" },
  "agent": { "id": "5b7d…", "name": "esc-180-les-regles" }
}
```

- `request.tool` : l’outil, nommé comme Claude Code le nomme, **quel que soit le harness** (Codex, Copilot CLI, OpenCode, Cursor CLI, Mistral Vibe, Antigravity CLI et Gemini CLI sont traduits) : `Bash`, `Edit`, `Write`, `Read`, `WebFetch`, `mcp__<serveur>__<outil>`…
- `request.kind` : sa famille, pour une règle simple : `command` (`Bash`), `edit` (`Edit`, `Write`, `MultiEdit`, `NotebookEdit`), `read` (`Read`, `Grep`, `Glob`, `LS`), `fetch` (`WebFetch`, `WebSearch`), `mcp` (un outil MCP) ou `tool` (le reste).
- `request.input` : ce que l’outil a reçu, ses textes longs coupés.
- `project`, `ticket` (absent hors ticket), `agent` : comme dans [un fait](plugins/events). Jamais la conversation.

## La réponse

```json
{ "decision": "deny", "message": "pas de rm -rf ici" }
```

| `decision` | Ce qui se passe |
| --- | --- |
| `allow` | La demande est autorisée, une fois, si aucun autre plugin ne refuse. |
| `deny` | Elle est refusée ; `message` (facultatif) est dit à l’agent et sur la carte. |
| `ask` | Le plugin laisse la main : la demande va à l’utilisateur si aucun autre ne tranche. |

Une erreur JSON-RPC, une réponse au-delà de 500 ms ou une réponse qui n’est pas l’une des trois valent `ask`. Un plugin qui plante pendant une décision est relancé comme d’habitude ; la demande, elle, va à l’utilisateur.

Lis tes règles **avant** la première demande (au démarrage), pas pendant : 500 ms, c’est court. Escouade démarre les plugins qui décident en même temps que lui ; un plugin pas encore démarré a, une seule fois, trois secondes pour démarrer et répondre.

## Un exemple

« Règles de l’équipe » refuse ce que `deny` interdit, autorise ce que `allow` permet, et laisse le reste :

```json
{
  "id": "dev.exemple.team-rules",
  "name": { "fr": "Règles de l'équipe", "en": "Team rules" },
  "version": "1.0.0",
  "escouade": { "api": "1", "min": "2.1.0" },
  "run": { "command": "team-rules", "windows": "team-rules.cmd" },
  "permissions": { "decide": true }
}
```

Avec le [SDK](plugins/sdk) :

```ts
import { readFileSync } from 'node:fs';
import { createPlugin } from '@escouade-dev/plugin';

const rules = JSON.parse(readFileSync(new URL('./rules.json', import.meta.url), 'utf8'));
const plugin = createPlugin();

plugin.decidePermission(({ request }) => {
  if (request.kind !== 'command') return { decision: 'ask' };
  const command = String(request.input?.command ?? '').trim();
  const denied = rules.deny.find((r) => command.startsWith(r));
  if (denied) return { decision: 'deny', message: `Les règles de l'équipe interdisent \`${denied}\`.` };
  if (rules.allow.some((r) => command === r || command.startsWith(r + ' '))) return { decision: 'allow' };
  return { decision: 'ask' };
});

void plugin.start();
```

Le banc le vérifie : `escouade-plugin test` joue une décision et mesure son délai, et `escouade-plugin dev` permet d’en jouer à la main (`decide {"request": {...}}`). Voir [Tester un plugin](plugins/testing).

## Le journal

Chaque décision est écrite au journal du plugin (Plugins › le plugin › Journal, filtre « Décision ») : l’outil et ce qu’il visait, la réponse, son délai, et pourquoi il n’a pas décidé (pas de réponse à temps, erreur, plantage).

```
14:03:12  Décision  refused Bash rm -rf build (2 ms): pas de rm -rf ici
14:05:40  Décision  allowed Bash npm test (1 ms)
14:07:02  Décision  did not decide Bash cargo build: no answer in time; the user is asked
```

## Reprendre la main

Dans **Plugins**, sur la page du plugin :
- retire la permission « Décide des autorisations de tes agents » : il continue sans elle et n’est plus interrogé ;
- éteins-le pour un projet : ses agents ne lui sont plus soumis ;
- un projet **privé** n’est jamais soumis à un plugin qui n’est pas allumé pour lui.
