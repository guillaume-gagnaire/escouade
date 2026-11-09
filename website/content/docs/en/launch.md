# Running the project

Launch commands start the processes you need to develop a project (server, API, database, worker…), each in its own read-only terminal. This page explains how to define them, run them, follow their state and let Claude suggest them.

![The sidebar’s Launch section, with three commands, one of them crashed, and the log of the development server](/images/en/launch.jpg)

## What it’s for

A launch command is a command line you often rerun while developing: the front-end server, the API, a local database, a worker. It is stored with the project. It runs in its own terminal, whose log stays from one run to the next, and you run, stop or restart it in a click, alone or with the others.

It isn’t an ordinary [terminal](terminals): the log is read-only, and Escouade follows the process to say whether it is running, has stopped or has crashed.

## Defining the commands

Commands are set per project, in Settings › Projects › Launch. The quickest way is the gear button of the sidebar’s “Launch” section (“Launch commands…”), or “Configure” when there is no command yet: they open the project’s settings on that group. Each command has four fields:

| Field | Role |
|---|---|
| “Name” | The name in the list and in the log’s header (for example “Front”). It is required. |
| “Shell” | The shell that runs the command, among those the app detects. A command set to a shell this machine doesn’t have (copied from another system, for example) runs in the first detected shell, and the list marks it “bash (not found)”. |
| “Command” | The command line (for example `npm run dev`). It is required. |
| “Subfolder” | The folder it runs in, relative to the project folder (for example `apps/web`). Blank, it is the project folder. It must exist: otherwise the command doesn’t start and the log says “The folder “apps/web” doesn’t exist in the project”. |

“+ Add a command” adds one, and the command’s cross deletes it. Nothing takes effect before “Save”, which stays greyed out as long as a command has no name or no command line (“Every launch command in “demo-api” needs a name and a command line.”). A command removed from the settings and saved stops, and its log disappears. The full list of project settings is in [the project settings](settings-projects).

Commands run in the project receive no `ESCOUADE_…` variable (in an agent’s worktree, they receive its ports: see below). To give an environment variable to a process that doesn’t read it from a `.env` file, write it in front of the command, in the syntax of its shell: `$env:PORT = '3000'; npm run dev` in PowerShell, `PORT=3000 npm run dev` in bash and zsh. When “Also apply the proxy to the built-in terminals” is on (see [the network settings](settings-network)), commands receive the proxy variables.

## Running, restarting, stopping

The sidebar’s “Launch” section lists the project’s commands. Next to its title, it shows how many are running (`2/3`). Each row gives a colored dot, the command’s name, its state and its buttons; the row’s tooltip shows the command line. Clicking the row displays the command’s log in the main area.

| State | What it means |
|---|---|
| “ready” | The command hasn’t been run yet since Escouade started. |
| “running” | The process is running. |
| “stopping…” | You asked it to stop, and it hasn’t finished yet. |
| “stopped” | You stopped it. |
| “done” | It ended by itself, with code 0. |
| “crashed (code 2)” | It ended with an error. A message shows in the window too: ““Worker” stopped with an error (code 2)”. Escouade doesn’t send a system notification for it. Without an exit code, the state is “crashed”. |

The row’s buttons:

- “Run” (▶) starts the command. The next two replace it as long as it runs.
- “Restart” stops it, then starts it again. Escouade waits up to 10 seconds for the process to stop; if it is still running, the command stays as it is.
- “Stop” stops the command and the processes it started. Until the stop is done, the row says “stopping…”.

At the top of the section, “Run all” starts all the commands at the same time. As soon as one is running, the button becomes “Stop all”.

When a command doesn’t start, a message says so (““Front” couldn’t start: …”) and the log writes the reason in red. Closing the project stops its commands without counting them as crashed.

## A command’s log

A command’s log fills the main area when you click its row. The header gives its name, its shell, its state (“running”, “since 14:02”), the command line and the folder it runs in. On the right: the magnifier, “Clear” and the action buttons, which follow the state: “Restart” and “Stop” while it runs, “Restart” once it has finished, “Run” when it has never run (the log then says “Not run yet.”).

- The log starts with the command line (`$ npm run dev`). The program then writes whatever it wants, colors included. It keeps 10,000 lines.
- The log is read-only: you scroll it (mouse wheel, `Shift+PageUp`) and search in it, but keys don’t go to the process. The process follows the window’s size. As in a terminal, an `http://` or `https://` address in the log opens in the browser with a plain click.
- It stays from one run to the next: a restart adds “— restarted at 14:32 —” after it. It disappears when the command is removed from the settings or when the project is closed.
- The magnifier, or `Ctrl+Shift+F` (`⇧⌘F` on macOS) in the log, opens a search field: `Enter` goes to the next match, `Shift+Enter` to the previous one, `Esc` closes the field. “Clear” empties the log’s screen.

