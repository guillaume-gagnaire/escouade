# The interface

A tour of Escouade’s window, region by region: the projects rail and the title bar, the agents sidebar, the header and the conversation, the side panel, the two layouts, the status bar, the settings and what changes on macOS.

## The window at a glance

![Escouade’s window: the project tabs at the top, the list of agents on the left, the chosen agent’s conversation in the middle and the status bar at the bottom](/images/en/agents.jpg)

The window has six zones:

- **the projects rail**, on the far left: a tile per open project, the overview, the stats and the settings;
- **the title bar**, at the top: the name of the project on screen, the search and the editor;
- **the sidebar**, beside the projects rail: the agents of the project on screen, the project’s launch commands and its terminals;
- **the main area**, in the middle: the conversation of the chosen agent, or another view of the project (Kanban, editor, terminal, log of a launch command);
- **the side panel**, on the right, when it is open: the uncommitted files, the git history, the project’s workflow, and the branch’s pull request with its CI;
- **the status bar**, at the bottom: the active agents, the quotas, today’s cost and the branch.

The window opens at 1440 × 900 and never goes below 1000 × 600. Escouade remembers its size and position from one launch to the next.

Short messages (a confirmation, an error, a change of account) appear at the bottom, in the middle, above the status bar, after an icon (a green check for a success, a red shield for an error), and go away by themselves: after 3.5 seconds, 7 seconds for an error, 12 for a message that carries a button.

Closing the window does not quit Escouade and does not stop the agents: the window goes away to the notification area (the menu bar on macOS). A click on the icon, “Show” in its menu, or starting Escouade a second time brings it back. “Quit” in the icon’s menu stops the application; if files in the editor are not saved, the window comes back first and asks “Quit Escouade?” (“Quit anyway” button). The icon’s tooltip says how many agents are waiting for an answer.

## The projects rail

A narrow column on the left, from the title bar to the status bar, like Slack’s workspaces. From top to bottom: “Overview”, a tile per open project, the `+` button (“Add a project”), then, at the bottom, “Stats” and “Settings”. At the first start, with no project, it keeps `+` and “Settings” alone.

### A project tile

A tile is a square of the project’s color with two letters: the initials of the first two words of its name (“demo-api” gives “DA”), else its first two letters. The tile of the project on screen has a mark at the edge of the rail and a ring of its color. Signs come up when there is a reason for them:

| Sign | What it means |
|---|---|
| Amber pill with a number, top right | That many agents of the project are waiting for your answer. |
| Green dot, bottom right | An agent of the project is working. |
| Lock, bottom left | The project is private. See [Settings: Projects](settings-projects). |
| Breathing ring | An agent of the project asked a question, finished or failed while you were looking elsewhere: red for an error, amber for a question, green for an end, the most pressing one winning. It stops when you show the agent, the window in front. |

The tooltip, to the right of the tile, gives the project’s name and its shortcut (`Ctrl 1` to `Ctrl 9`), then the agents waiting and those at work, the agents to look at (“Needs a look: …”) and the uncommitted git changes (“Δ 7 · uncommitted changes”); “Nothing running, nothing changed” otherwise.

- A click shows the project. `Ctrl+1` to `Ctrl+9` show project n.
- Drag a tile to change the order of the projects: its place stays hollow and a line says where it lands. `Alt+↑` and `Alt+↓` move the tile that has the focus. The order is kept.
- A right click, or a middle click, opens the project’s menu; the project’s name in the title bar opens the same one:

