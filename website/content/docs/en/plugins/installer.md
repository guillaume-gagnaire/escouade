# Installing a plugin

A plugin adds a theme, a harness, a Kanban source, a webhook or tools to Escouade. This page tells how to install one, switch it on project by project, take a permission back, read its journal, update it and uninstall it.

## The view Plugins

The puzzle-piece icon of the projects’ rail, under the statistics, opens the view Plugins full screen; a second click goes back to the project. Settings › Plugins leads to the same place.

Its sidebar:

- **Store**: the catalog of store.escouade.dev, kept on your machine and searched there (see below).
- **Installed**: every plugin, in two groups, “Added” and “Shipped with Escouade”. A line per plugin: its version, “Not verified” or “In development”, the update found, where it comes from, the projects where it is on (a pill per project, a lock for a private project) and its switch for the whole app.
- **Updates**: the newer versions found, their count in amber.
- **In development**: the folders loaded in place.
- **The categories**: Harnesses, Kanban sources, Code hosts, Themes, Notifications, Permissions, Tools, Languages, drawn from what each plugin adds.

## Installing

Two buttons at the top of the view:

- **From a folder…**: the plugin’s folder, the one of its `escouade-plugin.json`. Escouade copies it into its data (`~/.escouade/plugins/<id>`), without its git history, and pins it by the SHA-256 fingerprint of its files: a copy changed afterwards loads disabled, and says so.
- **From a git repository…**: the repository’s address and a tag (`v1.2.0`). Escouade clones it at that tag alone and pins it by its commit.

Nothing is installed at once: the plugin is read, set aside, and the install window opens.

## From the store

