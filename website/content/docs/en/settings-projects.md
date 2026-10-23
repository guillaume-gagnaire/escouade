# Settings: Projects

The “Projects” tab sets one project at a time: its name and color, its remote control, its worktrees, how its commits are written, and its setup, teardown, and launch commands.

## Choosing the project

The tab acts on the project chosen in the “Project” row, under the header of the [settings window](settings-application): one button per open project. What you change on a project stays in the draft when you switch to another, and everything is saved together with “Save”.

## Identity

| Setting | Effect | Default |
|---|---|---|
| “Name” | The project’s name, as its tab and the project picker show it. It can’t be left empty: “Save” is blocked until it has a name. | The name given at creation, the folder’s name if you gave none |
| “Folder” | The project’s folder, shown but not editable. The “Open” button opens it with the system’s file manager. | — |
| “Color” | One swatch out of 14 (13 hues and a neutral tone). The color identifies the project and tints the interface while it is shown. | The first color no project has taken, at creation |
| “Preferred account” | The account new agents and tickets of the project start on. Only visible with several accounts. Choices: “Automatic” (the first active account under the pause threshold, in the order of the accounts) or one of the active accounts that serve this project. | “Automatic” |

Accounts are described in [accounts](accounts) and set in the [“Accounts”](settings-accounts) tab. An account switched off after it was chosen stays shown with “(inactive)” as long as it is chosen, and is ignored in the meantime.

## Remote control

| Setting | Effect | Default |
|---|---|---|
| “Remote control for this project’s agents” | Every agent of this project, existing or new (ticket agents included), is reachable from claude.ai and the Claude app. Escouade starts each one’s `claude` process right away, at most 3 at a time, and it stays running. The session appears on claude.ai as “project · agent” (for a ticket agent, the name comes from the ticket). In Claude Code view, an interface that is already open only picks it up at its next launch. | Off |

To set an agent apart, right-click its card, then “Turn off remote control”. When off, the setting takes remote control away from the agents that had it from this setting, without touching those you turned on by hand or those the global setting still covers.

When “Remote control for all agents” is on in the [“Harness”](settings-claude-code) tab, the project’s switch shows on and grayed out, with “On for all projects in Settings › Harness”. The project’s own choice is kept: once the global setting is off, the switch goes back to its state. See [remote control](remote-control).

## Privacy

| Setting | Effect | Default |
|---|---|---|
| “Private project” | The agents of the other projects don’t see it through Escouade’s MCP server: not in the lists, nor by its name, its tickets or its agents. Its own agents see it alone. You, Claude outside Escouade and the app’s views see everything. A lock follows its name, on its tab and in the “Project” row. | Off, or “New projects are private” for a project added |

| “Accounts” (private project) | The accounts of its default harness its agents may go to: reserved to it alone, or shared with other projects (sessions, memory, history in common). “Add a reserved account…” and “Reserve Pro to this project” fix it. | — |
| “Shared accounts at the limit” (private project) | When its reserved accounts reach the usage limit, its agents may go on on a shared account (a switch of account, a handover to another harness). Off, they wait for the reset. | Off |

