# Quick start

A first session step by step: add a project, create an agent, choose its model, effort and permission mode, give it a request, answer its questions, review what it changed and commit.

## Before you start

You need Claude Code installed and signed in to your Claude account, and git (see [installation](installation)). On macOS, `Ctrl` becomes `⌘` in the shortcuts of this page.

On the first launch, Escouade has no account: the first screen says “No account”. Click “Add an account…”, keep “This machine’s account”, your Claude Code already signed in, then “Add this account” (see [multiple accounts](accounts#no-account-to-begin-with)). Without an account, no agent starts.

## Add a project

A project is a folder of code. On the welcome screen, click “+ Add a project”. Later, the `+` button to the right of the tabs in the title bar does the same. The “New project” window opens: an assistant in steps, listed on the left, that sets everything Escouade’s way of working relies on (agents in worktrees, Kanban tickets, launches, approval) from the first ticket. Each step has “Next” and, except the first one and the summary, “Skip”, which keeps its values as they are; clicking a step in the list goes straight to it.

**1. Start.**

| Field | What it does |
|---|---|
| “Folder” | The path of the project. “Browse…” opens the folder picker. Under the field, a line says what Escouade knows about the folder: “Git repository detected · branch main · clean” (or the number of changed files), “No git repository · one will be initialized” or “Folder not found”. |
| “Name” | Filled in with the folder’s name, which you can change. “Tab preview” shows the tab as it will look. |
| “Color” | One of the 14 colors offered; the first one no project uses yet is picked for you. It tints the tab and the interface while the project is on screen. |
| “Starting point” | “Copy a project’s setup” (with the list of open projects, the one on screen first, private ones left out), “Recommended setup” (chosen by default) or “Start from scratch” (Escouade’s defaults). |

The recommended setup: a worktree per agent, approval by a squash merge into the current branch, worktree deleted once approved, conventional commits, the agent resolves conflicts, commits written by the agent.

**2. Copied settings** (only when copying). The other project’s settings are listed by group (“Worktrees”, “Launch”, “Kanban”, “Git and account”), each with its box, and checked in the new folder: everything is checked except what would not work here, with the reason beside it. A command whose folder is not in the new repository (“folder “web” not found here”) and a worktrees folder written as a full path (the other project’s) are not taken; a target branch not found is replaced by the current branch (“branch not found: main instead”); a preferred account that is off, reserved to other projects or removed falls back to “Automatic”. The linked sources (Jira, Trello, GitHub, Linear), the tickets, “Private project” and the reserved accounts are never copied: a private project is never offered, and its reserved accounts stay its own. What is taken shows in the next steps, where you can change it.

**3. Worktrees**: one worktree per agent, worktrees folder, branch prefixes, copied files, setup and teardown commands, with “✦ Fill in automatically”, where Claude reads the folder to suggest them (see [worktrees](worktrees)). **4. Launch**: the commands that launch the project, with “✦ Fill in automatically” too (see [launch](launch)). **5. Kanban**: action on approval, target branch, strategy, worktree removal, conventional commit message, conflicts, test command and “Run the tests again first”, agents in parallel, autopilot, model, effort and mode (see [Settings: Kanban](settings-board)). **6. Account and commits**: default harness (when several are installed), preferred account, who writes the commits, remote control, then “Private project” (on from the start if “New projects are private” is on) and, for a private project, the accounts to reserve to it (those serving no other project) and “Shared accounts at the limit” (see [Settings: Projects](settings-projects)). The ticked accounts are reserved to it when it is created. **7. Integrations** (optional): a source to link for each service whose account is connected.

**8. Summary.** What would keep the way of working from working, each with its correction, then what will be set, and “Create a first agent” (with its model):

| Check | Correction |
|---|---|
| “The repository has no commit”: no ticket starts, a worktree starts from a commit | “Make a first commit”: an empty commit, no file is added |
| “The target branch … is not found” | “Take the current branch” |
| “Worktrees would go in a folder git tracks” | “Use .claude/worktrees” |
| “No Claude account serves this project” | Switch an account on, or lift its reservation, in Settings › Accounts |
| “Claude Code is not found” | Install it, or give its path in Settings › Harness |

Each check is also told in the step it concerns (the first one from “Start”). Click “Create project”: the project opens in a new tab with its settings, then its first agent if you asked for one. “Create without the assistant”, at the bottom left, creates the project at once with Escouade’s defaults and a first agent, as before the assistant. Later, Settings › Projects › “Open the assistant…” opens it again on an existing project (see [Settings: Projects](settings-projects#setup-assistant)).

If the folder is the one of a project you closed, the history comes before the assistant: the window adds a notice, for example “History found: 1 agent, 3 tickets, last activity on Oct 14, 2026”, and its button becomes “Reopen project”: the project comes back with its history (agents, conversations, tickets, settings), with no step. The “Start from a blank project” switch creates a new one instead, through the assistant. See [Settings: Projects](settings-projects#closed-projects).

> [!NOTE]
> If the folder is not inside a git repository, Escouade runs `git init` to create one: without git, there would be no diff and no commit to review.

## Create an agent

An agent is a conversation with Claude Code in this project. To create one, press `Ctrl+N` or click “+ New agent” at the top of the sidebar: the agent is created without asking you anything, and its conversation appears. It is called `agent-1`, `agent-2`… until its first request; Escouade then gives it a short name taken from what you wrote. To rename it yourself, double-click its name.

Its status is “Ready”, and the empty conversation says “Agent ready”, with the folder it works in. The ▾ arrow to the right of “+ New agent” offers “New agent on a branch…” (see [branches](branches)).

## Choose the model, the effort and the permission mode

Under the message field, three menus set up the agent (a fourth one, “Account”, appears when you have several Claude accounts). You can change them in the middle of a conversation.

| Menu | Choices |
|---|---|
| “Model” | “Fable”, “Opus”, “Sonnet” and “Haiku”. Each runs the latest model of its family that Claude Code knows, and shows its version (for example “Sonnet 5.5”). |
| “Effort” | “Low” (quick answers, little thinking), “Medium” (balanced), “High” (deep thinking), “Very high” (very deep thinking), “Max” (maximum thinking, uses more tokens). Haiku doesn’t support effort. |
| “Mode” | The permission mode, described below. |

The permission modes decide when Claude asks you for permission to act:

| Mode | What happens |
|---|---|
| “Auto” | A classifier approves safe actions and asks about the rest. Unavailable with a model without it, like Haiku 4.5 (Claude Code 2.1.292 and earlier). |
| “Ask” | Claude asks for your approval before each sensitive action. |
| “Plan” | Claude analyzes and proposes a plan without changing anything. |
| “Accept edits” | File edits are accepted without asking. |
| “Bypass” | No permission requests. |

A new agent starts with the model, effort and mode of its account, set on its card in Settings › Accounts, “New agents” (by default the harness’s: Sonnet, Medium and Auto for Claude Code). See the [account settings](settings-accounts) and [agents](managing-agents).

> [!WARNING]
> In “Bypass” mode, Claude acts without any permission request: Escouade reminds you of it when you choose it. Keep it for safe environments.

## Send a first request

Click in the message field, describe the task (“Describe the task to give to Claude…”), then press `Enter` to send. `Shift+Enter` starts a new line.

- Type `@` followed by a bit of a file name to pick a file of the project, or `/` to pick a command.
- Paste or drop any file (an image, a PDF, a text file, but also an archive or a spreadsheet) into the field, or click the paperclip, to attach it to the message.

As soon as you send, the agent’s status changes to “Running” and the conversation says “Claude is working…”. Its tool calls show up as compact rows that you can unfold. You can write to it while it works: it takes your message into account at its next step. To stop it, press `Esc` in the message field or click “■ Stop”.

## Answer a question or a permission request

When Claude needs you, a card with an amber border appears in the conversation, the agent’s status becomes “Question”, and its tab and card blink if you are looking elsewhere. There are two cards.

**“Claude is asking for permission”** names the tool and what it is about to do. Three buttons:

- “Allow” (`Ctrl+Enter`);
- “Always allow” (`Ctrl+Shift+Enter`), when Claude Code offers to remember the choice;
- “Deny”.

To deny and explain what to do instead, write it in the message field and send it. When Claude Code would deny the request by default, “Deny” is the main button and only a click on “Allow” grants it: the keys don’t answer. When Claude proposes a plan, the card is called “Claude proposes a plan” and the buttons are “Approve the plan”, “Approve and accept edits” and “Keep planning”.

**“Claude is waiting for your answer”** asks a question with options. Click an option, or press `Alt+1` to `Alt+9`, or answer freely in the message field. With a single single-choice question, the answer goes out as soon as you click; with several questions or a multiple choice, “Submit” (`Ctrl+Enter`) sends the answers.

> [!TIP]
> `Ctrl+J` opens the next agent that is waiting for your answer, with the cursor in its message field. A card that has just arrived ignores the answer keys for half a second, so that a keystroke meant for your message doesn’t answer in its place.

## See what the agent changed

When Claude has finished, the status becomes “Done” and a “Task done” card gives the duration, the tokens, the cost and the list of edited files; a click on a file opens it in the editor.

To review the changes, open the “Uncommitted” panel: click “Files ▸” in the header of the conversation, or press `Ctrl+Shift+B`.

- Each row is a file, with its letter (M for modified, A for added, D for deleted) and its added and removed lines.
- A click on a file opens its diff in a large window. “Unified” and “Side by side” change the layout, `Esc` closes the window. When there are several files, a column on the left lets you switch from one to another.
- “Show diff” opens the diff of all the files in the list.
- “This agent” lists the agent’s files; “Whole project” lists those of all the project’s agents.

To keep the conversation and the diff in view together, switch to the side-by-side layout with `Ctrl+Shift+L`: the panel stays displayed on the right and shows the diff of the chosen file under the list.

![An agent whose “Uncommitted” panel lists three files, with the diff of the first one to the right of the conversation](/images/en/git.jpg)

## Commit

The “Commit…” button of the panel commits the agent’s changes (“Commit all…” when “Whole project” is chosen). What happens depends on the project’s “Commit” setting, in Settings › Projects:

- **“Written by the agent”** (the default): Escouade sends the agent a message asking it to commit its changes with a clear message in Conventional Commits format. A short-lived message confirms “Commit request sent to” followed by the agent’s name, then the agent commits like it does for any request.
- **“Direct, with a suggested message”**: the “Commit” window lists the files, Haiku writes a message that you read and edit, “Regenerate” asks for another one, and “Commit” commits. Nothing is committed without that click.

If the agent works in a worktree, the panel also offers “Merge … → …”, which merges its branch into the project’s. See [Git and commits](git) and [worktrees](worktrees).

## What next

- [Agents](managing-agents) and [the conversation](chat): permission modes, messages, attachments, search, history.
- [Notifications](notifications) and [keeping track of agents](overview): get notified, answer without opening each conversation.
- [Git and commits](git), [worktrees](worktrees) and [branches](branches): review, commit, isolate agents.
- [The editor](editor), [terminals](terminals) and [running the project](launch).
- [The Kanban](board) and [autopilot](autopilot): hand tickets to agents.
- [Claude accounts](accounts) and [quotas and cost](usage).
- [The keyboard shortcuts](shortcuts) and [the interface](interface) for a tour of the window.
