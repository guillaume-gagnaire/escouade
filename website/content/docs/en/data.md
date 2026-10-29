# Local data

Everything Escouade keeps on your disk: the ~/.escouade/ folder and each of its files, where tokens live, what it reads of Claude Code’s data, and how to back up or reset.

## The data folder

Escouade keeps its data in `~/.escouade/` (`C:\Users\<you>\.escouade` on Windows). Settings › About reminds you of it on the “Local data” line.

| File or folder | Contents |
|---|---|
| `settings.json` | The settings. |
| `state.json` | The projects, the agents (with the ID of their Claude Code session), the Kanban tickets, the autopilot’s pause, and the interface state. |
| `integrations.json` | The Jira, Trello, GitHub, Linear, ClickUp, GitLab, and Azure Boards accounts, without their tokens (see [Tokens](#tokens)). |
| `sync-queue.json` | The syncs with those services that are waiting to be retried. |
| `tickets/<ticket>/` | The images pasted or dropped in a ticket’s window, or attached through the MCP server, one per file: `state.json` only has their names. The ticket’s other attached files are copies under `files/<random>/<name>`, in the same folder (`state.json` only keeps their name, location, and size). The folder only exists when the ticket has images or files; deleted with the ticket, or when the history of its closed project is erased. |
| `attachments/<agent>/` | The files you attach to an agent’s messages, or paste or drop into its terminal (Claude Code view), when they are copied: each in a subfolder with a random name (8 characters), under its original name, cleaned up. `attachments/_drafts/` holds the copies of a ticket not saved yet, emptied after 24 h (at startup, then every hour). Deleted with the agent, with its project when you delete it for good, or by the purge if you set it (see [Settings › Harness](settings-claude-code#attachments)). A file attached at its original location is never in here. |
| `conversations/<agent>.jsonl` | The log of each conversation, named after the agent’s ID, one operation (an item added or corrected) per line. Deleted with the agent; those of a closed project stay with its history. |
| `closed-projects/<project>.json` | The history of a closed project, named after the project’s ID: the whole project (settings, Kanban, integrations, commands), its agents (archived ones included), its tickets, the agent on display, and the closing date. Its conversations stay in `conversations/`, its tickets’ images and files in `tickets/`, the copies of files attached to its agents in `attachments/`, its statistics in `stats.db`. The file goes away when the project is reopened or its history is erased (see [Closed projects](settings-projects#closed-projects)). |
| `claude/<account>/` | The Claude Code configuration folder of each account added in Settings › Accounts: its sign-in and its conversations. This machine’s account (Main) has none; it uses your own Claude Code’s folder. |
| `mcp-token.json` | The MCP server’s token for Claude outside Escouade, only when the system keychain refuses it (always on Linux). |
| `mcp/<agent>.json` | The MCP configuration of an Escouade agent, with its token. Deleted when its process stops, and each time the app starts and quits. |
| `tui/<agent>.json` | The hook settings that an agent’s Claude Code view passes to Claude Code. The token isn’t in it: it goes through an environment variable. |
| `logs/<agent>-setup.log` | The full output of the last setup of an agent’s worktree. Deleted with the agent; that of a closed project stays with its history. |
| `voice/models/` | The Whisper models downloaded for dictation and the voice conversation (`ggml-<model>.bin`). Each one is deleted from [Settings › Voice](settings-voice#whisper-s-models). |
| `voice/voices/` | The voice conversation’s downloaded voices (`<voice>.onnx` and `<voice>.onnx.json`). Each one is deleted from Settings › Voice. |
| `voice/espeak-ng-data/` | eSpeak NG’s pronunciation data (French and English), written by the app at the first reading aloud and written again after an update. |
| `plugins/<id>/` | The copy of each plugin installed from a folder or a git repository, without its git history (see [the manifest](plugins/manifest#installing-a-plugin)). Removed when the plugin is uninstalled. |
| `plugins.json` | The list of installed plugins: their origin (folder, repository and tag, folder in development) and their fingerprint. |
| `plugin-logs/<id>.log` | The journal of each plugin with a program: its starts, its stops, its crashes, its standard error and its messages, without its secrets (see [the protocol](plugins/api#the-journal)). Past 1 MB, it becomes `<id>.log.1`. |
| `update.json` | The update being installed, noted for the next start and then deleted. |
| `stats.db` | The statistics (a SQLite database, with its `stats.db-wal` and `stats.db-shm` files while Escouade runs). |
| `telemetry-id` | The random id of this install, for the usage statistics. Created when you accept them, deleted when you refuse or turn them off. |
| `app.log` | The app’s log. At startup, if it is over about 5 MB, it becomes `app.old.log`. |
| `settings.broken.json`, `state.broken.json`… | Copies of files the app couldn’t read (see [An unreadable file](#an-unreadable-file)). |

## The old Claude Code Manager folder

Escouade used to be called “Claude Code Manager” and kept its data in `~/.claude-code-manager/`. At the first launch that finds it, Escouade moves it to `~/.escouade/` and leaves a `.migrated-from-claude-code-manager` file in it. The web engine’s and window state’s folders, named after the app identifier, follow the same path. In practice, an update keeps your settings, projects, and conversations: see [updates](updates).

The rare cases are handled without losing anything:

- A `~/.escouade/` folder that is already there without that file (made by a launch that couldn’t move the old folder) is never deleted: it is set aside under the name `.escouade.before-migration-` followed by a number, and the old folder takes its place.
- If the old folder can’t be moved because a file is still open, Escouade keeps using it and tries again at the next launch.

## Choosing another folder

The `ESCOUADE_DATA_DIR` environment variable, when it is set and not empty, replaces `~/.escouade/` with the folder it names. The old `CCM_DATA_DIR` variable still works, but `ESCOUADE_DATA_DIR` takes precedence. Set it in the terminal you launch Escouade from.

It is meant for demos and trials. With a separate folder:

- Escouade moves nothing from `~/.claude-code-manager/`.
- Tokens go to a separate keychain entry (the `dev.gagnaire.escouade.sandbox` service): they never replace yours.
- Another instance can run alongside the one that uses the usual folder.
- The automatic restart for an update is turned off.

## Tokens

Tokens aren’t in the settings:

- **Jira, Trello, GitHub, Linear, ClickUp.** Each service’s token is in the system keychain: Windows Credential Manager, macOS Keychain Access. There is one entry per service (`jira`, `trello`, `github`, `linear`, `clickup`) under the `dev.gagnaire.escouade` service. `integrations.json` keeps the rest of the account. When the keychain refuses the entry, Settings › Integrations says so (“System keychain unavailable: the token stays in ~/.escouade/integrations.json.”) and the token stays in that file, readable only by you on macOS and Linux.
- **Linux.** There is no supported keychain: the integration tokens stay in `integrations.json`.
- **MCP server.** The token for Claude outside Escouade is in the keychain (an `mcp` entry), otherwise in `mcp-token.json`. Each agent’s is in its `mcp/<agent>.json`, which goes away with its process.
- **Online dictation.** Each service’s key is in the keychain, one entry per service (`dictation-openai`, `dictation-groq`, `dictation-mistral`, `dictation-elevenlabs`, `dictation-deepgram`), never in a file: on Linux, it cannot be saved. “Delete the key”, in Settings › Voice, removes its entry. See [dictation](chat).
- **Claude Code’s sign-in.** It isn’t Escouade’s: Claude Code keeps it in each account’s folder (the `.credentials.json` file), and on macOS in the keychain. See [Claude accounts](accounts).

To remove a token, disconnect the service in Settings › Integrations: its entry leaves the keychain. See [integrations](integrations).

## What Escouade reads of Claude Code

Escouade drives Claude Code and keeps its own conversations, but it also relies on Claude Code’s data:

- **Sessions.** Each agent has a Claude Code session. Escouade notes its ID in `state.json` and runs `claude --resume` on that session when the app restarts or a stopped process picks up again: the agent gets its whole context back. The sessions themselves are in the Claude Code configuration folder of the agent’s account (`~/.claude` by default for this machine’s account, `~/.escouade/claude/<account>/` for the others), under `projects/`. Deleting an agent removes its conversation from Escouade; the Claude Code session stays on disk. If a session has disappeared, the agent says so (“Claude session not found: a new session will start with your next message.”).
- **The sign-in, for quotas.** Escouade reads, without writing it, the sign-in that Claude Code keeps for each account, and uses it to ask Anthropic for the account’s quotas. See [quotas](usage).
- **The transcript, in the Claude Code view.** To follow an agent that runs in Claude Code’s own interface, Escouade reads the session it writes (`projects/<folder>/<session>.jsonl`). See [the Claude Code view](claude-code-view).
- **The MCP configuration.** When “Agents can drive Escouade” is on (Settings › MCP server), Escouade declares its MCP server in the configuration of each active account of each harness installed (Claude Code, Codex, OpenCode, Copilot CLI), the token in plain text. See [the MCP server](mcp).
- **A new account** takes from this machine’s account, by link or by copy, its settings, its `CLAUDE.md`, and its skills, agents, commands, plugins, hooks, and output styles. Never its sign-in or its `.claude.json`.

## Usage statistics

If you accepted them (the first-start question, or the switch in Settings › About), Escouade sends usage statistics, with nothing that identifies you, to a server of Escouade’s hosted in France. On your disk they leave only `telemetry-id`, a UUID drawn at random, derived from nothing about your machine or about you. Events wait in memory and are never written to a file.

The detail of what is sent, what never is, and how to ask for deletion is on the [Privacy](/en/privacy/) page. See also the [“About” settings](settings-about).

## What isn’t in the folder

- **Your projects and the agents’ worktrees.** They are yours, in each project: `.claude/worktrees` by default, or the “Worktrees folder” set for the project. Closing a project doesn’t touch them.
- **Claude Code’s sessions**, seen above.
- **The window’s size and position.** The `.window-state.json` file is in the system’s configuration folder, under `dev.gagnaire.escouade`: `%APPDATA%\dev.gagnaire.escouade` on Windows, `~/Library/Application Support/dev.gagnaire.escouade` on macOS, `~/.config/dev.gagnaire.escouade` on Linux.
- **A few display preferences**, kept by the app’s web engine: the text you have started in a message field (it survives a restart), the width of the editor’s file tree, the height of the “Plan” banner, how diffs are shown, the statistics period.

## Backing up

1. Really quit Escouade: closing the window leaves it in the notification area. Use “Quit” in the menu of its icon.
2. Copy the `~/.escouade/` folder.

The copy holds your settings, projects, tickets, conversations, and statistics. It also holds, under `claude/`, the sign-in of the Claude accounts you added (on Windows and Linux): keep it private.

To restore it on another machine, put the folder back in the same place, with your projects at the same paths (`state.json` remembers full paths). Three things don’t come along:

- The integration tokens (except on Linux): reconnect each service.
- The sign-in of this machine’s account: it is in your Claude Code.
- The agents’ sessions, which are in Claude Code’s folder: without them, an agent starts over with a new session.

## Resetting

Quit Escouade first, then:

| To… | Do… |
|---|---|
| Get the default settings back | Delete `settings.json`. |
| Erase the statistics | Delete `stats.db`. |
| Start over with no project, agent, or ticket | Delete `state.json`, and `conversations/` so the logs don’t stay behind. Also delete `closed-projects/` to forget the closed projects. |
| Erase the history of a closed project | In Settings › Projects, “Closed projects”, “Erase history…”: it deletes its file in `closed-projects/` and the logs of its agents. Deleting the file by hand would leave its logs in `conversations/`. |
| Erase everything | Delete the `~/.escouade/` folder. The sign-ins of the accounts added under `claude/` go with it. |
| Remove the integration tokens | Disconnect each service in Settings › Integrations. |

Your projects’ files, the agents’ worktrees, and Claude Code’s sessions are never touched by a reset: remove the worktrees you no longer want with git.

## An unreadable file

When `settings.json`, `state.json`, `integrations.json`, or `sync-queue.json` isn’t valid JSON, Escouade writes the error to `app.log`, keeps a copy of the file under a name like `state.broken.json`, and starts with the defaults for that file. The copy lets you recover by hand what was in it, such as the list of projects. See [troubleshooting](troubleshooting).
