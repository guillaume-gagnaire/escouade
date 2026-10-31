# .escouade.yml

A `.escouade.yml` file at the root of the repository holds the project settings the whole team shares. This page is the reference of its format: every key, its values, how it is read, and what Escouade does with it.

## What it is for

The file is committed with the code. Everyone who adds the repository to Escouade gets the same way of working: one worktree per agent, the same branch prefixes, the same setup commands, the same ticket approval. Escouade reads it when the project opens, reads it again when it changes, and can write it from a project that is already set up.

## A complete example

```yaml
# yaml-language-server: $schema=https://escouade.dev/schemas/escouade.schema.json
version: 1
agents:
  engine: claude
private: false
worktrees:
  per_agent: true
  at_first_message: true
  base: target
  dir: ../app-worktrees
  copy:
    - '.env*'
    - config/local.json
  setup:
    - npm ci
    - command: cargo fetch
      cwd: src-tauri
  teardown:
    - command: docker compose down
      shell: bash
launch:
  - name: web
    command: npm run dev
    cwd: web
  - name: api
    command: cargo run
    cwd: api
git:
  agent_branch_prefix: agent/
  ticket_branch_prefix: ticket/
  commit_mode: agent
  conventional_commits: true
board:
  on_approve: merge
  target_branch: main
  merge_strategy: squash
  draft_pr: false
  test_command: npm test
  tests_before_approve: true
  on_conflict: agent
  remove_worktree: true
  model: sonnet
  effort: medium
  mode: auto
```

A minimal file only says what it wants to set:

```yaml
version: 1
worktrees:
  per_agent: true
board:
  merge_strategy: rebase
```

## The rules of the format

### The style of the keys

Keys are in English, in `snake_case` (lowercase words joined by `_`), sorted into groups by theme: `agents`, `worktrees`, `git` and `board` hold their keys indented; `version` and `launch` are at the top level. The order of groups and keys is free. The file is YAML 1.2: a `#` starts a comment, and comments are kept when Escouade writes the file again.

### The version key

`version` is the number of the file’s format, a whole number: `1` today. Escouade writes it at the top of every file it creates. It is not required: without it, the file is read as version 1, and the “.escouade.yml” group of Settings › Projects says so (“no “version” key: read as version 1”). A value that is not a whole number of at least 1 (`one`, `'1'`, `0`) is told the same way, and the file is read as version 1.

### An older or a newer version

The format can evolve: each version of Escouade reads the ones it knows. A file of an older version is migrated to the current one as it is read, one version to the next, without being written back to disk: it stays valid as it is. A file of a newer version than the one Escouade knows is read as well as possible: the known keys apply, the others are told as unknown, with the warning “made for a newer version of Escouade (file version 2, 1 known here): read as well as possible”. Escouade doesn’t write over such a file: “Export to .escouade.yml” refuses it.

### A missing key

Every key is optional, `version` included. A missing key changes nothing: the project keeps its value. The “Missing” column of the tables below gives the value of a new project, Escouade’s default. Removing a key from the file doesn’t set the project back to that value: it keeps the last one the file gave it.

### An unknown key

A key Escouade doesn’t know (a typo, or a key from a newer version) doesn’t stop the reading: it is ignored and told with its line. When it looks like a known key of the same group, or bears the name of a key of another group, Escouade proposes the right one:

- `worktrees.per_agnet`: “unknown key: worktrees.per_agnet (did you mean worktrees.per_agent?), ignored”;
- `worktrees.test_command`: “unknown key: worktrees.test_command (did you mean board.test_command?), ignored”;
- an unknown group, `bord:` for instance, is told the same way (“did you mean board?”), and all it holds is ignored.

### An invalid value

A value of the wrong type or outside the allowed values is ignored and told with its line and what is expected; the rest of the file applies: “invalid value for board.merge_strategy: “fast” (expected: merge, squash, rebase) (ignored)”. In a list of steps or commands, only the invalid item is ignored, the others stay. A group that is not a set of keys (`board: true`) is ignored whole.

