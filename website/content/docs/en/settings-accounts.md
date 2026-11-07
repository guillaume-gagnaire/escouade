# Settings: Accounts

The “Accounts” tab lists the accounts agents start on, a group per harness, in the order Escouade tries them. Each change is saved right away, without going through “Save”.

## How Escouade picks an account

An account is a harness run with its own configuration folder (for Claude Code, its `CLAUDE_CONFIG_DIR`), so with its own sign-in, sessions, and quota. Each account belongs to a single harness, and the tab groups them by harness: “Claude Code accounts”, then those of the other harnesses when they have some. No account exists to begin with: only the ones you add count. A harness installed without an account has its group, with “No account” and “Add a Codex account…”. A harness’s machine account, its “Main”, is your usual install: once added, it runs as it always did, with no folder of its own, and it is removed like the others, without signing the machine out. How accounts and their quotas work is explained in [accounts](accounts) and in [usage](usage).

The note under the first group’s title sums it up: “New agents start on the first active account of their harness whose 5 hours is under the “Pause above quota” and “Switch accounts at” thresholds and whose week keeps its pace, in this order.” In detail:

- A new agent starts on an account of its harness, never on another harness’s: the first active account of the group that is “good”: no short window (the 5-hour one) has reached the pause threshold or “Switch accounts at” (the lower of the two counts), and no long window (the week, the 30 days, the month’s credits) is at its limit or would end past 100% at its current pace. A week above the threshold that keeps its pace stays good.
- If every active account has reached the threshold, it starts on the first active account, and [autopilot](autopilot) pauses.
- A project’s “Preferred account”, in the [“Projects”](settings-projects) tab, goes first as long as it is active.
- An account “Reserved to projects” only receives the agents and tickets of those projects.

## The list

Each account has its own block in its harness’s group.

| Setting | Effect | Default |
|---|---|---|
| Name (the “Name” field) | The account’s name, saved when you leave the field or press `Enter`. An empty name, or one already taken by another account, is refused: the field goes back to the old one. | “Main” for the Main account; the name given at creation for the others |
| “Active” | Switch. An inactive account receives no new agent. Each harness keeps at least one active: the switch of the last active account of a group is grayed out (“Each harness needs at least one active account.”). | On |
| ↑ and ↓ arrows | Move the account up or down in its group: the order of the group is its harness’s order of priority. | — |
| “Folder” | The account’s configuration folder, shown but not editable. | — |
| “Executable” | This account’s `claude`. Blank, it is the one in the “Harness” tab (the field says “The one in the settings”). Saved when you leave the field. | Blank |
| “Sign in…” or “Sign in again…” | Opens the account’s sign-in. | — |
| “New agents” | “Model”, “Effort” and “Mode” of this account’s new agents, among its harness’s: the models its harness lists for it, the efforts of its model (absent for a harness without efforts), its harness’s modes. “The harness’s” takes its harness’s default. A new agent takes them unless its message field picks others; a ticket, after the ticket’s and the Kanban’s. Saved at once. | “The harness’s” (on the update, a Claude Code account kept takes the former “New agents” settings of the “Harness” tab) |
| “Remove” | Takes the account off the list, this machine’s account included (the confirmation says it stays signed in on the machine). | — |
| “Reserved to projects” | One checkbox per open project, plus a ticked one, marked “(closed)”, for each closed project the account is still reserved to: the account only serves the projects ticked. | None ticked: “Every project” |

### Name and state

Under the name, a colored dot and a text say where the sign-in stands:

- “Checking…” while it is being read;
- “Signed in · ” followed by the claude.ai account’s email, or just “Signed in” when the email isn’t known;
- “Not signed in”;
- “Not signed in with a claude.ai account (API key?)” for Main, which then most likely runs with an API key;
- “Inactive” for a switched-off account.

When the sign-in exists but is out of date, a line below gives the reason.

### Folder and executable

For Main, the folder is Claude Code’s own (`~/.claude`, or the one `CLAUDE_CONFIG_DIR` points to). Every other account has its own, which Escouade creates in `~/.escouade/claude/`, one subfolder per account. An account can run a different `claude` than the one in the settings, for example a Claude Code in WSL for one and a Windows Claude Code for the other.

### Signing in

“Sign in…” (“Sign in again…” when the account is already signed in, for example to switch to another claude.ai account) opens a window with a terminal running the account’s `claude`. Sign in there, with the `/login` command if Claude Code doesn’t offer it. Escouade checks for the sign-in every 2 seconds: as soon as it is done, the window shows “Signed in as” followed by the email, closes the terminal, and reads the account’s quota. If you close the window before that, the account stays, not signed in. If Claude Code stops before the sign-in, the window says so and offers “Restart”.

### Reserved to projects

The checkboxes keep an account to those projects alone (a client’s projects, say): other projects never start on it. With nothing ticked, the account serves every project, which the row says as “Every project”. The accounts of every harness are reserved the same way, and Main can be reserved like any other.