**Store**, in the sidebar, shows the catalog of [store.escouade.dev](https://store.escouade.dev). Escouade reads it at the start then every 6 hours and keeps it on your machine: the header says when (“catalog read at 10:42”) and, when the store does not answer, shows the catalog kept and says so.

- **The search** runs on your machine, nothing sent: the name, the description, the keywords, the author and the id, case and accents ignored. **Verified only** keeps the plugins with the “Verified by Escouade” seal; the chips and the sidebar filter by category.
- **For <project>**, at the top: up to three plugins Escouade suggests from what it sees on this machine and in the open project, each with its reason (“vibe is installed on this machine”, “DEM-… keys in this repository’s commits”). Nothing of it is sent. “Not now” puts a suggestion away for a week, “Don’t suggest again” for good.
- **Each line** says the seal of the version offered or “Not verified”, the author, the installs and what the plugin asks for in one line (“Runs a program”, the hosts, “A secret”, or “No program”).
- **A store plugin’s page**: its description, its permissions, its versions (each verified or not, taken back by its author), the review’s summary, where it comes from, and “Report on the store”.

Escouade offers **the last version it can run**: a version written for a newer Escouade is skipped, and so is one its author took back.

**Install** downloads the version, then checks everything before the install window opens: `version.json` against the catalog’s fingerprint, the signature against the public keys Escouade embeds (the store cannot make a seal: it does not have the key), the archive and your system’s program against `version.json`, the manifest against its fingerprint. A gap stops everything and says so (“The archive received is not the one the store published”): nothing is installed. The catalog, the archives and the revocation list go through **Settings › Network** (proxy, the system’s certificates).

The install window then says “Verified by Escouade” and “Signature: checked here, by the app”; for an unverified version, a band: “Nobody has reviewed this version. It comes from the store, its archive is the one its author published, but nothing says what its program does”, and the button “Install anyway”.

**Install verified versions only**, at the top of the installed ones (off by default), refuses any version of the store without a seal, at an install as at an update.

A plugin installed from a folder or a git repository whose id is on the store says so on its page, with “Move to the store’s v…”: the same window, your choices kept.

> [!NOTE]
> If you accepted the usage statistics, an install from the store is counted on the store (`POST /api/v1/installs`: the id, the version, “install” or “update”, nothing else) and in Escouade’s statistics (“plugin installed from the store”, without its name). Without your consent, nothing is sent.

## The install window

It tells, before anything is installed:

- **who made it**: its name, its author, its version, its license and its id;
- **where it comes from**: the folder, or the repository and the tag; its version and its commit; its fingerprint; its signature, “None” outside the store. A plugin from outside the store is **not verified**: nobody has reviewed it, and the window says so first;
- **what it adds**: its themes, its harnesses, its webhooks, its settings…, and where to find them;
- **what it will be able to do**, permission by permission, the strongest first:

| Permission | What the window says |
| --- | --- |
| `run` | “Runs a program on your computer, with your rights”, in bold |
| `permissions.decide` | “Decides your agents’ permissions”, in red |
| `permissions.commands` | “Runs commands”, and which |
| `permissions.events` | “Gets these events”, and which |
| `permissions.network` | “Connects to”, and the host: declared, Escouade does not filter a program’s network yet |
| `permissions.secrets` | “Keeps a secret:”, and its name |

“Install anyway” installs the plugin; “Cancel” erases what was set aside. A manifest that does not hold opens no window: the reason shows at the top of the view, field by field (see [the manifest](plugins/manifest)).

## Switching on for the app or by project

- **For the whole app**: the switch of its line, or “Switch off” on its page. Switched off, nothing of it runs and no project gets it; its process is stopped.
- **By project**: the Projects tab of its page, a line per open project. An installed plugin is on in every project, **but the private projects**: a private project is off by default, and the line says so (“nothing reaches it”). Switching it on there is a choice made one by one; the line then reminds that Escouade cannot keep a program from reading the disk.

> [!NOTE]
> A project that recommends a plugin in its `.escouade.yml` never installs or switches it on by itself: it is always the install window, and your consent.

## Taking a permission back

The Permissions tab of its page lists what it can do, with a switch per permission. Taking one back stops its process; the next one starts without it, and the plugin goes on, or stops if it cannot. A secret taken back is no longer given to it: `secrets/get` answers it an error for that key. The program itself is not taken back: to stop it, switch the plugin off.

## Its settings and secrets

The Settings tab shows the fields it declares (`contributes.settings`, `contributes.secrets`). A setting is kept with the list of the installed plugins and given to it by `settings/get`. A secret goes to the system’s keychain, under a name of its own (`plugin:<id>:<key>`): the tab only says whether it is kept, and “Forget” erases it.

## The journal

The Journal tab shows where its process stands (ready, running, restarted after a crash, stopped after three crashes, stopped for its memory…) and the last 500 lines of its journal: its starts, its stops and their reason, its crashes, its standard error, its messages, the calls left unanswered and what it was refused. It is read again every two seconds, filters by kind and copies whole. The secrets it was given are masked there.

- **Start** starts a plugin that has not run yet (it starts by itself at its first activation).
- **Restart** starts a stopped plugin again, its crashes forgotten.

The journal’s file is `~/.escouade/plugin-logs/<id>.log` ([the protocol](plugins/api#the-journal)).

## Updates

When it starts, Escouade looks for a newer version of each plugin: a higher tag of its git repository, a higher version in the folder it was copied from, the last compatible version of the catalog for a plugin of the store (“v1.3.0 · not verified” when it has no seal). What it finds is **offered, never installed by itself**: “v1.3.0 available” on its line, “Update to v1.3.0” on its page, the list in Updates, and “Update all” at the top of the installed ones.

An update goes through the install window, from the installed version to the new one. **If it asks for more**, the window says so (“your consent is asked again”) and marks “new” each permission more. Your choices stay: its projects, the permissions taken back, its settings and its secrets. “Update all” opens the window of each, one after the other.

## Withdrawn by Escouade

Escouade reads the store’s **revocation list** at the start then every 6 hours (or “Read again now”, at the bottom of the installed ones), checks it with its keys and refuses a list older than the last one read; when the store does not answer, it reads its copy on escouade.dev. The last list read applies offline too.

- A **withdrawn** version is stopped and switched off; a red band at the top of the installed ones says which, when and why. Escouade deletes nothing: its settings and secrets stay, and “Uninstall” is on its page.
- A version **whose seal is withdrawn** keeps running; its page says why it is no longer verified.

The seal of an installed plugin is checked again on your machine, with the `version.json` and the signature kept at the install: it holds without the store.

## Uninstalling

“Uninstall”, on its page, stops its process and erases its folder, its settings, its secrets and its journal. The box “Keep its settings and its secrets for a later install” keeps them: installed again, the plugin finds them. A folder in development stays where it is.

## The developer mode

In development, “Load a folder…” reads a plugin’s folder where it is, without a copy or a fingerprint, and reads it again as soon as a file changes. Its journal is read live in the Journal tab. See [the manifest](plugins/manifest) and [the protocol](plugins/api) to write one.