Agents’ test launches (the “▶ Test” button, see [testing](testing)) appear in the same section, under the agent’s name, once launched. They are followed the same way.

## In an agent’s worktree

A command also runs in an agent’s worktree, to see its work running without merging it.

- As soon as an agent of the project has a worktree, “Run” (▶) opens a menu: “In the project”, then “In the worktree of <agent>” for each agent that has one (not the archived ones). Without a worktree, ▶ runs in the project, as before. “Run all” always runs in the project, and first says which commands already run in a worktree.
- On a right-click on the card of an agent with a worktree, “Run…” lists the project’s commands; each one runs in its worktree.
- The command runs in the same subfolder, taken in the worktree. When it isn’t there (the agent’s branch doesn’t have it, or not yet), it doesn’t start and the message says why: “The folder “web” of “Front” doesn’t exist in the worktree of “refacto-auth” (its branch doesn’t have it, or not yet).”
- It shows under the agent’s name, with its own status and its own log; its terminal is named “Front · refacto-auth”. The project’s row keeps its own.
- It receives `ESCOUADE_PORT_BASE` and `ESCOUADE_PORT_END`, the agent’s block of 10 ports (reserved then if it has none), the same as its test launches. For two instances not to fight over a port, make the command read it, for example `npm run dev -- --port $env:ESCOUADE_PORT_BASE` in PowerShell or `npm run dev -- --port ${ESCOUADE_PORT_BASE:-5173}` in bash.
- When the same command already runs elsewhere (in the project or in another worktree), a dialog says so before running: “Stop the other and run”, or “Run anyway”.
- It stops with the agent: when it’s archived, deleted, when its ticket is approved (which removes its worktree), and when the project closes.
- A new worktree doesn’t have the project’s dependencies (`node_modules`, Python environment…): that’s the job of the worktree setup steps (Settings › Projects › Git and worktrees, “Worktrees” group), which run when it’s made.

## Fill in automatically

Rather than writing the commands, you can ask Claude to suggest them.

1. In the “Launch” section of a project with no command, click “Suggest commands”: the settings open on the “Launch” group and the reading starts. You can also click “✦ Fill in automatically” in that group, in the project’s settings.
2. The button says “Claude is reading the project…”. Claude reads the manifests and scripts (`package.json`, `Makefile`, `Procfile`, `justfile`, `pyproject.toml`…), the `docker-compose` files, the README and the CONTRIBUTING, at the root and in the subfolders. It only has reading tools: it changes nothing and runs nothing. It has 4 minutes at most.
3. A message says how many commands are suggested (“3 commands suggested: review them before saving.”), and a “Suggested launch commands” box shows them, each with its name, its command line in full, its shell and its folder (“the project folder” when blank).
4. “Replace the commands” puts the suggestion in place of the draft’s list, and “Ignore” drops it. Nothing is kept until you click “Save”.

> [!WARNING]
> These commands will run in your shell, with your rights. Read each one in full before saving: “Replace the commands” replaces all the ones in the list.

What Claude is asked to suggest: the processes needed to develop the project (development servers, watchers, workers and job queues, database and local services), one per command, with a short name. No install or test commands, nor commands that end by themselves (build, lint, migration). A command that returns at once (`docker compose up -d`) can’t be followed in a terminal: it must be given in the foreground (`docker compose up db`). Eight commands at most. All of them run in the first detected shell.

### What gets left out

A suggested command is left out, and counted in the message, when it spans several lines, contains a character that shows nothing or changes the reading order (a zero-width space, a direction mark…), contains a run of 24 spaces or more, or is longer than 300 characters. The reason: the settings show a command on a single line, and what you read in the field must be what runs. The message then says, for example, “2 commands suggested, 1 left out (invisible characters or too long): review them before saving.” A longer sequence goes in a script of the project, which the command calls.

Other commands are dropped without being counted: those with no command line, those whose subfolder isn’t in the project, and those beyond eight. A name longer than 40 characters is shortened.

If Claude finds nothing to run, the message says “Claude found no command to run for this project.” If all its commands are left out, the error says “Claude proposed unreadable commands: none was kept.”
