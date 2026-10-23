# Several accounts

Escouade can run your agents on several accounts of each harness (personal, work, a client’s) and picks each new agent’s account by itself. This page explains how to add them, order them, reserve them to projects and resume an agent stopped by the usage limit on another account.

## Why several accounts

A Claude account is a sign-in to claude.ai with its own quotas (a 5-hour window and a weekly window) and its own conversations. With several accounts you keep personal and work apart, or you set aside an account for a client’s projects. When an account reaches the pause threshold, new agents start on another one; with “Switch accounts at”, agents already running do too, at the end of their turn.

Each account is Claude Code run with a configuration folder of its own (the `CLAUDE_CONFIG_DIR` variable), so it has its own sign-in, sessions and quota. Escouade never shares one account’s sign-in with another.

This page covers how to use accounts; the full list of the tab’s options is in [the accounts settings](settings-accounts).

## No account to begin with

A first installation has no account, for any harness, Claude Code included: only the accounts you add exist and count (the account an agent or a ticket goes to, switches, handovers to another harness, quotas, statistics, the MCP server, written texts). Settings › Accounts shows each harness installed, with “No account” and an “Add a Codex account…” button while it has none.

Without an account of its harness, nothing starts silently: a new agent, a ticket started, the autopilot, a handover or a written text is refused (“No Claude Code account: add one in Settings › Accounts.”), with an “Add an account…” button on that harness. The first screen and the “Account and commits” step of the “New project” assistant lead there too; from the assistant, the window opens on top of it and what you typed stays.

> [!NOTE]
> When updating to 1.8.1, the Main account of each harness stays, as an account added (same agents, tickets and statistics), if it is signed in or has served already. A Main never signed in nor used goes. Nothing else changes if you only had your Claude Code account.

## This machine’s account (Main)

“Main” is your usual account of the harness, the one of its usual folder (`~/.claude` for Claude Code), once added as “This machine’s account”. Its card marks it “This machine’s account”.

- Escouade runs it as it always did: it doesn’t give it a configuration folder. It keeps your sign-in and your conversations. Its folder is the one the `CLAUDE_CONFIG_DIR` variable names when it is set for the app, otherwise `~/.claude`. On macOS, giving it a folder would sign you out: Claude Code then changes the name of the keychain entry where it keeps the sign-in.
- It is called “Main” until you rename it, in the language of the interface.
- It is removed like any other account: removing it doesn’t sign the machine out (the confirmation says so). Like them, it can be renamed, switched off (as long as another account of the harness is active), reserved to projects and given its own executable.
- If it isn’t signed in with a claude.ai account (it runs on an API key, say), the tab says so: “Not signed in with a claude.ai account (API key?)”.

## Add an account

1. Open the settings (`Ctrl+,`, or ⚙ in the status bar) and the “Accounts” tab.
2. Click “Add an account…” (or “Add a Codex account…” in the group of a harness without one).
3. Choose the account:
   - “This machine’s account”: the account already signed in on this machine, in the harness’s usual folder, offered when the harness is signed in there (“Claude Code is signed in on this machine · ada@example.dev”) and it isn’t added yet (“Added already”). “Add this account” adds it as it is: there is nothing to sign in.
   - “A new account”: a folder of its own under `~/.escouade`, and its own sign-in. Then go on below.
4. Type the account’s name in “Name” (the field suggests “Pro, Team…”). Two accounts can’t have the same name, whatever the capitals.
5. Choose what the account takes from this machine’s account, under “Share with…” (see below).
6. Click “Create and sign in”. The account is created, active and placed last in the list, then the window opens its sign-in.

