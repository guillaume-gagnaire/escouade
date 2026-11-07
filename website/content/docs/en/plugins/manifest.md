# The manifest

Every Escouade plugin describes itself in an escouade-plugin.json file at the root of its folder. This page is its reference: each field, what Escouade checks, how it installs a plugin and why it disables one.

## A complete example

The Discord plugin, one of the [example plugins](plugins/examples), whole: a plugin without a program, which keeps a secret and sends two webhooks.

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

The `$schema` line gives completion and checks in VS Code and the editors that read JSON schemas. The schema is published at `https://escouade.dev/schemas/plugin-1.json`.

## The fields

| Field | Required | What it says |
| --- | --- | --- |
| `id` | yes | The plugin’s unique id, a reversed domain name in lower case: `dev.exemple.slack`. The `dev.escouade.*` ids are kept for Escouade’s own plugins: the store takes them from its repository alone (`plugins/<folder>/`), the app installs them signed by the store only. |
| `name` | yes | Its name: one text, or a text per language. |
| `version` | yes | Its version, in three numbers: `1.2.0`, `0.3.0-beta.1`. The same as the version’s git tag. |
| `description` | no | What it does, in a sentence: one text, or a text per language. |
| `author` | no | Who made it: `name` required, `url` and `email` optional. |
| `license` | no | Its license, as an SPDX identifier: `MIT`, `Apache-2.0`. |
| `repository` | no | The address of its repository, in `https://`. |
| `escouade` | yes | The Escouade it was written for: `api`, `min` and `max` (see below). |
| `icon` | no | Its logo: a square PNG of its folder, 256 px a side at least, 1 MB at most (see below). |
| `run` | no | Its program. Without one, the plugin is declarative: nothing runs. |
| `activation` | no | What starts it, for example `event:ticket.column`. Without an activation, its program does not run. |
| `permissions` | no | What it asks for: `events`, `network`, `secrets`, `files`, `commands` (lists) and `decide` (`true` or `false`). See [the host’s capabilities](plugins/capabilities). |
| `limits` | no | What more it asks of the host: `fetchSeconds`, `fetchBytes`, `fileBytes` ([the limits](plugins/capabilities#the-limits-limits)). |
| `contributes` | no | Its extension points: what it adds to Escouade. |
| `store` | no | What the marketplace shows of it: `categories`, `keywords`, `screenshots`, `docs`. |

### Texts in two languages

`name`, `description` and the titles of settings take one text, or an object of texts per language: `{ "fr": "Salon", "en": "Channel" }`. Escouade shows the one in the interface’s language, else the English one, else the French one. An object needs at least `en` or `fr`.

### The range of Escouade

- `api`: the major version of the plugins’ protocol the plugin speaks, in quotes: `"1"`. Escouade 2.1 serves version 1.
- `min`: the oldest version of Escouade the plugin was tried with, `2.1.0` or `2.1`.
- `max` (optional): the newest. `2.4` covers every 2.4.x.

A plugin of another API, or for an Escouade out of its range, stays installed but disabled, and Escouade says why: for example “This plugin needs Escouade 2.3.0 or newer; you have 2.1.0. Update Escouade.”

### The icon

`icon` is the path, in the plugin’s folder, of its logo: a **square PNG of 256 px a side at least** (512 advised), of **1 MB at most**. No SVG (it may carry a script), no remote image, no path that leaves the folder (absolute, or climbing up with `..`). Escouade and the marketplace read its header, and refuse the plugin saying why: “`icon` is not square (a square PNG of 256 px at least, 512 advised).”, “`icon` weighs over 1 MB.”, “`icon` is no PNG (no SVG nor JPEG: a square PNG of 256 px at least).”, “`icon` is no file of the plugin’s folder.”

The marketplace re-encodes it without its metadata at 256 and 64 px, and shows it in place of the two letters of the plugin’s tile. Without an icon, the tile keeps its letters.

### The program

A program is one of the three, never two:

- `js`: the bundle of a plugin in JS or TypeScript, in its folder. Escouade runs it inside the app, in WASM, without Node; the install window says “Runs inside Escouade, isolated”. This is what `npm create escouade-plugin` writes (see [the SDK](plugins/sdk#without-node)):

`sdk/create-escouade-plugin/templates/program/escouade-plugin.json` (excerpt)

```json
  "run": { "js": "dist/plugin.mjs" },
```

- `wasm`: the `Cargo.toml` of a plugin in Rust, never a binary. The store compiles it into a module, beside the manifest as `plugin.wasm`; the window says the same as for `js`, and the same [capabilities](plugins/capabilities) apply (see [the Rust kit](plugins/rust)):

`sdk/create-escouade-plugin/templates/rust/escouade-plugin.json` (excerpt)

```json
  "run": { "wasm": "Cargo.toml" },
```

- `command`: the path of a program in the plugin’s folder, which a process runs; `windows`, `macos` and `linux` give another one for each system; `args` adds arguments. The install window says “Runs a program on your computer, with your rights”. **The store refuses it**: a plugin of the store never has a `run.command`, each one runs in WASM there. It is still started for a plugin installed from a folder or a git repository, unverified. To run a program of the computer, a plugin in WASM asks for `permissions.commands` ([the programs](plugins/capabilities#the-programs-commands)).

An absolute path, or one that climbs up with `..`, is refused: a plugin only starts what it brings.

### The extension points

`contributes` gathers what the plugin adds. Escouade already checks their shape:

- `settings`: its settings, each with a key (`key`), a type (`string`, `number`, `boolean` or `select`) and a title (`title`);
- `secrets`: the secrets it keeps, each with a key and a title;
- `accounts`: a kind of account, shared by its other points;
- `ticketSources`, `forges` and `harnesses`: a source of the Kanban, a code host, a harness; each with an `id`, and the account it uses (`account`) among those of `accounts`;
- `launchEnvironments`: a launch environment, which handles `up`, `ls`, `down` and `destroy` of a worktree and its services; for now reserved to the plugins shipped with Escouade (isola);
- `workflowReaders`: a reader of workflows, to teach Escouade a workflow it does not read by itself. Each has an `id` and `files`, the globs (from the project’s folder, without `..`) of the files it is given; see [The workflow reader](plugins/events#the-workflow-reader);
- `themes`: themes, described in [Themes](plugins/themes);
- `webhooks` and `commands`: a webhook or a command on a fact, described in [Events](plugins/events);
- `menus`, `statusBar`, `badges`, `projectTemplates` and `escouadeYml`: described with the versions that bring them in.

One plugin can hold several points: a GitHub plugin gives its issues to the Kanban and follows its PRs on a single account.

### Kinds of account

A kind of account (`accounts`) says how an account is added, and that account serves every point of the plugin that names it (`account`): one connection, every use.

```json
"accounts": [{
  "id": "gitlab",
  "name": "GitLab",
  "defaultHost": "gitlab.com",
  "selfHosted": true,
  "connect": ["token", { "cli": "glab", "login": "glab auth login" }],
  "fields": [{ "key": "url" }, { "key": "token", "secret": true }],
  "scopes": ["api"],
  "tokenPage": "{url}/-/user_settings/personal_access_tokens"
}]
```

- `defaultHost`: its public instance; `selfHosted`: it has instances of one’s own; `probe`: the path that tells an instance whose name says nothing;
- `connect`: a typed token (`token`), or the token of the service’s CLI; a token by default;
- `fields`: what the form asks (`site`, `email`, `key`, `token`…), `secret` for what goes to the keychain; a token alone by default;
- `scopes`: the rights the token must have; `tokenPage`: the page that makes one.

A kind with the `id` of one of Escouade’s integrations (`github`, `gitlab`, `bitbucket`, `gitea`, `azure`, `jira`, `trello`, `linear`, `clickup`) is offered beside it: “Connect…” then asks which one serves the account, Escouade first. Never silently.

An account’s secret is in the keychain under `plugin:<id>:account:<account>`: the account is its instance’s host (`gitlab.com`), or the kind’s `id` for a kind without instances (`jira`). The plugin reads it with `secrets/get` of `account:<account>`, if it has `account:*` in `permissions.secrets`; never another plugin’s account. What it holds: the token, or `{ "key": …, "token": … }` for a kind with two secrets (Trello).

## What Escouade checks

- **An invalid manifest loads nothing.** Escouade says why, field by field, with the field’s path: “`escouade.min` is missing.”, “`contributes.settings[0].type` is one of: string, number, boolean, select.”
- **An unknown field is ignored.** A manifest written for a newer Escouade loads all the same; the ignored fields are listed.
- The same check runs on the marketplace when an author submits a plugin: both report the same errors ([publishing on the store](plugins/publishing)).

## Installing a plugin

Escouade installs a plugin in four ways:

- **From the store**: the version is downloaded from store.escouade.dev and checked (its fingerprint, its signature when Escouade verified it) before it is copied to the same place (see [installing a plugin](plugins/installer#from-the-store)).
- **From a folder**: the folder is copied to `~/.escouade/plugins/<id>/`, without its git history, and pinned by the SHA-256 fingerprint of its files.
- **From a git repository and a tag** (`v1.2.0`): Escouade clones that tag alone, copies it to the same place, and pins it by its commit. A plugin is always pinned to a version; installing another tag replaces the previous one.
- **A folder in development**: loaded where it is, without a copy, and read again as soon as one of its files changes. This is the mode for plugin authors.

The list of installed plugins, their origin and their fingerprint are in `~/.escouade/plugins.json`. A copy whose files changed since it was installed is disabled, with the reason.

A plugin’s program starts at its first activation and speaks [the protocol](plugins/api). The facts that start it and that it receives are in [Events](plugins/events). The Plugins view installs, enables and uninstalls plugins from the interface: see [installing a plugin](plugins/installer). The plugins shipped with Escouade need no install: they come with the app.
