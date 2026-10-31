# Team rules · Règles de l'équipe

`dev.escouade.team-rules`, a plugin of [Escouade](https://escouade.dev)'s own hand.

Allows or denies the agents' commands by the team's rules, written in `rules.json`. A program in TypeScript on
`@escouade-dev/plugin`, asked by Escouade before an agent runs a command (`permissions/decide`).

Autorise ou refuse les commandes des agents selon les règles de l'équipe, écrites dans `rules.json`.

## Use it

Install it from the Plugins view of Escouade (or from <https://store.escouade.dev/plugins/dev.escouade.team-rules>),
or copy this folder under an id of your own and write your team's `rules.json`:

```json
{
  "deny": ["rm -rf", { "command": "git push --force", "message": "No force push in this team." }],
  "allow": ["npm test", "git status"]
}
```

Each command of a line is judged: one denied denies the line, with its message; every one allowed allows it. A line
with a substitution (`$(…)`, backquotes, `$VAR`), a redirection or an escape is never allowed: it goes to you, as does
anything the rules do not name.

## What it may do

- decide the agents' permission requests (`permissions.decide`);
- run its program, `dist/plugin.mjs` (Node 22.18 or later), as this folder holds it. No network, no secret.

## What is in this folder

| File                   | What it is                                                     |
| ---------------------- | -------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest                                                   |
| `rules.json`           | The rules                                                      |
| `src/`                 | The program: `plugin.ts` (its rules), `main.ts` (its start)    |
| `dist/plugin.mjs`      | Its build, committed: Escouade runs it as the repository holds |
| `test/`                | Its tests, against the SDK's fake Escouade                     |
| `CHANGELOG.md`         | Its versions                                                   |

## Check it

```sh
npm install
npm run typecheck && npm test        # its tests
npm run build                        # dist/plugin.mjs again, then commit it
npm run validate && npm run bench    # the manifest, then Escouade's conformance bench
```

Its scripts pass `--first-party`: its id `dev.escouade.*` is kept for Escouade's own plugins. The store takes it from
this folder of Escouade's repository alone; its tags are `plugins/team-rules/v1.2.3`.

MIT license (`LICENSE`).