A project closed with its history kept stays reserved to the account: it appears in the row, ticked and marked “(closed)”, for example “Client (closed)”. Untick it to remove it from the account’s list; it also leaves by itself when its history is erased. While it is ticked, an account reserved to that project alone serves no open project, and “Every project” isn’t shown. The row appears even when no project is open, as long as the account is reserved to a closed project.

If no active account of its default harness serves a project (all are reserved to others, or switched off), a line under the groups names the project and says its agents and tickets won’t start. A project that no active account of a harness serves starts nothing on that harness: an agent or a ticket that asks for it is refused, with the harness and the setting to change.

### Removing an account

“Remove” asks for confirmation. The dialog reminds you that new agents will no longer start on the account, and that its folder stays on the disk, with its sign-in and its conversations. While agents that are not archived still run on the account, removal is refused. Projects that preferred it go back to “Automatic”.

## Harness order

As soon as two harnesses have accounts, the “Harness order” list shows under the groups, a harness per line with its ↑ and ↓ arrows. It orders the groups of the tab and of the quota panel, and gives each project its fallback harnesses: after its default harness, the next ones in this order, only those with an active account that serves it. A change of the order is saved right away.

## Adding an account

“Add an account…”, under the groups (or “Add a Codex account…” in the group of a harness without one), opens the “Add a Claude Code account” window, with “Harness” once two harnesses are installed. It offers first:

- “This machine’s account”: the account of the harness’s usual folder, when the harness is signed in there (“Claude Code is signed in on this machine · ada@example.dev”) and it isn’t added yet (“Added already”); greyed out otherwise. “Add this account” adds it as it is.
- “A new account”: a folder of its own under `~/.escouade`, and its own sign-in, set up this way:

1. “Name”: the new account’s name (“Pro, Team…” as an example). Two accounts can’t have the same name.
2. “Share with…”: checkboxes, all ticked at first, for the items of this machine’s account’s folder the account takes over. The files are `settings.json` and `CLAUDE.md`, the folders `skills`, `agents`, `commands`, `plugins`, `hooks`, and `output-styles`; only the ones that folder has are offered.
3. “Sharing”: “Link (a change applies to both accounts)”, the default, or “Copy”. On Windows, folders are linked with a junction, which needs no administrator rights, and files are copied, since a file link would need them. On macOS and Linux, folders and files are linked with symbolic links; the “copied (linking a file needs administrator rights)” note the window shows next to the files only holds for Windows, even though it also appears on Linux.
4. “Create and sign in” creates the account, active and last in the list, with its folder in `~/.escouade/claude/`, then opens its sign-in.

The sign-in (`.credentials.json`) and `.claude.json` are never shared: each account has its own.

## Switch accounts at

In the “Quotas” group, at the bottom of the tab, “Switch accounts at” moves an agent to another account before its own runs out.

| Setting | Effect | Default |
|---|---|---|
| “Switch accounts at” | An agent goes on on another account of its harness at the end of its turn when a short window (5 hours) of its account reaches this threshold, or a long one (week, 30 days, month’s credits) reaches it without lasting to its reset at the current pace. New agents and tickets go to the first account in the order whose 5 hours is not full and whose week keeps its pace, even on “At the limit”. Choices: 80%, 90%, 95%, or “At the limit”. | At the limit |

On “At the limit”, an agent already running doesn’t move before the limit (new agents follow the rule of the choice), then “Resume a ticket’s agent stopped by the usage limit on another account” ([“Harness”](settings-claude-code) tab) decides. The two settings work together: the threshold acts at the end of a turn, the resume at the limit on a turn the limit stopped halfway. The details are in [the accounts](accounts#switch-accounts-before-the-limit).

## The pause threshold

The threshold is set in the [“Kanban”](settings-board) tab, not here, but it decides which accounts Escouade sets aside.

| Setting | Effect | Default |
|---|---|---|
| “Pause above quota” (for all projects) | The autopilot starts no ticket while the 5-hour window or the weekly window is at or above this threshold; a ticket launched by hand still starts. Choices: 80%, 90%, 95%, or 100%. | 100% |

An account with a short window past the threshold (or a long one at its limit) is set aside in favor of the next one of its harness, and the quota panel marks it “past the pause or switch threshold” (it counts “Switch accounts at” too); an account that only its pace puts behind is marked “pace too fast for its window”. The pause threshold itself stays a percentage, window by window. Autopilot pauses harness by harness: a ticket waits for one of its harness’s accounts to go back under the threshold, while those of another harness start. A harness without a quota never pauses it.

## Resuming on another account

When a turn stops on the usage limit, the agent’s card offers “Resume on” followed by an account, or “Resume on another account” with a menu when there are several: the agent goes on with its session on the other account. The accounts offered are the active ones of its harness, other than the agent’s own, that serve its project, are signed in, and stay under the pause threshold: the automatic switch between accounts stays within the harness.

For a ticket’s agent, the “Resume a ticket’s agent stopped by the usage limit on another account” setting in the [“Harness”](settings-claude-code) tab does it by itself. A session doesn’t move from an account that runs Claude Code on Windows to one that runs it in WSL, or the other way around.
