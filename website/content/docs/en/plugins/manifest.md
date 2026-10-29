# The manifest

Every Escouade plugin describes itself in an escouade-plugin.json file at the root of its folder. This page is its reference: each field, what Escouade checks, how it installs a plugin and why it disables one.

## A complete example

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.exemple.slack",
  "name": "Slack",
  "version": "1.2.0",
  "description": { "fr": "Les tickets à tester dans un salon Slack.", "en": "Tickets to review in a Slack channel." },
  "author": { "name": "Exemple", "url": "https://exemple.dev" },
  "license": "MIT",
  "repository": "https://github.com/exemple/escouade-slack",
  "escouade": { "api": "1", "min": "2.1.0" },
  "icon": "assets/icon.png",
  "run": { "command": "bin/escouade-slack", "windows": "bin/escouade-slack.exe" },
  "activation": ["event:ticket.column"],
  "permissions": {
    "events": ["ticket.*"],
    "network": ["hooks.slack.com"],
    "secrets": ["webhook"]
  },
  "contributes": {
    "settings": [{ "key": "channel", "type": "string", "title": { "fr": "Salon", "en": "Channel" } }],
    "secrets": [{ "key": "webhook", "title": { "fr": "Adresse du webhook", "en": "Webhook URL" } }]
  }
}
```

The `$schema` line gives completion and checks in VS Code and the editors that read JSON schemas. The schema is published at `https://escouade.dev/schemas/plugin-1.json`.

## The fields

| Field | Required | What it says |
| --- | --- | --- |
| `id` | yes | The plugin’s unique id, a reversed domain name in lower case: `dev.exemple.slack`. The `dev.escouade.*` ids are kept for Escouade’s own plugins. |
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
| `permissions` | no | What it asks for: `events`, `network`, `secrets`, `commands` (lists) and `decide` (`true` or `false`). |
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

`command` is the path of the program in the plugin’s folder; `windows`, `macos` and `linux` give another one for each system that has its own; `args` adds arguments. An absolute path, or one that climbs up with `..`, is refused: a plugin only starts what it brings.

### The extension points

`contributes` gathers what the plugin adds. Escouade already checks their shape:

- `settings`: its settings, each with a key (`key`), a type (`string`, `number`, `boolean` or `select`) and a title (`title`);
- `secrets`: the secrets it keeps, each with a key and a title;
- `accounts`: a kind of account, shared by its other points;
- `ticketSources`, `forges` and `harnesses`: a source of the Kanban, a code host, a harness; each with an `id`, and the account it uses (`account`) among those of `accounts`;
- `themes`: themes, described in [Themes](plugins/themes);
- `webhooks`, `commands`, `menus`, `statusBar`, `badges`, `projectTemplates` and `escouadeYml`: described with the versions that bring them in.

One plugin can hold several points: a GitHub plugin gives its issues to the Kanban and follows its PRs on a single account.

## What Escouade checks

- **An invalid manifest loads nothing.** Escouade says why, field by field, with the field’s path: “`escouade.min` is missing.”, “`contributes.settings[0].type` is one of: string, number, boolean, select.”
- **An unknown field is ignored.** A manifest written for a newer Escouade loads all the same; the ignored fields are listed.
- The same check runs on the marketplace when an author submits a plugin: both report the same errors ([publishing on the store](plugins/publishing)).

## Installing a plugin

Escouade installs a plugin in three ways:

- **From a folder**: the folder is copied to `~/.escouade/plugins/<id>/`, without its git history, and pinned by the SHA-256 fingerprint of its files.
- **From a git repository and a tag** (`v1.2.0`): Escouade clones that tag alone, copies it to the same place, and pins it by its commit. A plugin is always pinned to a version; installing another tag replaces the previous one.
- **A folder in development**: loaded where it is, without a copy, and read again as soon as one of its files changes. This is the mode for plugin authors.

The list of installed plugins, their origin and their fingerprint are in `~/.escouade/plugins.json`. A copy whose files changed since it was installed is disabled, with the reason.

A plugin’s program starts at its first activation and speaks [the protocol](plugins/api). The Settings › Plugins tab, which installs, enables and uninstalls plugins from the interface, and the facts that start a plugin come with the rest of 2.1.