| Entry | What it does |
|---|---|
| “Rename…” | Opens the “Rename the project” window. |
| “Color” | A row of swatches: the project’s color, which tints the interface when it is on screen. |
| “Turn on a worktree per agent” or “Turn off a worktree per agent” | Toggles the option for the project’s new agents. See [worktrees](worktrees). |
| “Project settings…” | Opens the settings, Projects tab, on that project. |
| “Open the folder” | Opens the project’s folder in the system’s file manager. |
| “Move up” and “Move down” | Move the tile by one place (`Alt+↑`, `Alt+↓`). |
| “Close the project…” | Removes the project from the app after a confirmation: its agents, terminals and launches stop. Its history (conversations, tickets, settings) is kept, and `+` reopens it; a box of the confirmation erases it too. The files and the worktrees on disk are not touched. See [Settings: Projects](settings-projects#closed-projects). |

When the projects don’t fit in the column, the list scrolls between “Overview” and `+`, its faded edges saying there are more. The tile of the project on screen comes back into view by itself, however it was shown (click, notification, shortcut, “Overview”). A project out of view with an agent waiting is signalled by an amber pill under the list (or above it): a click brings it into view.

### Its width

The rail’s right edge is dragged to widen it, from 52 to 300 px. The tiles keep their size and their signs; up to 160 px, a project that has its label (see [Settings: Projects](settings-projects)) shows it under its tile, on one line, cut with “…”, the shown project’s in its color. Past that, every tile has its name on its right, a row per project: its label if it has one, else the project’s name, on or not; “Overview”, `+`, “Stats” and “Settings” have theirs too, and the agents waiting move to the end of the row. The tooltip then opens on the rail’s right.

The width chosen holds for the whole app and stays after a restart. A double click on the edge goes back to the usual width: 52 px, or 72 px as soon as a project has its label. With the keyboard, the edge takes the focus with `Tab`; `←` and `→` move it by 16 px, `Home` and `End` go to either end. In a narrow window, the rail is drawn narrower than chosen (at most the window’s width minus 948 px: 80 px at 1028 px, 300 px from 1248 px), so that the agents, the conversation and the panel keep their room; the width chosen comes back with the room.

### The rail’s buttons

- **“Overview”** (`Ctrl+Shift+A`) shows the agents of every project on one page; the same button, or the same shortcut, closes it. See [following your agents](overview).
- **`+`** opens the “New project” assistant. When projects were closed, it opens a short menu first: “Add a project…”, the latest closed projects with “Reopen”, and “All closed projects…” (Settings › Projects).
- **“Stats”** shows the usage statistics; a second click goes back to the project. See [statistics](stats).
- **“Settings”** (`Ctrl+,`) opens the settings window.

## The title bar

Above the projects rail, the logo (on Windows and Linux). Above the sidebar, the name of the project on screen: its swatch, its name, the lock of a private project, the number of uncommitted git changes (“Δ 7”) and an arrow; a click opens the project’s menu. In the middle, “Search” (`Ctrl+K`) searches the conversations of every agent. On the right, “Editor” opens the editor on the project’s folder (“Browse and edit the project’s files”, see [the editor](editor)), then, on Windows and Linux, the window buttons (“Minimize”, “Maximize” or “Restore”, “Close”). The empty parts of the bar move the window.

## The sidebar

### Agents and Kanban

At the top, a two-button switch: “Agents” shows an agent’s conversation, “Kanban” the project’s board of tickets. When tickets are waiting for your test, a badge on “Kanban” gives their number (“N to review”). See [the Kanban](board).

### The list of agents

Under the selector, “New agent” (`Ctrl+N`) creates an agent on the project’s harness. The arrow to its right, or `Ctrl+Shift+N`, opens the “Harness” menu: each harness Escouade knows, numbered (`1` to `4` pick one while the menu is open), with the account, model and mode its new agent would take; the project’s is marked “The project’s”. A harness with no account stays in its place with “No account” and “Add an account…”; a harness not found is grayed out with where to set its executable. Under “Worktree”, “New agent without a worktree” (or “in a worktree”, depending on the project’s setting) and “New agent on a branch…”, then “Project’s default harness…”. With no agent, the list says “No agents.” with a “Create an agent” button.

The agent waiting for your answer goes first in the list. Each agent is a card, without a frame; the card of the agent on screen is raised:

- its **name** (double-click to rename it in place: `Enter` confirms, `Esc` cancels), the number of its subagents and background tasks at work, a waves icon when [remote control](remote-control) is on (teal once connected; its tooltip says where it comes from: you, the project’s setting, or the setting for all agents; in the [Claude Code view](claude-code-view), it says the state of the interface), and its **status**;
- what the agent says it is doing, when it said so through the [MCP server](mcp);
- the mark of its harness (“Claude”, “Codex”… once agents run on two harnesses), its **model**, the time it has worked (“No message” until you write to it) and a “Plan 2/7” mini bar when it follows a plan (see [plan progress](plan)), otherwise its branch if it has a worktree; for a ticket’s agent, the ticket’s key and its loop (“DEM-3 loop 1/5”, with a box per loop) or “to review”;
- **what it is doing** during a turn (“Editing src/auth/jwt.ts”, “Thinking”), or afterwards what it changed (“2 files changed”), then its **tokens** and its **cost** (preceded by “≈” while a turn is running: it is then an estimate).

The card of an agent waiting for your answer shows its request and its answers, numbered: a click on an answer, or its key `1`, `2`… anywhere outside a text field, answers without leaving the conversation on screen. Several questions come one after the other (“Question 1 of 2”). A permission shows the command or the file, what the harness says, and “Allow”, “Always allow” (Claude Code, OpenCode, which tells the pattern it will keep) or “For this session” (Codex, Copilot CLI), and “Deny”. The keys answer the card that has the focus, else the first one of the list; they wait half a second after a request arrives. The agent goes on, its card flashes green a moment and goes back to its place. What does not fit on the card (an answer to write, a multiple choice, a plan, a request too long or that Claude Code would deny by default) shows its beginning and “Reply in the conversation”, which opens the agent.

Under the card of the agent on screen, the list of its subagents and that of its background tasks (see [plan progress](plan)). Under an agent’s card, its pull requests (“#41 → main”, the state of their CI, which opens the “PR and CI” tab, and the button of the useful fix), and the agents it drives through the MCP server.

A click on a card shows the agent. `Ctrl+Tab` and `Ctrl+Shift+Tab` go to the project’s next or previous agent, skipping archived ones.

### The statuses and their colors

| Status | Dot | Text |
|---|---|---|
| “Ready” | Gray ring | Gray |
| “Running” | Green | Green |
| “Question” | Amber, circled by a breathing ring | Amber, the card in amber |
| “Awaiting” or “Wakes at 3:30 PM” | Empty green ring: the turn is over, not the work | Light; the tooltip says what goes on |
| “Done” | Gray ring | Gray |
| “Error” | Red | Red |

In place of the status, the card can say “Setting up…” (the agent’s worktree is being prepared) or “Resumes” followed by a time (the agent, stopped by the usage limit, then resumes by itself), in amber.

A card whose agent asked a question, finished or failed while it was not on screen is circled in its color, amber, green or red, until you open it. `Ctrl+J` puts the focus on the card of the next agent that is waiting for your answer or needs a look, without opening it: `1`, `2`… answer there, `Enter` opens it. With the system’s “Reduce motion”, nothing moves.

### An agent’s menu

A right click on a card opens its menu:

| Entry | What it does |
|---|---|
| “Rename” | Renames the agent. |
| “Duplicate the conversation” | Creates a copy of the agent that carries all its context into a new session, without touching the original. Grayed out during a turn (“Wait for its turn to end.”). |
| “Archive” or “Restore” | Puts the agent in the “Archived (N)” list, keeping its conversation, or takes it out again. If its ticket is in progress or to review, a confirmation warns that it goes back to “To do”. |
| “Prepare launch” | For an agent in a worktree, with no ticket under way: asks it how to launch its worktree, on its own ports. |
| “Integrate …” | For an agent in a worktree: brings the project’s base branch into its own. |
| “Open in editor” | Opens the editor on its worktree, otherwise on the project’s folder. |
| “Open a terminal” | Opens a terminal in its worktree, otherwise in the project’s folder. |
| “Claude Code view” or “Escouade view” | Switches views, like the button in its conversation header. Absent for a ticket’s agent and for an archived agent. See [the Claude Code view](claude-code-view). |
| “Turn on remote control” or “Turn off remote control” | Makes the session reachable from claude.ai and the Claude app (“claude.ai, mobile”). When the link is available, “Open on claude.ai” and “Copy the claude.ai link” appear. “Turn off” always sets the agent apart: a setting turned on afterwards doesn’t give it remote control back. In the [Claude Code view](claude-code-view), “Open on claude.ai” and “Copy the claude.ai link” only appear once the interface is connected, and the change takes effect the next time the interface starts. |
| “Delete…” | Stops the Claude process and removes the conversation from the application; the Claude Code session stays on disk. For an agent with a worktree, a checkbox offers to delete the worktree too, and its branch if it was created for the agent. |

Archived agents are under “Archived (N)”, a title you unfold with a click; their card shows the name and the cost, and in amber what a worktree left behind keeps (“worktree kept: 2 commits neither merged nor pushed”).

### Launch

The “Launch” section lists the project’s launch commands (development server, watchers…) with their state (“running” in green, “crashed (code 1)” in red). On hovering a command, “Restart” and “Stop” if it runs, otherwise “Run”, which asks “In the project” or “In the worktree of …” as soon as an agent has a worktree; “Run all” and “Stop all” act on all of them. An agent’s launches (in its worktree, or those of a test) are grouped under its name. A click on a command shows its log in the main area. The settings button opens “Launch commands…”; with no command, “Configure” opens their settings and “Suggest commands” lets Claude suggest some. See [running the project](launch).

### Terminals

The “Terminals” section lists the project’s open terminals. The `+` button (“New terminal”) offers the shells detected on the machine, then “In the worktree of…”, which lists the project’s agents that have a worktree (not the archived ones) to open a terminal in it; the entry isn’t there when no agent has one. `Ctrl+T` opens one right away. A click on a terminal shows it in the main area, “Close” on hover closes it, and a terminal whose shell stopped is marked “exited”. See [terminals](terminals).

In a window 820 pixels high or less, “Launch” and “Terminals” fold to their title to leave the room to the agents; a crashed command is still said there (“1 crashed”).

### The foot of the sidebar

The foot recalls the project’s folder, its git branch and the count of its changed files (`~2`, `+1`, `−0`; their detail in a tooltip), or “No git repository”. A click on the folder (“Browse”) opens the editor on the project’s folder.

## The header and the conversation

![An agent’s header, with its buttons and its figures, above its conversation and its message field](/images/en/chat.jpg)

The header of an agent’s conversation shows, from left to right:

| Element | What it shows or does |
|---|---|
| Name and status | The agent’s name (double-click to rename it) and its status, in a pill. |
| Harness, `project / branch` | The agent’s harness (when several are installed), the project, and the agent’s branch if it has a worktree, otherwise the branch of the project’s folder; then the number of its pull requests, and “Sandbox” for a Codex agent. |
| “Context” | The context’s tokens out of the size of the model’s window (“76.2k / 200k”), with a thin gauge under it. The figure turns amber from 80%, when the context is nearly full. |
| “Tokens”, “Cost”, “Duration” | What the agent has used and the time it has worked. |
| “Files” | The number of the agent’s uncommitted files. |
| “Open a PR”, “Test” or “Prepare launch” | A single place: open the pull request of its branch while it has commits without a PR; otherwise, for an agent in a worktree, launch its work to try it, or ask the agent how to launch it. See [review and approve](testing). |
| “Escouade” and “Claude Code” | The agent’s two views, as segments: Escouade’s conversation, or Claude Code’s own interface. Absent for a ticket’s agent. See [the Claude Code view](claude-code-view). |
| “…” | The agent’s menu, the same as the right click on its card: the editor, a [terminal](terminals) in its worktree, rename, archive… |
| Two buttons | The choice of layout (see below). |

Under the header, the conversation shows your messages, Claude’s replies in markdown, its foldable reasoning, its tool calls as compact rows you can unfold, the cards for questions and permissions, and a line or a card at the end of each turn. When the agent follows a plan, a “Plan” banner sits between the header and the messages: see [plan progress](plan). A long conversation only shows its latest 80 items: “Show the previous N messages” adds more. If new messages arrive while you read higher up, the “New messages” button takes you back down. `Ctrl+K` searches the conversations.

The message field floats at the bottom, as wide as the conversation: its “Model”, “Effort” and “Mode” chips, then attach a file, dictate and talk aloud, “Stop” during a turn and “Send”. An archived agent shows “Agent archived.” instead, with a “Restore” button. An agent in error says so (“The agent stopped.”): a new message restarts Claude on the same session. See [the conversation](chat) and [agents](managing-agents).

## The side panel

The side panel has four tabs: “Uncommitted”, followed by the number of files, “History”, “Workflow” and “PR and CI” (“MR and CI” on GitLab). At 340 px wide the panel has no room for the last one: the state of the CI on the agent’s card opens it then.

### Uncommitted

- The “This agent” and “Whole project” selector chooses the scope of the list. Under it, a line says where the files come from: the agent’s worktree (“isolated from the other agents”), its edits, or all the project’s agents.
- Each row is a file: its letter (M, A or D), its name, its folder, its added and removed lines, and for “Whole project” the agent’s name. The `</>` button of a row opens the file in the editor. A right click offers “Open in editor” then, according to the file’s letter, “Discard changes…” (M) or “Delete the file…” (A), always after a confirmation, or “Restore the file” (D).
- At the bottom: “Show diff”, “Commit…” (“Commit all…” for “Whole project”) and, for an agent in a worktree, “Merge … → …” and “Integrate …”.

The list only shows 500 files; the diff and the commit take all of them. See [Git and commits](git).

### History

The “History” tab shows the git graph of the whole repository, with the agent’s branch highlighted. A click on a commit opens its diff; a right click offers actions on branches. See [branches](branches).

### PR and CI

The “PR and CI” tab shows what the repository’s code host (GitHub, Azure DevOps) says of the open agent’s branch. A red dot on the tab says the CI of its pull request failed.

- **Its pull request** at the top: number, title, state (open, draft, merged, closed), branch → target, a conflict with the target, its age and its author; the icon on the right opens it in the browser. “Agent fixes it” and “Agent addresses the feedback” hand it the failures or the feedback, on your click only. Its other pull requests are listed at the bottom.
- **The CI**: a summary (“2 failures out of 14”) then each check, failures first, with its state (queued, running, passed, failed, canceled) and its duration. A click on a failed check opens the end of its log. “Re-run failed jobs” re-runs the failed jobs of the last commit, on your click only. While a check runs and the tab is shown, the CI is read again every 20 s; never in the background.
- **The reviews and comments**, read only: who approved or requested changes, the comments with their file and line. Escouade publishes nothing there in your name.
- **The deployments** of the branch, when the host gives them (GitHub): the environment, its state and the preview address to open.

Without a pull request on its branch, the tab offers “Open a PR…”, or says why it can’t: no branch of the agent’s own, no remote, a host Escouade doesn’t read yet (GitLab, Bitbucket, Gitea), or no account, with the link to Settings › Integrations. In a project without an agent, “Project PRs and CI” opens the list of the project’s open pull requests, each with the state of its CI; a click on an agent’s opens that agent. A refused token, the host’s API limit (until what time) or a host that can’t be reached are said in the tab, with “Try again”.

## The two layouts

You choose with the two buttons to the right of the header, “Classic layout” and “Conversation and files side by side”, or with `Ctrl+Shift+L`. Escouade keeps your choice from one launch to the next.

| Item | Classic | Side by side |
|---|---|---|
| Side panel | Optional, 330 px wide: “Files” in the header or `Ctrl+Shift+B` opens it, the × closes it. | Always displayed, on the right half of the main area. `Ctrl+Shift+B` does nothing. |
| Click on a file | The diff opens in a large window. | The diff shows under the list, in the panel. |
| “Files” in the header | A button that opens the panel. | A plain number. |

![The side-by-side layout: the conversation on the left, the uncommitted files and the diff of the chosen file on the right](/images/en/git.jpg)

When the conversation is narrow, which happens in the side-by-side layout, the header first drops “Tokens” and “Files”, then “Duration” and the text of its buttons, then its figures. The cards of the sidebar show the same figures.

## The status bar

From left to right:

| Element | What it shows |
|---|---|
| `N active` | The number of agents “Running”. |
| `N waiting Ctrl J` | The number of agents “Question”, in amber when there are some. A click puts the focus on the card of the next agent waiting or to look at (`Ctrl+J`). |
| `N done` | The number of agents “Done”. |
| `N agents · memory · CPU` | The agents’ running processes, with the tools and MCP servers they start; the tooltip details them agent by agent. |
| Update | “Update X ready · Restart” when an update is downloaded: a click opens the update window. While it downloads, the bar says “Update X…”; when an automatic restart is planned, “Restarting in N s” and the “Later” button take its place. See [updates](updates). |
| `Claude 5h 22% W 36%` | A group of gauges per harness running agents (the one of the agent on screen first): the harness’s name, then each quota window of the account its new agents go to, a thin bar (amber past 80%) and its percentage. The reset is in the tooltip. A projection (“→ 86%”) only shows past 80%, in orange, in red past 100%. A harness without a quota (OpenCode) shows its cost of the day out of its budget (“$0.42 / $5”). “extra usage”, in amber, says an account of the harness answers past its quota, billed apart. A click opens the “Accounts quota” panel: each account, grouped by harness, with all its windows, their projections and their reset, its cost of the day, OpenCode’s budget and Copilot CLI’s credits. Under 1240 pixels wide, the percentages, the projections and “extra usage” make room: the panel writes them. |
| `Today $3.48` | What your agents cost today, after “≈” when part of it is estimated. |
| `main ↑1 ↓3` | The branch of the project on screen, with the commits to push and to pull. A click opens the branch picker. “not published” or “remote deleted” replaces the arrows when the branch has no tracking. |
| Speaker | Turns the sound of the notifications on or off. |

See [quotas and cost](usage) and [branches](branches).

## The settings

`Ctrl+,` or the “Settings” button, at the bottom of the projects rail, opens the settings window: the tabs on the left, the content on the right. A modified tab is marked; “Save” saves all the tabs at once, and “Cancel” (or `Esc`) closes the window without saving. The “Accounts” tab is the exception: it saves at each change. The “Projects”, “Kanban” and “Integrations” tabs set one project at a time, chosen at the top; “Project settings…” in a project’s menu opens the window on that project.

| Tab | Page |
|---|---|
| “Application” | [Application settings](settings-application) |
| “Harness” | [Harness settings](settings-claude-code) |
| “Accounts” | [Claude accounts settings](settings-accounts) |
| “Notifications” | [Notifications settings](settings-notifications) |
| “Projects” | [Projects settings](settings-projects) |
| “Kanban” | [Kanban settings](settings-board) |
| “Integrations” | [Integrations settings](settings-integrations) |
| “Terminals” | [Terminals settings](settings-terminals) |
| “Network” | [Network settings](settings-network) |
| “About” | [“About” settings](settings-about) |

## On macOS

- **Native title bar.** macOS draws the window buttons itself (red, yellow, green) at the left of the title bar, above the projects rail, in place of the logo; Escouade doesn’t show its own. The red button puts the window away in the menu bar, as on Windows and Linux, and `⌘Q` or “Quit Escouade” stops the application.
- **`⌘` instead of `Ctrl`.** Every shortcut of this page uses `⌘` (`⌘N`, `⌘,`, `⌘K`, `⇧⌘L`…), except `Ctrl+Tab` and `Ctrl+Shift+Tab`, because `⌘Tab` switches applications. In terminals, `⌘C` and `⌘V` copy and paste, and the `Ctrl` keys go to the shell.
- **Menu bar.** Escouade has its macOS menus: its own (“About Escouade”, “Hide Escouade”, “Quit Escouade”…), “Edit”, “View”, “Window” and “Help”, in the interface language.
- **Notifications.** The Dock icon bounces when an agent needs you, where the taskbar flashes on Windows. See [notifications](notifications).