See [private projects](mcp#private-projects) and [an account for a private project](accounts#an-account-for-a-private-project).

### Its files

For a private project, the agents of the other projects neither read nor write its folder, its worktrees, its data in Escouade’s folder (tickets, attached files, conversations) or its harness sessions. Its own agents read none of these for the other projects, nor the rest of Escouade’s folder. Escouade gives these rules to each harness when an agent starts, and the tab says, under “Private project”, what each one guarantees:

| Harness | Protection | What is not guaranteed |
|---|---|---|
| Claude Code | Guard: its rules refuse these paths to its file tools and to the commands it recognizes (`cat`, `sed`, `Get-Content`…), even in Bypass mode. | A script that opens a file itself. With its sandbox turned on (macOS, Linux, WSL2), all its commands are held too. |
| Codex | Barrier: its sandbox refuses these paths to all its commands. | Bypass mode, which has no sandbox. |
| OpenCode | Guard: its rules refuse these paths to its tools and to the commands that name them. | A command that reaches them without naming them: it has no sandbox. |
| Copilot CLI | Guard: Escouade refuses what it asks about these paths right away, without offering it to you. | Bypass mode, where it asks nothing, and the Copilot CLI view, where its requests don’t go through Escouade. |

A guard is a rule of the harness, which a command can get around; a barrier holds for all its commands. A barrier for every harness would take a system account per project: Escouade doesn’t offer it, as the harnesses and their accounts wouldn’t follow. When you change “Private project”, an agent already running takes its new rules before its next turn: its process starts again, its conversation stays. An agent open in a terminal view takes them at its next launch.

## Git

| Setting | Effect | Default |
|---|---|---|
| “One worktree per agent” | Keeps each new agent’s changes in its own branch. | Chosen when the project is created (off until you tick it) |
| “Create the worktree at the first message, under the agent’s name” | The first message names the agent, then its branch and its folder take that name before the setup (see [at the first message](worktrees#at-the-first-message)). Off, the worktree is created at once, as agent-N. | On |
| “Starting branch of new worktrees” | The branch a new agent’s worktree starts from, which is also its base (merge, “Integrate”, pull request): “Current branch of the folder”, “Board’s target branch”, or a branch chosen in the picker, local or remote (see [the starting branch](worktrees#the-starting-branch)). “New agent from another branch…” chooses another one for a single agent. | “Current branch of the folder” |
| “Worktrees folder” | Where the project’s agent worktrees go. | Blank: `.claude/worktrees` |
| “Agents’ branch prefix” | The start of a new agent’s branch name: the prefix followed by the agent’s name. Blank, the branch takes the agent’s name alone. | `escouade/` |
| “Tickets’ branch prefix” | The start of a ticket’s branch name, the one its pull request comes from: the prefix followed by the ticket’s key. Blank, the branch takes the key alone. | `ticket/` |
| “Files copied into worktrees” | The project files copied into each new worktree, one pattern per line. | `.env*` |
| “Commit” | Who writes the commits asked for from the files panel. | “Written by the agent” |

Worktrees are explained in [worktrees](worktrees).

### Worktrees folder

You can write a path relative to the project’s folder (`../app-worktrees`, `.worktrees`), a path relative to your home folder (`~/worktrees/app`), or a full path. The quotes that “Copy as path” adds are removed. Left blank, the field means `.claude/worktrees`, as `claude --worktree` does.

Under the field, a line says where the next worktree will go (“The next one:” followed by the path, with `<name>` standing for the agent’s name). Worktrees already created stay where they are. Outside the project, the Claude Code view first asks you to trust each of them.

A folder that is refused replaces that line with a red message and blocks “Save”:

- the project’s own folder;
- a folder that holds the project’s: pick one beside it (`../app-worktrees`) or in it (`.worktrees`);
- a folder inside a `.git` folder;
- on Windows, a path that is neither relative nor full, such as `D:wt` or `\wt`.

### Branch prefixes

The branch names of agents and tickets are the ones that go to the remote repository with a pull request: the prefixes let you follow a repository’s convention. [Branches](branches) are described separately. Under each field, a line shows the resulting form.

A prefix that wouldn’t give a valid branch name is refused as you type, and blocks “Save”. These are refused: a space, `..`, `~`, `^`, `:`, `?`, `*`, `[`, `\`, and `@{`; a `-` or a `/` at the start; `//`; a part of the name that starts with `.` or ends with `.lock`.

### Files copied into worktrees

The field is a text area, one pattern per line: `.env*` copies the `.env` files at the root, `**/.env*` those in every folder. Only files git ignores are copied, and they are never committed. Empty lines are ignored.

### Commit

This choice concerns the “Commit…” and “Commit all…” buttons of the files panel, described in [Git](git).

- “Written by the agent”: the agent is asked to commit its changes.
- “Direct, with a suggested message”: Escouade suggests a message, you review it, and you commit yourself.

## Worktrees

This group sets the commands Escouade runs when a project worktree opens, and before it is removed. No command is defined at first.

| Setting | Effect |
|---|---|
| “When a worktree opens” | The setup. The commands run in order, before the agent’s first message: dependencies, generated code… Messages wait for them to finish. |
| “Before a worktree is removed” | The teardown. What the setup created outside the worktree (database, containers…); often nothing. |
| “Leftover worktrees” | “Clean up” deletes the worktrees of archived agents, after their teardown commands, except those holding uncommitted changes or commits neither merged nor pushed, and says what it did. See [worktrees](worktrees#leftover-worktrees). |

Each command is a numbered line.

| Field | Effect |
|---|---|
| “Shell” | The shell that runs it, among those the app detects. A new command takes the first one found. A shell the machine doesn’t have shows with “(not found)”. |
| “Command” | The command line, such as `npm ci`. |
| “Subfolder” | The worktree folder it runs in. Blank: its root. |
| ↑ and ↓ | Move the command up or down. `Alt+↑` and `Alt+↓` (`⌥↑` and `⌥↓` on macOS) do the same from any field of the line. |
| × | Deletes the command. |

“+ Add a command” adds a line to the list. A command whose command line is empty is not saved.

These variables are available: `ESCOUADE_PROJECT_DIR` (the project), `ESCOUADE_WORKTREE_DIR`, `ESCOUADE_BRANCH`, and the ports reserved for a ticket (`ESCOUADE_PORT_BASE`, `ESCOUADE_PORT_END`). A failure is reported in the agent’s conversation.

### Fill in automatically

The “✦ Fill in automatically” button (“Claude is reading the project…” while it works) asks Claude to suggest the commands. It reads the project (manifests, lockfiles, README…) with read-only tools: it changes nothing and runs nothing. Its commands are written for the first shell detected.

The suggestion is shown in full, with each command’s shell and folder, under “Suggested worktree commands”. Read each command in full: once saved, they run on their own, the setup when each new worktree opens and the teardown before it is removed.

- “Replace the commands” puts the suggestion in place of the draft’s two lists.
- “Ignore” throws it away.

Nothing is saved before “Save”. A message says how many commands are suggested, and how many Claude left out (invisible characters or a line that is too long). When it finds none, it says “Claude found no command to run for this project.”.

## Launch

Launch commands start the project’s servers, watchers, and services. Each one runs in its own terminal, read-only, started from the Launch section of the sidebar. That section’s gear (“Launch commands…”) opens this tab on this group. When the section is empty, its “Configure” and “✦ Suggest commands” links open it too, the second one also starting the fill-in. See [launch](launch) and [terminals](terminals).

With no command, the group says “No commands yet.”. Each command has four fields.

| Field | Effect |
|---|---|
| “Name” | The name shown in the Launch section, such as “Front”. Required. |
| “Shell” | The shell that runs it. A new command takes the first shell found. |
| “Command” | The command line, such as `npm run dev`. Required. |
| “Subfolder” (blank = project folder) | The folder, under the project’s, it runs in, such as `apps/web`. |

The cross on the right deletes the command, and “+ Add a command” adds one. A command with no name or no command line blocks “Save”.

“✦ Fill in automatically” works as it does for worktrees. Claude reads the project (manifests, scripts, docker-compose, README…) and suggests the commands to run under “Suggested launch commands”, which “Replace the commands” puts in place of the list.

## Setup assistant

“Open the assistant…” replaces the settings with the assistant of the “New project” window, on this project (“Set up <project>”), to review its settings step by step: “Keep the current setup” (the default), “Copy a project’s setup”, “Recommended setup” or “Start from scratch”, then the same steps and checks as when creating a project (see [Add a project](quick-start#add-a-project)). “Save”, in the summary, saves what the assistant set; the rest of the settings’ draft, if there was one, is not saved. Its linked sources and tickets stay its own.

## Danger zone

| Setting | Effect |
|---|---|
| “Close the project” | The “Close the project…” button asks for confirmation, then removes the project from the app: its agents, terminals, and launches stop. Its history (conversations, tickets, settings) is kept: adding a project in the same folder brings it back (see [Closed projects](#closed-projects)). The files and worktrees on disk are not touched. |

The confirmation window has a box “Also erase its history (conversations, tickets, settings)”, unticked by default. Ticked, nothing is kept: the project’s conversations, tickets, and settings are erased for good, and the same folder, added again, starts from scratch.

Closing the project acts right away, without waiting for “Save”, and takes you back to the settings with the draft as it was.

Closing is refused while an approval of one of the project’s tickets is running (“The ticket DEM-3 is being approved: wait for the end of its approval to close the project.”). Once it has finished, close the project.

## New projects

This group applies to every project: it is there even with no project open.

| Setting | Effect | Default |
|---|---|---|
| “New projects are private” | Each project added is private from the start. Existing ones keep their setting, and a project reopened with its history keeps its own. | Off |

## Closed projects

Until a project has been closed with its history kept, this group doesn’t exist. After that, “Closed projects” lists those projects, from the most recently closed to the oldest, under the note “Their history is kept: adding a project in their folder brings it back.”. Each line gives the project’s color, its name, its folder, the space its history takes on disk (its conversations, its tickets’ images and files, and the files attached to its agents included), and its last activity (“last activity on …”), with two buttons: “Reopen” and “Erase history…”.

“Reopen” opens the “New project” window already on that project’s folder, read right away, with the history found. The history of that line is the one that comes back, even when several closed projects were in the same folder (without “Reopen”, it is the most recently closed one). If you change the path in the window, that history is dropped: the new folder has its own, or none. “Cancel” closes the window without reopening or erasing anything.

A file the app can’t read (damaged, or renamed by hand) still appears in the list, under “Unreadable history”, with its file name (`….json`) and its size. It has no “Reopen”: only “Erase” is offered. After confirmation (“This file, which the app cannot read, is erased for good. The other histories, and the files and worktrees on disk, are not touched.”), it deletes that file, and only that file.

Without going through the list, simply add the project again (the `+` button to the right of the tabs), in the same folder. It doesn’t matter how you write the path: upper or lower case (on Windows), `/` or `\`, a trailing separator, a short name, a WSL share… all are recognized as the same folder. The “New project” window then says what it found, for example “History found: 12 agents, 34 tickets, last activity on Oct 14, 2026”, fills in the closed project’s name, color, and “One git worktree per agent”, and its button becomes “Reopen project”.

The project comes back as it was:

- its settings, its Kanban, and its commands;
- its agents, with their conversations. No process runs yet: they pick up again at your next message, as after a restart of Escouade. An agent whose worktree has disappeared in the meantime is treated the same way;
- its tickets. A ticket that was “In progress” when you closed the project comes back blocked, with the reason “Project closed during the ticket”: “Resume” restarts it;
- what was waiting in the sync queue with Jira, Trello, GitHub Issues, or Linear: the queue of its tickets is kept while it is closed and goes out again when it is reopened (it is forgotten if the history is erased);
- its [statistics](stats), which get its name back.

The accounts that were reserved to it (“Reserved to projects”) didn’t move while it was closed: see [the Claude accounts](accounts#reserved-to-projects). The project’s “Preferred account” goes back to “Automatic” if its account no longer exists.

Reopening is not creating a project: the window doesn’t offer a first agent, since the project’s own come back.

The “Start from a blank project” switch (“The history stays in Settings › Projects, where you can erase it.”) creates a new project in that folder instead, without the earlier agents and tickets. The old history stays under “Closed projects”: nothing is erased unless you ask.

“Erase history…” asks for confirmation (“Its conversations, tickets, and settings are erased for good. The files and worktrees on disk are not touched.”), then deletes the kept conversations, tickets, and settings of that project. The same folder, added afterwards, has no history to find.

> [!NOTE]
> A project closed before this version has already lost its history: there is nothing to get back for it.

## The project’s other settings

A project’s links to Jira, Trello, GitHub Issues, and Linear are set in the [“Integrations”](settings-integrations) tab. Its Kanban (autopilot, agents, approval) is set in the [“Kanban”](settings-board) tab.