A file that isn’t readable YAML (an unclosed bracket, broken indentation), or whose top level isn’t a set of keys, applies nothing: the message says why and at which line.

## The types of values

| Type | YAML form | Example |
|---|---|---|
| Boolean | `true` or `false`, without quotes (any case) | `per_agent: true` |
| Text | A single value, with or without quotes; `''` or `~` is an empty text | `target_branch: main` |
| One of a list | A text, exactly one of the allowed values | `merge_strategy: squash` |
| List of texts | A YAML list, as `- …` lines or in brackets; a single value is a list of one; `[]` or `~` is an empty list | `copy: config/local.json` |
| Relative path | A text, relative to the repository; a full path (`/…`, `\…`, `C:…`) or one from the home folder (`~…`) is refused | `dir: ../app-worktrees` |
| Branch prefix | A text that can start a git branch name: no spaces, `..`, `~`, `^`, `:`, `?`, `*`, `[`, `\` or `@{`, no `-` or `/` at the start, no `//` | `agent_branch_prefix: agent/` |

`'true'` in quotes is text, not a boolean, and `yes`, `no`, `on`, `off` are not booleans (YAML 1.2). A number is read as text where text is expected (`target_branch: 2024` is the branch `2024`). A pattern that starts with `*` is written in quotes (`'*.local'`): without them, YAML reads an alias.

## The keys

### agents

| Key | Type | Values | Missing | Example |
|---|---|---|---|---|
| `agents.engine` | One of a list | `claude`, `codex`, `copilot`, `opencode` | `claude` | `engine: codex` |

The harness of the project’s new agents and tickets that don’t choose another (“Default harness”).

### private

| Key | Type | Values | Missing | Example |
|---|---|---|---|---|
| `private` | Boolean | `true`, `false` | each machine’s setting | `private: true` |

“Private project”: the agents of the other projects don’t see it through Escouade’s MCP server and don’t read its files (see [Settings › Projects](settings-projects)). The repository is sensitive for the whole team, hence its place in the file; the accounts reserved to the project stay each machine’s own.

The file can make a project private, never public again: on a machine where the project is private, `private: false` isn’t applied, the setting becomes a local setting of this machine and a warning says so (“private: false ignored: this project is private on this machine, untick “Private project” to make it public”). To make it public, untick “Private project” yourself.

### worktrees

| Key | Type | Values | Missing | Example |
|---|---|---|---|---|
| `worktrees.per_agent` | Boolean | `true`, `false` | `false` | `per_agent: true` |
| `worktrees.at_first_message` | Boolean | `true`, `false` | `true` | `at_first_message: false` |
| `worktrees.base` | Text | `head`, `target` or a branch name | `head` | `base: origin/release` |
| `worktrees.dir` | Relative path | A folder relative to the repository | empty: `.claude/worktrees` | `dir: .worktrees` |
| `worktrees.copy` | List of texts | File patterns (`*`, `**`) | `['.env*']` | `copy: ['.env*', config/local.json]` |
| `worktrees.setup` | List of steps | See [worktree steps](#worktree-steps) | none | `setup: npm ci` |
| `worktrees.teardown` | List of steps | See [worktree steps](#worktree-steps) | none | `teardown: [docker compose down]` |

- `per_agent`: “One worktree per agent”, each new agent in a worktree of its own (see [worktrees](worktrees)).
- `at_first_message`: “Create the worktree at the first message, under the agent’s name”. A new agent’s worktree waits for its first message, which names it: its branch and its folder take that name (see [at the first message](worktrees#at-the-first-message)). `false` creates it at once, as `agent-N`.
- `base`: “Starting branch of new worktrees”, the branch a new agent’s worktree starts from, its base: `head` (the folder’s branch), `target` (the board’s target branch) or a branch’s name, local or remote (`origin/release`). A branch named `head` or `target` is written `refs/heads/target`. See [the starting branch](worktrees#the-starting-branch).
- `dir`: “Worktrees folder”. It can’t be the project’s folder, a folder holding it, or a folder in `.git`: such a path is told and ignored.
- `copy`: “Files copied into worktrees”, the files git doesn’t track copied into every new worktree.
- `setup`: “When a worktree opens”, run one after the other in every new worktree, before its agent’s first message. Commands: they wait for your approval.
- `teardown`: “Before a worktree is removed”, run in a worktree before Escouade removes it. Commands: they wait for your approval.

### launch

| Key | Type | Values | Missing | Example |
|---|---|---|---|---|
| `launch` | List of launch commands | See [launch commands](#launch-commands) | none | `- name: web` then `command: npm run dev` |

The project’s “Launch” commands (dev servers, watchers), each in its own terminal (see [launch](launch)). Commands: they wait for your approval.

### git

| Key | Type | Values | Missing | Example |
|---|---|---|---|---|
| `git.agent_branch_prefix` | Branch prefix | The start of a branch name | `escouade/` | `agent_branch_prefix: agent/` |
| `git.ticket_branch_prefix` | Branch prefix | The start of a branch name | `ticket/` | `ticket_branch_prefix: tickets/` |
| `git.commit_mode` | One of a list | `agent`, `direct` | `agent` | `commit_mode: direct` |
| `git.conventional_commits` | Boolean | `true`, `false` | `true` | `conventional_commits: false` |

- `agent_branch_prefix`: a new agent’s branch is `<prefix><name>`. Empty (`''`), it is its name alone.
- `ticket_branch_prefix`: a ticket’s branch is `<prefix><key>` (`ticket/DEM-12`).
- `commit_mode`: “Commit”, who writes the commits asked from the files panel: `agent` (“Written by the agent”) or `direct` (“Direct, with a suggested message”).
- `conventional_commits`: “Generated commit message”, messages in the Conventional Commits form.

### board

| Key | Type | Values | Missing | Example |
|---|---|---|---|---|
| `board.on_approve` | One of a list | `merge`, `pr`, `push`, `keep` | `merge` | `on_approve: pr` |
| `board.target_branch` | Text | A branch name | empty: the folder’s branch at the first ticket | `target_branch: develop` |
| `board.merge_strategy` | One of a list | `merge`, `squash`, `rebase` | `squash` | `merge_strategy: rebase` |
| `board.draft_pr` | Boolean | `true`, `false` | `false` | `draft_pr: true` |
| `board.test_command` | Text | A command | empty: none | `test_command: npm test` |
| `board.tests_before_approve` | Boolean | `true`, `false` | `false` | `tests_before_approve: true` |
| `board.on_conflict` | One of a list | `ask`, `agent`, `abort` | `ask` | `on_conflict: agent` |
| `board.remove_worktree` | Boolean | `true`, `false` | `true` | `remove_worktree: false` |
| `board.model` | Text | A model, as the harness names it | empty: the account’s | `model: opus` |
| `board.effort` | One of a list | `low`, `medium`, `high`, `xhigh`, `max` | empty: the account’s | `effort: high` |
| `board.mode` | One of a list | `auto`, `default`, `plan`, `acceptEdits` | empty: the account’s | `mode: plan` |

- `on_approve`: “When I approve a ticket in “To review””: merge its branch into the target branch (`merge`), open a pull request (`pr`), push its branch (`push`), or keep it as it is (`keep`) (see [the Kanban](board)).
- `target_branch`: “Target branch”, where tickets merge, and the base of their branch and pull request.
- `agents_from_target`: the former key of `worktrees.base`, still read when the file has no `worktrees.base` (`true` reads as `target`, `false` as `head`), with a warning. “Export to .escouade.yml” replaces it with `worktrees.base`.
- `merge_strategy`: the merge’s “Strategy”: merge commit, squash or rebase.
- `draft_pr`: “Draft PR”.
- `test_command`: “Test command”. A command: it waits for your approval.
- `tests_before_approve`: “Re-run the tests first” before a ticket is approved; without a test command, nothing runs again.
- `on_conflict`: “On conflict” at the merge: ask you (`ask`), let the agent resolve it (`agent`), or give up (`abort`).
- `remove_worktree`: “Delete the worktree of a done or deleted ticket”.
- `model`, `effort`, `mode`: the model, effort and mode of the ticket agents (Settings › Kanban, group “Agents”). An empty value (`model: ''`) leaves the account’s. `bypassPermissions` is never in the file: a repository doesn’t lift the permissions of whoever opens it. Read, it is refused (“bypassPermissions is not shared: it is set on each machine”); set in the window, it stays a local setting, which the export doesn’t write.

### workflows

| Key | Type | Values | Missing | Example |
|---|---|---|---|---|
| `workflows.enabled` | Boolean | `true`, `false` | `true` | `enabled: false` |

- `enabled`: `false` stands for “No workflow for this project” (Settings › Projects › Workflow): Escouade neither reads nor writes the project’s [workflows](workflows) and follows no agent in them, on every machine that reads the file; `.escouade/workflows/` is left as it is.

### plugins

| Key | Type | Values | Missing | Example |
|---|---|---|---|---|
| `plugins.recommended` | List of plugins’ ids | the ids of the [store](plugins/installer) (`dev.exemple.slack`) | no plugin proposed | `recommended: [dev.exemple.slack]` |
| `plugins.<id>` | Settings of the plugin `<id>` | one key by setting it declares: a text, a number, `true` or `false` | its settings of each machine | see below |

```yaml
plugins:
  recommended:
    - dev.exemple.slack
    - dev.exemple.team-rules
  dev.exemple.slack:
    channel: '#team-api'
```

The plugins the project **proposes**, like VS Code’s recommended extensions. When the project opens, Escouade says how many your machine lacks; at the head of Settings › Projects, a band names them, each with what it asks for and “See in the store”, which opens its page in the view Plugins. Nothing is installed nor switched on by itself: a cloned repository must not be able to run a program. Installing stays your gesture, plugin by plugin, after the window that shows its permissions. An installed plugin leaves the band; “Not for this project” silences it on this machine, and it comes back only for a plugin the file would propose later. An id that is none (`Slack`) is told and ignored.

Under `plugins.<id>`, the settings the project gives the plugin: those it declares (`contributes.settings`), with a value of their type, go before those of its page when it asks for them for this project (`settings/get` with the project, see [the plugins’ API](plugins/api)). A key it does not declare, or one of its secrets, is never given to it: **never a secret in this file**, they go to the system’s keychain, through its page. A private project gives nothing to a plugin not switched on in it.

These keys stay the file’s: they are no settings of the project, are not marked in the window, and “Export to .escouade.yml” keeps them as they are written.

## Worktree steps

Each item of `worktrees.setup` and `worktrees.teardown` is a step, in one of these two forms:

```yaml
setup:
  # The command alone, as text: run by the default shell, at the root of the worktree.
  - npm ci
  # Or its fields:
  - command: cargo fetch
    shell: bash
    cwd: src-tauri
```

| Field | Required | What it says |
|---|---|---|
| `command` | yes | The command line, as the shell reads it. |
| `shell` | no | The shell that runs it (see [shells](#shells)). Missing: the system’s default shell. |
| `cwd` | no | A subfolder of the worktree, relative. Missing or `.`: the root of the worktree. |

A step without `command`, or whose `cwd` is a full path, is told and ignored; another field (`env:`, `name:`) is told as an unknown key. Steps run in the file’s order. A single step can be written without a list: `setup: npm ci` is a list of one step.

## Launch commands

Each item of `launch` is a named command:

```yaml
launch:
  - name: web
    command: npm run dev -- --port 5173
    cwd: web
  - name: docs
    command: npm run docs
    shell: pwsh
```

| Field | Required | What it says |
|---|---|---|
| `name` | no | The name shown for the command and its terminal. Missing: the command is its name. |
| `command` | yes | The command line. |
| `shell` | no | The shell that runs it. Missing: the system’s default shell. |
| `cwd` | no | A subfolder of the project, relative. Missing: the root of the project. |

Unlike steps, a launch command has no short form: a text alone is told as an invalid value. A single command can be written without a list (`launch:` followed directly by `name:` and `command:`, indented): it is a list of one command.

## Shells

`shell` takes the id of a shell Escouade finds on the machine:

| System | Ids |
|---|---|
| Windows | `pwsh` (PowerShell 7), `powershell` (Windows PowerShell), `bash` (Git Bash), `wsl` |
| macOS and Linux | `zsh`, `bash`, `fish`, `sh` |

A shell the reading machine lacks (`pwsh` on a Mac, `zsh` on Windows) is no error: the command runs in that system’s default shell. For a file shared across systems, leave `shell` out and write commands every shell understands.

## The JSON schema and autocompletion

The format’s JSON schema is published at `https://escouade.dev/schemas/escouade.schema.json`. It follows every version of Escouade: it describes each key, its type and its allowed values.

For autocompletion, put at the top of the file the line the files Escouade writes already have:

```yaml
# yaml-language-server: $schema=https://escouade.dev/schemas/escouade.schema.json
```

Editors that read YAML schemas (VS Code with Red Hat’s YAML extension, JetBrains IDEs, Neovim with `yaml-language-server`) then complete the keys, show their description on hover, and flag an unknown key or a value outside the list as you type.

## What stays out of the file

The file is committed and read by the whole team, and a public repository shows it to anyone. So it only holds what is true for everyone, never what belongs to one person, one machine, or is secret:

- the project’s name and color: each person’s display choice, in their own tab bar;
- the preferred account, the accounts reserved to a private project and “Shared accounts at the limit”: everyone has their own accounts;
- remote control: a personal choice, which makes one’s agents reachable from one’s own claude.ai;
- “In parallel” and autopilot: they spend your quota; someone who only reviews doesn’t want tickets to start by themselves on their machine;
- the `bypassPermissions` mode of ticket agents: a repository doesn’t lift the permissions of whoever opens it;
- the tickets’ key: each machine has its own Kanban and numbers its own tickets, and the key is fixed by the first ticket;
- the integrations (Jira, Trello, GitHub, Linear, ClickUp, GitLab, Azure Boards): they belong to your connected accounts;
- full paths: they only hold on one machine (hence a full `dir` or `cwd` is refused);
- tokens and passwords: no key takes one, and “Export to .escouade.yml” never writes one.

## When the project opens

When the folder you add has a `.escouade.yml`, the “New project” assistant offers “Settings from .escouade.yml” as a starting point, above “Recommended setup”, and picks it by itself. Once the project is made (or reopened), Escouade applies the file and says so: “settings read from .escouade.yml”.

The file is read again as soon as it changes: after a `git pull`, a branch switch or an edit by hand. The settings that changed apply, and a message says so.

## Commands wait for your approval

A cloned repository can come from anyone. The file’s commands (`worktrees.setup`, `worktrees.teardown`, `launch` and `board.test_command`) never run before you approve them, and reading the file runs none. Until you approve them, the project keeps its own, and the Kanban shows “The commands of .escouade.yml wait for your approval: they don’t run yet.”.

“Read them” opens Settings › Projects, group “.escouade.yml”, which shows each command in full, with its shell and folder. “Trust these commands” applies them. If the file changes them later (a command, its shell or folder, a command added or removed), Escouade asks for your approval again; until then, the ones you approved stay in place. Commands the same as those the project already has need no approval.

> [!WARNING]
> Read each command before you trust it: the setup runs in every new worktree, before its agent’s first message.

## Marked settings

In Settings › Projects and Settings › Kanban, a setting the file defines says so under its label: “Defined by .escouade.yml”. Changing it and saving makes it a local setting of this machine, which goes before the file, even when the file is read again: “Local setting of this machine: it goes before .escouade.yml.”. Under the setting, “Write into .escouade.yml” puts your value into the file (to commit), and “Use the file’s value” hands it back to the file.

## Export a project’s settings

To share a project that is already set up, Settings › Projects › “Export to .escouade.yml” writes the file at the root of the repository from the project’s saved settings: every key, in the order of the complete example (a target branch still empty is left out, and so is the `bypassPermissions` mode). An existing file keeps its comments, the order of its keys and the keys Escouade doesn’t know; a new file starts with the schema line and `version: 1`. The exported commands are the project’s own: they need no approval. Then commit the file.
