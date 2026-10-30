# Permission rules

A plugin can decide the permissions your agents ask for before they reach you: allow a safe command, deny a dangerous one, and leave you the rest. The requests that interrupt you most become rules, without switching to Bypass mode.

It is a plugin’s strongest permission: it is shown **in red** at install (“Decides your agents’ permissions”), each of its decisions goes to its journal, and no project can give it: an `.escouade.yml` never turns a plugin on.

## How Escouade decides

When an agent asks for a permission (a command, a file change, a web page, an MCP tool), whatever its harness:

1. Escouade asks every plugin that has `permissions.decide` and is on for the agent’s project, **all at once**.
2. Each has **500 ms** to answer: `allow`, `deny` with a message, or `ask`.
3. **A denial wins**, whatever the order of the answers. Otherwise one allowance is enough. Otherwise (they all answered `ask`, kept silent, crashed or answered something else) the request goes to the user as it would without plugins.

Meanwhile the request’s card is already in the conversation, but Escouade does not notify you: you are called only if no plugin settled it. You can still answer on the card yourself; whichever answer comes first is kept.

A plan to approve (`ExitPlanMode`) and a question of the agent never go to the plugins: they are yours.

## What the agent sees, what you see

The card of a request a plugin settled says who answered: “**Allowed by Team rules** via Escouade”, or “**Denied by Team rules** via Escouade — no rm -rf here”.

The agent learns who refused, never that you did: it is told “Refused by Team rules through Escouade: no rm -rf here”, and can look for another way.

## The request

Escouade calls `permissions/decide` (see [The protocol](plugins/api)):

```json
{
  "request": {
    "id": "req_perm",
    "kind": "command",
    "tool": "Bash",
    "input": { "command": "rm -rf build", "description": "Removes the build folder" }
  },
  "project": { "id": "8c1f…", "name": "Escouade", "private": false },
  "ticket": { "id": "e02a…", "key": "ESC-180", "title": "Permission rules" },
  "agent": { "id": "5b7d…", "name": "esc-180-permission-rules" }
}
```

- `request.tool`: the tool, named as Claude Code names it, **whatever the harness** (Codex, Copilot CLI, OpenCode, Cursor CLI, Mistral Vibe, Antigravity CLI and Gemini CLI are translated): `Bash`, `Edit`, `Write`, `Read`, `WebFetch`, `mcp__<server>__<tool>`…
- `request.kind`: its family, for a simple rule: `command` (`Bash`), `edit` (`Edit`, `Write`, `MultiEdit`, `NotebookEdit`), `read` (`Read`, `Grep`, `Glob`, `LS`), `fetch` (`WebFetch`, `WebSearch`), `mcp` (an MCP tool) or `tool` (the rest).
- `request.input`: what the tool was given, its long texts cut.
- `project`, `ticket` (absent outside a ticket), `agent`: as in [a fact](plugins/events). Never the conversation.

## The answer

```json
{ "decision": "deny", "message": "no rm -rf here" }
```

| `decision` | What happens |
| --- | --- |
| `allow` | The request is allowed, once, unless another plugin denies it. |
| `deny` | It is denied; `message` (optional) is told to the agent and shown on the card. |
| `ask` | The plugin leaves it: the request goes to the user unless another one settles it. |

A JSON-RPC error, an answer past 500 ms or an answer that is none of the three count as `ask`. A plugin that crashes during a decision is restarted as usual; the request goes to the user.

Read your rules **before** the first request (at start), not during one: 500 ms is short. Escouade starts the plugins that decide when it starts; a plugin not started yet has, once, three seconds to start and answer.

## An example

“Team rules” denies what `deny` forbids, allows what `allow` permits, and leaves the rest:

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

With the [SDK](plugins/sdk):

```ts
import { readFileSync } from 'node:fs';
import { createPlugin } from '@escouade-dev/plugin';

const rules = JSON.parse(readFileSync(new URL('./rules.json', import.meta.url), 'utf8'));
const plugin = createPlugin();

plugin.decidePermission(({ request }) => {
  if (request.kind !== 'command') return { decision: 'ask' };
  const command = String(request.input?.command ?? '').trim();
  const denied = rules.deny.find((r) => command.startsWith(r));
  if (denied) return { decision: 'deny', message: `The team's rules forbid \`${denied}\`.` };
  if (rules.allow.some((r) => command === r || command.startsWith(r + ' '))) return { decision: 'allow' };
  return { decision: 'ask' };
});

void plugin.start();
```

The bench checks it: `escouade-plugin test` plays a decision and measures its delay, and `escouade-plugin dev` lets you play some by hand (`decide {"request": {...}}`). See [Testing a plugin](plugins/testing).

## The journal

Each decision is written in the plugin’s journal (Plugins › the plugin › Journal, filter “Decision”): the tool and what it aimed at, the answer, its delay, and why it did not decide (no answer in time, an error, a crash).

```
14:03:12  Decision  refused Bash rm -rf build (2 ms): no rm -rf here
14:05:40  Decision  allowed Bash npm test (1 ms)
14:07:02  Decision  did not decide Bash cargo build: no answer in time; the user is asked
```

## Taking it back

In **Plugins**, on the plugin’s page:
- take back the permission “Decides your agents’ permissions”: it goes on without it and is no longer asked;
- turn it off for a project: its agents are no longer submitted to it;
- a **private** project is never submitted to a plugin that is not turned on for it.
