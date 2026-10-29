# Installing a plugin

A plugin adds a theme, a harness, a Kanban source, a webhook or tools to Escouade. This page tells how to install one, switch it on project by project, take a permission back, read its journal, update it and uninstall it.

## The view Plugins

The puzzle-piece icon of the projects’ rail, under the statistics, opens the view Plugins full screen; a second click goes back to the project. Settings › Plugins leads to the same place.

Its sidebar:

- **Store**: the catalog of store.escouade.dev, which is not read in the app yet; meanwhile, plugins are installed from a folder or a git repository.
- **Installed**: every plugin, in two groups, “Added” and “Shipped with Escouade”. A line per plugin: its version, “Not verified” or “In development”, the update found, where it comes from, the projects where it is on (a pill per project, a lock for a private project) and its switch for the whole app.
- **Updates**: the newer versions found, their count in amber.
- **In development**: the folders loaded in place.
- **The categories**: Harnesses, Kanban sources, Code hosts, Themes, Notifications, Permissions, Tools, Languages, drawn from what each plugin adds.

## Installing

Two buttons at the top of the view:

- **From a folder…**: the plugin’s folder, the one of its `escouade-plugin.json`. Escouade copies it into its data (`~/.escouade/plugins/<id>`), without its git history, and pins it by the SHA-256 fingerprint of its files: a copy changed afterwards loads disabled, and says so.
- **From a git repository…**: the repository’s address and a tag (`v1.2.0`). Escouade clones it at that tag alone and pins it by its commit.

Nothing is installed at once: the plugin is read, set aside, and the install window opens.

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

When it starts, Escouade looks for a newer version of each plugin: a higher tag of its git repository, a higher version in the folder it was copied from. What it finds is **offered, never installed by itself**: “v1.3.0 available” on its line, “Update to v1.3.0” on its page, the list in Updates, and “Update all” at the top of the installed ones.

An update goes through the install window, from the installed version to the new one. **If it asks for more**, the window says so (“your consent is asked again”) and marks “new” each permission more. Your choices stay: its projects, the permissions taken back, its settings and its secrets. “Update all” opens the window of each, one after the other.

## Uninstalling

“Uninstall”, on its page, stops its process and erases its folder, its settings, its secrets and its journal. The box “Keep its settings and its secrets for a later install” keeps them: installed again, the plugin finds them. A folder in development stays where it is.

## The developer mode

In development, “Load a folder…” reads a plugin’s folder where it is, without a copy or a fingerprint, and reads it again as soon as a file changes. Its journal is read live in the Journal tab. See [the manifest](plugins/manifest) and [the protocol](plugins/api) to write one.
