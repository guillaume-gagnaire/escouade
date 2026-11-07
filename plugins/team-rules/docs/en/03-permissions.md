# Team rules’ permissions

What the plugin sees, what it decides, and what it cannot do.

## What it is

A program in TypeScript (`dist/plugin.mjs`), which Escouade compiles to WASM and runs isolated, inside Escouade: it has no process of its own, no access to your disk or to the network, and no Node to install.

## What it asks for

| Permission | What the install window says |
| --- | --- |
| `run`: `dist/plugin.mjs` | “Runs inside Escouade, isolated” |
| `decide` | “Decides your agents’ permissions”, in red |

`decide` is the strongest permission a plugin has: an allowance it gives does not reach you. It can be taken back in the **Permissions** tab of its page: it is then no longer asked. No project can give it: an `.escouade.yml` never turns a plugin on.

## The requests it gets

For each permission an agent of a project it is on in asks for, Escouade sends it:

- the request: the tool (`Bash`, `Edit`, `WebFetch`…), its family (`command`, `edit`, `read`, `fetch`, `mcp`, `tool`) and what the tool was given, its long texts cut (for a command: the line and its description);
- the project (its id, its name, whether it is private), the ticket (its id, key and title) and the agent (its id and name).

Never the agent’s conversation. A private project sends it nothing until you turn it on there. A plan to approve and a question of the agent are never sent to it.

## The decisions it makes

It judges commands only (`kind` at `command`); for all the rest it answers “yours”. For a command, it answers by its `rules.json`: allowed, denied with a message, or yours (the Use tab says how).

Escouade holds the rest:

- it has 500 ms to answer (3 seconds, once, if it had not started yet); past that, or if it crashes, the request goes to you;
- a denial, its own or another plugin’s, wins; else one allowance is enough; else it is you;
- an allowance holds for that request alone;
- each decision is written in its journal.

## What it reads and writes

It reads its `rules.json`, which Escouade gives it at start, and nothing else. It writes nothing.

## Secrets

None.

## What leaves your machine

Nothing. It has no network: its rules and its decisions stay on your machine.

## What it cannot do

- Run a command itself: it answers a request, it makes none.
- Allow anything but a command of its `allow` rules: a file edit, a web page, an MCP tool always come back to you.
- Override another plugin’s denial.
- Answer a plan or a question of the agent.
- Read your files, reach the network, keep a secret.
- Act in a project where it is off.