Once two harnesses are installed, the window starts with “Harness”: the account’s harness, Claude Code, Codex or Copilot CLI (see [Codex accounts](#codex-accounts) and [Copilot CLI accounts](#copilot-cli-accounts)).

Escouade creates a folder for the account under `~/.escouade/claude/`, named after the account. It never reuses the folder of a removed account: a new account always gets a fresh folder.

> [!NOTE]
> On macOS, `Ctrl` becomes `⌘` in shortcuts (except `Ctrl+Tab`).

### What an account takes from Main

A new account can take part of what you set up in Claude Code for Main. Only the items Main’s folder contains are offered, and they are all ticked at first. If Main has nothing to share, the window says so.

| Item | Type |
|---|---|
| `settings.json` | File: Claude Code’s settings |
| `CLAUDE.md` | File: your instructions |
| `skills`, `agents`, `commands` | Folders: skills, agents and commands |
| `plugins`, `hooks` | Folders: plugins and hooks |
| `output-styles` | Folder: output styles |

The “Sharing” choice says how these items are taken:

- “Link (a change applies to both accounts)” (the default): the account points to Main’s item. A change made on one side shows on the other. On Windows, folders are linked with a junction, which needs no administrator rights, but files are copied, because a link to a file needs them; the window reminds you: “copied (linking a file needs administrator rights)”.
- “Copy”: the account gets a copy, and each one then changes its own.

Two cases are exceptions. When Main’s folder is in a WSL distribution, everything is copied, whatever you choose. On Windows, a path that contains `%` or a straight quote can’t be linked: the message then asks you to choose “Copy”.

> [!NOTE]
> The sign-in (`.credentials.json`) and the `.claude.json` file (the email address, the MCP servers and the projects of the account) are never shared: each account has its own.

### Sign in to the account

After “Create and sign in”, the window changes its title (“Sign in to the account” followed by the account’s name) and shows a terminal running the account’s own `claude`, with its configuration folder.

1. Sign in from that terminal, as you would in Claude Code: follow the instructions, the browser opens. If Claude Code doesn’t offer the sign-in, type `/login`.
2. Escouade looks for the sign-in every 2 seconds. As soon as it finds it, the window says “Signed in as ada@example.dev” (or “Signed in” when the address isn’t known yet), closes the terminal a second and a half later and reads the account’s quota.
3. Click “Done”.

If Claude Code stops before the sign-in, the window says “Claude Code stopped before signing in.” and offers “Restart”. If you close the window along the way, the account stays in the list, “Not signed in”: pick it up again with “Sign in…”.

“Sign in…” also reconnects an account whose sign-in has expired. When the account is signed in and up to date, the button reads “Sign in again…”: handy to switch to another claude.ai account.

When Claude Code can’t refresh an account’s sign-in, the agent that stops goes on by itself if the sign-in on disk is good; otherwise the “Sign in again” card offers “Sign in again” and the agent goes on once you’re signed in. See [troubleshooting](troubleshooting).

## Codex accounts

A Codex account is Codex started with a folder of its own (the `CODEX_HOME` variable): its ChatGPT sign-in, its sessions and its quota. They are managed in the “Codex accounts” group of the tab, as Claude Code’s are.

- Its “Main” is your usual Codex, `~/.codex`: Escouade starts it without touching `CODEX_HOME`. “Add an account…” offers it as “This machine’s account” when Codex is signed in there.
- “Add an account…”, harness “Codex”, makes its folder under `~/.escouade/codex/`. It may take from Main `config.toml` (its settings, the Windows sandbox and the trust of your folders included), `AGENTS.md` and its `skills`, `prompts` and `rules` folders, linked or copied, as a Claude Code account does.
- The sign-in (`auth.json`) is never copied: its token changes each time it is renewed, and two folders using it would sign each other out.
- “Create and sign in” opens a terminal running `codex login` in the account’s folder: the browser opens on ChatGPT’s sign-in. As soon as `auth.json` is written, the window says “Signed in as” followed by the ChatGPT account’s address.

A Codex account’s quota comes from Codex itself: from its agents while they run, else from an app-server of the account started every 5 minutes just long enough to read it, with no turn nor conversation. Its windows are labeled by their length (“5h”, “W”, or a single “30d” window on the free plan). A turn stopped by the limit waits for the reset then goes on by itself, and new Codex agents go to the next Codex account under the threshold, as with Claude Code. Resuming a Codex agent on another account isn’t offered: it waits for its reset, or goes on on another harness.

## Copilot CLI accounts

A Copilot CLI account is Copilot started with a folder of its own (the `COPILOT_HOME` variable): its GitHub sign-in, its sessions and its AI credits. They are managed in the “Copilot CLI accounts” group of the tab.

- Its “Main” is your usual Copilot, `~/.copilot`: Escouade starts it without touching `COPILOT_HOME`.
- “Add an account…”, harness “Copilot CLI”, makes its folder under `~/.escouade/copilot/`. It may take from Main `mcp-config.json` (your MCP servers), `copilot-instructions.md` and its `agents` and `skills` folders, linked or copied.
- The sign-in is never copied: `config.json` (which says which GitHub user is signed in) and `settings.json` stay Main’s, and the token stays in the system keychain.
- “Create and sign in” opens a terminal running `copilot login` in the account’s folder: it gives a code to enter on github.com/login/device. As soon as it is signed in, the window says “Signed in as” followed by `@` and your GitHub user name. Escouade neither takes nor copies any token, not even the GitHub CLI’s.

Two accounts never mix. Not signed in itself, Copilot takes the GitHub CLI’s account (`gh`) on its own, and it prefers a token of the environment (`GH_TOKEN`, `GITHUB_TOKEN`, `COPILOT_GITHUB_TOKEN`) to its sign-in: an account other than Main never gets those tokens, and on the GitHub CLI’s account it counts as not signed in (its agent doesn’t start and the conversation says to sign it in). Main may run on the GitHub CLI’s account: the conversation says so.

Copilot counts in AI credits, by month: the “credits” bar says how much of them is used and when they come back (on the 1st of the month). They come from each call of its agents, else from a `copilot` of the account started at most every 5 minutes just long enough to read them, with no session nor model. Past “Pause above quota”, the autopilot pauses as for Claude Code’s windows. A turn stopped by the credits running out goes on by itself when they come back. If a session has a ceiling of credits (`--max-ai-credits`) and reaches it, a card offers to add as many credits, to lift the ceiling or to stop there.

## OpenCode account

OpenCode has a single account in Escouade: its “Main”, your usual OpenCode config and the providers you connected to it (“Sign in…” runs `opencode auth login`). “Add an account…” offers it as “This machine’s account” once OpenCode is installed (“OpenCode is installed on this machine”).

OpenCode has no quota: its cost is the one your providers give, 0 with a local model. The status bar shows, instead of the bars, its account’s cost for the day. To cap it, fill in “Daily budget ($)” on its block, in Settings › Accounts (empty by default): once the day’s cost reaches it, the autopilot pauses until midnight, as when a quota is used up, and the status bar shows it (“$0.42 / $5.00”). “Resume now” lifts the pause until midnight.

## Manage the accounts

The “Accounts” tab of the settings lists the accounts. Each change is saved right away, without going through the saving of the other settings (“Cancel” doesn’t undo it). One row per account:

| Item | Role |
|---|---|
| Name | An editable field. The change is taken when you leave the field or press `Enter`. |
| State | “Checking…”, “Signed in · ada@example.dev”, “Signed in”, “Not signed in” or “Inactive”. An expired sign-in is flagged in amber under the row: “Sign-in expired: run Claude Code again for this account.” |
| Active | A switch. An inactive account no longer receives new agents. The last active account can’t be switched off (“At least one account must be active.”). |
| ↑ and ↓ | The order of the accounts. New agents try them in this order (“Move Pro up”, “Move Pro down”). |
| Folder | The account’s configuration folder, read-only. |
| Executable | The `claude` this account runs. Empty (“The one in the settings”), it takes the executable of the “Harness” tab. |
| Sign in… / Sign in again… | Opens the account’s sign-in (see above). |
| New agents | The “Model”, “Effort” and “Mode” this account’s new agents start with, among its harness’s (the models its harness lists for it, the efforts of its model, its harness’s modes). “The harness’s” takes the harness’s default. |
| Remove | Removes the account (see below), this machine’s account included. |
| Reserved to projects | The projects the account is reserved to (see below). |

Under the list, an amber line appears when a project has no active account left to serve it.

## Which account an agent starts on

### The automatic choice

A new agent, or a ticket the autopilot takes, starts on:

1. the project’s “Preferred account”, if it has one, that account is active and it serves the project;
2. otherwise the first active account of the list, in order, that serves the project and whose quotas aren’t at the “Pause above quota” threshold (an account that just hit its usage limit is also left out);
3. if no account is under the threshold, the first active account. The [autopilot](autopilot) then pauses.

The order of the list therefore decides: put at the top the account you want to use up first.

The threshold is set in Settings › Kanban, “Autopilot” group: 80, 90, 95 or 100% (100% by default), for all projects. It is detailed in [the Kanban settings](settings-board).

When the account that received new agents passes the threshold and another one is available, a notification says so: “The Main account has reached 91%: new agents now go to Pro.”

### A project’s preferred account

With several accounts, a project’s settings (“Projects” tab, “Identity” group) have a “Preferred account” choice: the account this project’s new agents and tickets go to.

- “Automatic” (the default): the automatic choice above.
- A specific account: it is taken even if its quota is at the threshold. The project’s tickets then wait for that account alone.

A preferred account that has become inactive, that is reserved to other projects or that no longer exists is ignored: the project goes back to automatic. Removing an account also clears the preference of the projects that had chosen it.

### Choose the account by hand

With several accounts, an agent’s input field (see [the chat](chat)) has an “Account” menu next to the model. As long as you haven’t written to the agent, it offers:

- “Automatic (Pro)”: the account the automatic choice would take now (“The project’s, else the first account under the pause and switch thresholds.”);
- every active account that serves the project.

Choosing an account changes the agent’s account before it starts.

### Each agent stays on its account

From the first message on, an agent’s account can’t change: the menu becomes read-only (“An agent’s account can’t change after its first message (its conversation is filed in that account).”). Claude Code files each conversation in the folder of the account that runs it.

Two things move an agent, with its session: “Resume on…” and “Switch accounts at” (below). The statistics count each turn on the account the agent had at that moment.

## Reserved to projects

Every account, Main included, can be kept for certain projects. Under an account, the “Reserved to projects” row lists your projects with a checkbox each. While no box is ticked, the row ends with “· Every project”: the account serves every project. Hovering the label recalls the principle: “Tick projects to keep this account to them alone (a client’s projects, say): the others never start on it.”

A project you don’t tick can never use the account:

- not through the automatic choice, for an agent or for a ticket;
- not through the agent’s “Account” menu;
- not as “Preferred account”, which the project doesn’t offer;
- not through “Resume on…”, nor through the automatic resume.

Agents that already started on the account stay there.

It is meant to isolate a client: tick their projects on their account, and their projects only start on it.

If no active account serves a project (because every account is reserved to other projects, say), nothing starts for it:

- the project’s Kanban says “No Claude account for this project — no tickets will start”;
- a new agent is refused, with a message that says where to tick the project;
- the “Accounts” tab says it under the list, in amber, for example: No active account serves “Client”: its agents and tickets won’t start.

To fix it, tick the project on an active account, or untick all the boxes of one account so that it serves every project again.

“Close the project” (the danger zone of the project’s settings) keeps the project’s history, and the project stays in the “Reserved to projects” list of the accounts it was reserved to: it only leaves when its history is erased (the “Also erase its history” box when closing, or “Erase history…” under “Closed projects”).

An account that was reserved to that project alone therefore serves no open project while the project is closed. If it was your only active account, nothing starts for the other projects, and the “Accounts” tab says so under the list, as above.

That tab shows the closed project in the account’s “Reserved to projects” row, ticked and marked “(closed)”, for example “Client (closed)”. Untick it to free the account: with nothing ticked, it serves every project again. When you reopen the project (see [Closed projects](settings-projects#closed-projects)), its reservations are still there: there is nothing to redo. If you erase the history, when closing or later under “Closed projects”, the project leaves every account’s list. The project’s “Preferred account”, for its part, goes back to “Automatic” on reopening if its account no longer exists.

### An account for a private project

Two projects on the same account share its folder: the harness’s sessions, memory, history and global settings. A [private project](mcp#private-projects) is only fully partitioned with an account reserved to it alone: ticked for that project and no other.

As soon as you turn on “Private project” in the project’s settings, the “Accounts” row says where its agents may go. An account reserved to it alone: “Account reserved to this project: “Pro”. Claude Code’s sessions, memory and history there are shared with no other project.” An account still shared, in amber: “Its agents may go to “Main”, an account shared with other projects: Claude Code’s sessions, memory, history and global settings are common there.” To fix it:

- “Add a reserved account…” opens “Add an account”, on a new account with its own folder. The account made is reserved to the project alone, then you are back in its settings.
- “Reserve Main to this project” reserves an existing account to the project alone at once: the other projects no longer go to it.

The “Accounts” tab says it too, under an account a private project still goes to: “The private project “Client” still goes to this account, shared with other projects.”, with “Reserve to “Client””.

The agents of a private project that has an account of its own never switch to a shared account by themselves: past the threshold, its new agents and tickets stay on its account, the automatic resume of a ticket agent only goes to another account of its own, and the automatic handover to another harness only if the account it would take there is its own. Otherwise, they wait for the reset. Without an account of its own, the project keeps its shared accounts, but no automatic switch moves it to another account. To allow it, turn on “Shared accounts at the limit” in the project’s settings. “Resume on…” and “Continue on…” are still yours to click.

What a new account shares with Main when it is made (settings, instructions, skills…) stays shared, never its sessions or its memory.

## Switch accounts before the limit

The “Switch accounts at” setting (Settings › Accounts, “Quotas” group) moves an agent to another account when its own nears the limit: at 80%, 90%, or 95% of its 5-hour window or of its weekly window, whichever comes first. By default it is “At the limit”: nothing moves before it, as before this setting.

At the threshold, an agent is never stopped in the middle of its work:

- it finishes its turn on its account, then its process starts again on the other account, with its session: its next turn goes there;
- if it waits for your answer (a question, a permission) or a task runs in the background in its process (a command, a monitor, a subagent), it stays while they are there, and moves at the first end of a turn without them;
- ordinary agents and ticket agents both move; new agents start on the other account directly.

The account it goes to is the project’s preferred account, else the first in the list, as long as it serves the project, is signed in, and is itself under the threshold. A private project that hasn’t turned on “Shared accounts at the limit” only goes to an account reserved to it alone. Only Claude Code moves an agent already running: a Codex, Copilot CLI or OpenCode agent stays on its account, while its new agents follow the threshold.

The conversation says so (“The agent goes on on the Pro account: Main is at 96% of its 5-hour window.”), a notification too, and the agent’s “Account” menu shows the new account once its process has started again on it.

With no usable account, the agent stays where it is and its conversation says why, once: “Main is at 96% of its 5-hour window, but no other Claude Code account can take the agent (under the threshold, signed in, serving this project): it stays there.” If that account carries on in extra usage, it adds: “This account is in extra usage, billed separately.”

No back and forth: an agent that moved to an account only leaves it once that one reaches the threshold in turn, and the first is a choice again only when it resets.

## Resume an agent stopped by the usage limit

### What happens at the limit

When Claude refuses a turn because the account’s quota is used up, the agent stops. The turn’s error card, the agent’s card in the sidebar (“Resumes at 3:00 PM”) and the Kanban give the time of the automatic resume.

By default, Escouade sends “continue” to the agent about thirty seconds after the quota resets, when it knows when that is. This is the “Auto-resume after the usage limit” setting (Settings › Harness, “Processes” group). The card’s “Cancel auto-resume” button cancels it for that agent.

### Resume on another account

With several accounts, the card offers “Resume on Pro”: the first eligible account in the order of the list. When several accounts qualify, the ▾ button (“Resume on another account”) opens the menu of the others.

An account is eligible if it is active, different from the agent’s, serves its project, is signed in and is under the pause threshold: an account already at the threshold is left out, the agent would meet the limit there again.

Clicking does this:

1. Escouade stops the agent’s Claude process.
2. It copies its session (the conversation and what its subagents and tools kept in it) into the other account’s folder.
3. The agent moves to that account, and its automatic resume is canceled.
4. Escouade sends it “continue”, as for an automatic resume.

The conversation keeps a trace: “The agent resumes on the Pro account (it was on Main).”

The move has a few conditions: the agent must have finished its turn (“The agent is still working: resume once its turn is over.”) and have a session. A session also can’t go from an account that runs Claude Code on Windows to one that runs it in WSL, or the other way round.

If the turn that follows fails on the new account, the card says “Resuming on Pro failed: …” and offers “Go back to Main”. That button brings the session back to the original account, with what the failed turn added to it, and the agent waits again for the reset it was waiting for.

### By itself, for ticket agents

The “Resume a ticket’s agent stopped by the usage limit on another account” setting (Settings › Harness, “Processes” group, visible with several accounts) does the same without you. It is off by default; its description reads: “Otherwise it waits for its account to reset”.

When on, the agent of a ticket stopped by the limit moves by itself to another account when one fits: the project’s preferred account, otherwise the first one in the list. That account must be signed in, under the threshold, held back by no pause after a usage limit, with no agent waiting for its own reset, and on the same side of WSL as the agent. If there is none, or if the move fails (the conversation then says “Couldn’t resume on another account: …”), the agent waits for its account to reset, as before.

The account left receives no new ticket until it resets (30 minutes when the reset isn’t known). A project whose “Preferred account” is the agent’s own account keeps its agent there: it waits.

### Continue on another harness

When no account of the agent’s harness serving its project can take it any more (limit reached, or past “Pause past quota”), the card offers “Continue on Codex”: the project’s next harness. A project goes first to its default harness, then to the others in the “Harness order” (Settings › Accounts), only those with an active account serving it: an account “Reserved to projects” serves only the ones it ticks. The ▾ button (“Continue on another harness”) opens the menu of the next ones. With no fallback harness, the agent waits for the reset, as before.

A session doesn’t move from one harness to another: this is a handover. The agent keeps its conversation, its worktree and its branch; a new process of the target harness starts on the account its rule picks, with that harness’s “Handover model” (its best one by default, Settings › Harness) and the same mode, or the closest one that never allows more. Its first message is a handover file Escouade writes from the conversation, without asking anything of the harness it left: the original request and the later messages, the agent’s latest messages, its progress and the tasks of its plan, the changed files (`git status`, `git diff --stat`, the branch’s commits), the last turn if it was cut and where, a question or a permission left unanswered. A ticket keeps its protocol through the added instructions. The file is bounded by the target model’s context window; the conversation folds it under “Handover file”.

The conversation marks it: “Continued on Codex · Claude Code quota spent until 6:00 PM”, then each turn says the harness that ran it, and keeps its statistics and its cost.

The agent’s menu also offers “Continue on another harness…” at any time, outside a running turn.

An agent in a terminal view (Claude Code view, Codex view…) switches too: the handover file goes through Escouade’s view, then, once its turn is over, the agent moves to the target harness’s terminal view, on its new session. A harness with no terminal view leaves it in Escouade’s view, like an agent that was already there.

The harness left is set aside until its reset. After that, the card and the menu offer “Go back to Claude Code”: a handover the other way. The agent never goes back by itself.

The setting “Continue on the next harness when every account of a harness is spent” (Settings › Harness, “Processes” group, shown with two harnesses) does it without you. It is off by default. When on, the agent stopped by the limit moves by itself to the first fallback harness with an account to take it, never to the one it left; the autopilot follows its ticket on the new harness.

## Disable or remove an account

**Disable.** The “Active” switch keeps the account in the list, with its sign-in, its folder and its conversations, but no new agent, ticket or resume starts on it any more. Its state becomes “Inactive”. Agents already on it stay there; their “Account” menu shows it as “Pro (inactive)”.

**Remove.** The “Remove” button asks for confirmation. For this machine’s account, the window is titled “Remove this machine’s account” and says the harness stays signed in on this machine: removing it signs nothing out. The window starts with “Remove the account” followed by the account’s name, then says: “New agents will no longer start on it. Its folder stays on the disk, with its sign-in and its conversations:” and the folder’s path.

- While agents that aren’t archived run on the account, removal is refused: “The account is still used by 2 agents.” Archive or delete them first.
- The account’s folder stays on the disk: Escouade doesn’t erase it, and doesn’t reuse it if you add an account with the same name. To get the account back, add a new one and sign in again.
- Projects that had this account as “Preferred account” go back to “Automatic”.
- Its turns stay in the statistics, under “Deleted account (pro)”.
- An archived agent of this account that you restore later goes to the account a new agent of its project would go to on the same harness; without an account of that harness, it doesn’t start and says so.

## Quotas and statistics by account

- The status bar shows the quota of the account a new agent would start on; a click opens each account’s details. See [quotas and cost](usage).
- The [statistics](stats) can be filtered by account, and their “By account” table puts the accounts side by side.
- When an account goes past its 5-hour quota but carries on in extra usage (billed separately, if your plan has it on), the status bar says so: agents already running stay on that account, only new ones go to another. To move an agent, use “Resume on another account”, or set “Switch accounts at” below the limit: they then go on on another account at the end of their turn.
- An account with its own folder never takes on the identity of a variable in your environment (`ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_SECURESTORAGE_CONFIG_DIR`): Escouade sets them aside for it. Main keeps them, as it does outside Escouade.
