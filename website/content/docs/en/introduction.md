# Introduction

Escouade is an agentic development environment (ADE): a desktop app, for Windows, macOS and Linux, where you build software with agents from start to finish, from the ticket to the merge. This page says what it is, who it is for, the words it uses and where to find the rest of the documentation.

## What Escouade is

An ADE brings the whole development cycle into one place, carried out by agents working in parallel, each in its own conversation and, if you want, in its own git worktree:

- **plan**: a Kanban board of tickets with their acceptance criteria, their screenshots and their files, imported if need be from Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues or Azure Boards, that agents pick up on their own (that is autopilot) and loop on until the criteria are met;
- **code**: one tab per project, as many agents as you want, each with a readable conversation (markdown, tool calls you can unfold, questions and permission requests as cards) or Claude Code’s own interface; its subagents and background tasks in the sidebar, the progress of its plan, a file editor and real terminals;
- **review and test**: the changed files, their diffs, the repository’s git graph and its branches, “▶ Test” on each ticket and the commands that run the project;
- **ship**: the commit, the merge, squash or rebase into the target branch, the push or the pull request;
- **steer**: notifications when an agent is waiting for your answer or has finished, the overview of every agent, quotas, cost and statistics, several accounts, an MCP server that lets an agent drive Escouade, and remote control from your phone.

Today, each agent is a real session of [Claude Code](https://code.claude.com/docs/en/overview), Anthropic’s command-line tool for programming with Claude: Escouade starts `claude` for it and shows its conversation. The session also stays on your disk, in Claude Code’s own files. The app runs on your machine: your projects, conversations and settings are in `~/.escouade/` (see [local data](data)). It is free and open source, under the GPL-3.0 license (or later).

## What Escouade is not

- **An Anthropic product.** It is an independent project, not affiliated with Anthropic. Claude and Claude Code are trademarks of Anthropic.
- **An agent engine.** Claude Code must be installed on the machine and signed in to your Claude account: Escouade contains no model and no subscription, it uses yours. See [installation](installation).
- **One more IDE.** Its editor is enough to review and touch up what the agents do, but it has no language server and no extensions: keep your IDE for the rest, each worktree is a real folder you open wherever you like. See [the editor](editor).

## Who it is for

Anyone who builds software with agents and wants to carry the whole cycle through in one place, without juggling terminals, a ticket tool and a Git client:

- you work on several tasks or several projects at the same time, and you want to see at a glance which agent is working, which one is waiting for your answer and which one has finished;
- you want to review and commit what each agent changed, without the agents stepping on each other;
- you write your tasks as tickets, in Escouade or in Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues or Azure Boards, and you want to hand them to agents that keep going until the acceptance criteria are met;
- you have several Claude accounts (a personal one, a work one) and you want to keep an eye on their quotas.

## The basic concepts

These words come back throughout the interface and this documentation.

### Project

A project is a folder of code, opened in a tab of the title bar. It has its own color, agents, Kanban, terminals and settings. If the folder is not a git repository, Escouade initializes one when the project is created. See the [quick start](quick-start).

### Agent and statuses

An agent is a conversation with Claude Code in a project. It always has one of these five statuses, which you read in the sidebar, in the header of its conversation and in the status bar:

| Status | What it means | Sign |
|---|---|---|
| “Ready” | The agent exists and has not received a request yet. | Gray dot |
| “Running” | Claude is working on a request. | Green dot that pulses |
| “Question” | Claude is waiting for your answer: a question or a permission request. | Amber dot that pulses |
| “Done” | Claude has finished its request. | Green check mark |
| “Error” | The request stopped on an error. A new message restarts Claude on the same session. | Red dot |

See [agents](managing-agents).

### Conversation

The conversation is the exchange between you and an agent: your messages, Claude’s replies, its tool calls and the cards for questions and permissions. It is kept from one restart to the next, and each agent picks its Claude session up where it left off. See [the conversation](chat).

### Ticket and Kanban

A ticket describes a task: a title, a description and acceptance criteria. A project’s Kanban, opened with the “Kanban” button of the sidebar, sorts tickets into four columns: “To do”, “In progress”, “To review” and “Done”. An agent takes a ticket, works on it and loops until its criteria are met; you test it, then you approve it. You can also import tickets from Jira, Trello, GitHub Issues and Linear. See [the Kanban](board).

### Autopilot

Autopilot is what starts the “To do” tickets by itself as soon as a slot is free. It is on by default for a project: the “In parallel” setting sets how many agents take tickets at the same time (from 1 to 6, 2 by default), each in its own worktree. It pauses near the limit of your quotas (the “Pause above quota” setting). See [autopilot](autopilot).

### Worktree

A worktree is a separate working copy of your repository, on its own branch. With the “One git worktree per agent” option, each new agent works in its own: its changes mix neither with those of the other agents nor with your folder, and you review and merge them afterwards. The agents of tickets each work in their own worktree. See [worktrees](worktrees).

### Claude account

A Claude account is a Claude Code configuration folder, with its sign-in. There is none to begin with: in Settings › Accounts and harnesses, add your usual account (“This machine’s account”, named “Main”, started as before), or a new one (a personal one, a work one). Each agent stays on its account. See [Claude accounts](accounts).

### Overview

The “Overview” is a page that lists the agents of all projects, the ones waiting for your answer first. You open it with its button in the title bar or with `Ctrl+Shift+A`. See [keeping track of agents](overview).

## Supported systems and languages

| System | Versions |
|---|---|
| Windows | Windows 10 or 11 |
| macOS | macOS 11 (Big Sur) or later, on Apple Silicon or Intel Macs |
| Linux | x86-64, with glibc 2.35 or later (Ubuntu 22.04, Debian 12, Fedora 36 and later) |

The full requirements are in [installation](installation).

The interface comes in French and English. By default it follows the system language: French if the system is in French, English otherwise. Settings › General lets you pick either one without restarting, and pick separately the language of the texts Claude writes (commit messages, pull request descriptions, comments posted to Jira, Trello, GitHub, Linear, ClickUp, GitLab and Azure Boards, instructions given to agents), because many teams commit in English with an interface in French. See the [application settings](settings-application).

## Map of the documentation

- **Get started**: [installation](installation), the [quick start](quick-start) and a tour of [the interface](interface).
- **Agents and chat**: start with [agents](managing-agents), then the conversation, the overview, notifications, the Claude Code view and remote control.
- **Code and Git**: [Git and commits](git), then worktrees, branches, the [code hosts](code-hosts), the editor, terminals and running the project.
- **Kanban and automation**: [the Kanban](board), then autopilot, testing and approving tickets, integrations and the MCP server.
- **Accounts and usage**: [Claude accounts](accounts), then quotas, cost and statistics.
- **Settings**: every page of the settings window, option by option, starting with [the window and General](settings-application).
- **Reference**: the [keyboard shortcuts](shortcuts), local data, updates and [troubleshooting](troubleshooting).

## Support Escouade

Escouade is free and open source. If it helps you, you can [buy a coffee](support:) to help it move forward, for the amount you choose: the “Buy a coffee” button of [Settings › About](settings-about) opens the same page.
