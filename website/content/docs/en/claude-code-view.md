# The terminal view

The terminal view runs an agent in its harness’s interface (Claude Code, Codex, OpenCode, Copilot CLI, Cursor CLI, Mistral Vibe, Antigravity CLI or Gemini CLI), in a terminal, without losing its session. This page explains how to switch views, what Escouade keeps tracking, and what changes.

## What it’s for

Escouade’s conversation shows the essentials of an agent’s work, but not everything Claude Code offers. In its own interface, you get all of it: `/` commands, shortcuts, dialogs, `/model`, `Shift+Tab` to change mode… It’s useful when you want to work the way you would in a terminal, while keeping the agent in Escouade’s list with its statuses, notifications, and cost.

## Switching views

In an agent’s conversation header, the two segments “Escouade” and “Claude Code” choose the view: “Claude Code” launches Claude Code’s interface in a terminal that takes the place of the conversation, “Escouade” takes you back to the conversation. The view shown is named, the other one is only an icon. The menu of the agent’s card in the sidebar (right click) offers the same: “Claude Code view” or “Escouade view”. The session is the same in both directions: Escouade stops the process of one view and resumes the session in the other (`--resume`), so the agent forgets nothing. The choice is kept for each agent. The interface starts when you show the agent (the terminal then says “Starting Claude Code…”), including after Escouade restarts, and picks up its session. An agent in remote control is the exception: when Escouade starts, its interface is started in the background so that the agent is reachable (see [below](#remote-control-in-this-view)).

Switching to the Claude Code view is refused in two cases, with a message; switching back to the Escouade view is refused in none of them:

- **The agent is still working**: switch views once its turn is over.
- **The agent works on a ticket**: a ticket agent stays in Escouade’s view, which its board drives (see [the Kanban](board)). Neither the button nor the entry of its card’s menu exists for it, nor for an archived agent.

If the interface stops (you typed `/exit`, or the process was killed), the terminal shows “Claude Code stopped.” and a “Restart” button; if it doesn’t start, “Claude Code didn’t start:” followed by the error, with the same button. Archiving or deleting the agent also closes its terminal.

## What Escouade keeps tracking

Claude Code’s interface is a terminal, but Escouade still knows what the agent does. It reads it in two ways: Claude Code’s hooks, which the interface calls on your machine (`127.0.0.1`, with a token of its own for each interface), and the session’s transcript, which the app rereads continuously. Your own hooks stay active.

Thanks to this, an agent in the Claude Code view keeps:

- **its dot and its statuses**: “Running”, “Question” when it’s waiting for a permission or an answer, “Done” at the end of a turn;
- **its notifications**: the blinking, the chime, and the system notification when it’s waiting for a permission or an answer, then when it finishes (see [notifications](notifications));
- **its conversation**: what happens in the interface is also written to Escouade’s conversation, which you find again when you return to the “Escouade view”;
- **search** (`Ctrl+K`, see [keeping track of your agents](overview#searching-conversations));
- **the “Plan” banner** (see [plan progress](plan));
- **its subagents**: their list in the sidebar and, read-only, their conversation (see [below](#subagents));
- **its background tasks**: their list in the sidebar and their view, without “Stop” (see [below](#background-tasks));
- **its statistics and its cost** (see [statistics](stats)). The interface doesn’t give the cost of each turn: Escouade estimates it from public prices.

When the agent is waiting for an answer, you give it in the terminal: the [overview](overview) reminds you (“Waiting for your answer in Claude Code’s interface”) and its “Reply” button opens the agent.

## Subagents

The subagents of an agent in the Claude Code view are listed as elsewhere, under its card in the sidebar and in the “Plan” banner (see [subagents in the sidebar](plan#subagents-in-the-sidebar)). A click on one opens its conversation in place of the terminal; the breadcrumb, or `Esc`, takes you back to the agent.

This conversation is **read-only**. Escouade reads it in the transcript Claude Code keeps for this subagent, next to the agent’s: it’s read when the view opens, read again every 2 seconds while the subagent runs, then one last time when it finishes. The view says so at the top (“Read-only: its transcript, as Claude Code writes it. This subagent is acted on in Claude Code’s own interface.”). There’s no “Stop” and no “Write to it”: to stop a subagent or write to it, use Claude Code’s interface, in the terminal. If Escouade can’t find that transcript (Claude Code hasn’t written it yet, or keeps it elsewhere), the view says “No message from this subagent yet.”

In this view, no signal tells Escouade that a background subagent has finished. It works it out from what Claude Code replays to the agent in its transcript: the report the subagent hands back to it, or the notification of its end. Until that message has arrived, the subagent’s line stays “Running”. Its report is the one it handed back to the agent.

## Background tasks

The tasks that Claude Code starts in the background for an agent in the Claude Code view (commands, monitors…) are listed as elsewhere, under its card in the sidebar, their number on the card (see [background tasks](plan#background-tasks)). A click on one opens its view in place of the terminal; the breadcrumb, or `Esc`, takes you back to the agent.

In this view, no signal announces them to Escouade. It reads a task’s start in the result of the tool that Claude Code writes in the agent’s transcript, with the call’s command, and its end in the notification Claude Code replays there. Until that notification has arrived, the task’s line stays “Running”, and the start time isn’t exact to the second. There’s no “Stop”: the view says “This task is stopped from Claude Code’s own interface.”, and you stop the task in the terminal. The output is read from the output file Claude Code writes for the task (the view says “read from the output file”), since no request can ask Claude Code for it. Through the [MCP server](mcp#an-agent-s-subagents-and-background-tasks), Claude can list them and read their output, but `stop_background_work` and `message_subagent` are refused for an agent in the Claude Code view: there, a task is stopped and a subagent is written to from Claude Code’s own interface.

## Writing to an agent in this view

Escouade’s message field isn’t shown: you write in the terminal, as in Claude Code. To attach an image, such as a copied screenshot, paste it into the terminal (`Ctrl+V`, `Cmd+V` on macOS) or drop it onto it: Escouade saves it to a file and types its path, which Claude Code takes for the image (“[Image #1]”). Pasted text is typed as it is.

Any other file (an archive, a spreadsheet, a document…) is attached the same way: paste it into the terminal, or drop it onto it (a dashed frame shows the drop zone). Escouade always copies it, and never reads the file at its original location: Claude Code’s interface is launched once, and only the agent’s folder can be read there without asking. It then types the path of the copy (`/mnt/c/…` for a Claude Code in WSL), which Claude reads with its tools. The copy sits in the agent’s folder, with the chat’s copies (see [attachments](chat#attachments)). The maximum size from the [settings](settings-claude-code#attachments) applies; the size warning doesn’t exist here, for lack of a chip.

To dictate rather than type, click the microphone at the top right of the terminal (or press `Ctrl+Shift+Space`, `⇧⌘Space` on macOS): one click starts and another ends, a long press dictates while it is held. The text is typed into Claude Code’s prompt as a paste, at its cursor, and is not submitted: you read it, then press `Enter`. The transcription runs on your machine (see [dictating a message](chat#dictating-a-message)).

A message sent to the agent from elsewhere, for example through the [MCP server](mcp), is typed into its interface. It’s refused while the agent is waiting for an answer in its terminal (“The agent is waiting for an answer in its terminal: answer there first.”). It is also refused for an interface started in the background that nobody has shown yet, as long as it isn’t ready (see [below](#remote-control-in-this-view)). An interruption requested from Escouade becomes the `Esc` key in the interface.

## Remote control in this view

[Remote control](remote-control) follows the same rules as in the Escouade view: you turn it on with a right-click on the agent’s card or with a setting (a project’s or the global one), and the agent is reachable from claude.ai and the Claude app as `project · agent`. Switching to the Claude Code view isn’t refused when it is on. Claude Code’s interface then holds the remote session.

- **Starting.** The interface of an agent in remote control is started with `--remote-control=<project · agent>`: Claude Code shows the session’s link. When the interface resumes its session (Escouade restarting, a relaunch), Claude Code reattaches the same remote session, with the same link. A closed interface archives its remote session.
- **When Escouade starts.** The interface of such an agent is started in the background, without you opening it. Opening the agent shows the interface already running; if its remote control goes off before then, it stops, unless it is in the middle of a turn. These starts are made at most 3 at a time: a start keeps its place until the interface is ready, 15 seconds at most.
- **A change.** Escouade can’t turn remote control on or off in an interface that is already open: a change takes effect the next time the interface starts. After a right-click, a message says so; for a setting, its description does. Until then, Claude Code’s `/remote-control` command does it.
- **The card’s menu.** It offers “Turn on remote control” or “Turn off remote control”. “Open on claude.ai” and “Copy the claude.ai link” are offered too, once the interface is connected: the link comes from the interface, which shows it in its own.
- **The card’s icon.** Its tooltip says how the interface is doing: no interface running (not reachable for now), connecting…, connected, from the next launch of the interface (open before remote control was turned on), or not opened by Claude Code (no claude.ai subscription, network, proxy or organization policy). Off for Escouade but still on in the open interface, remote control keeps a crossed-out icon, in the warning color, until the next launch, if the interface really had its link. The detail is in [remote control](remote-control#the-icon-of-an-interface).
- **A refused message.** As long as an interface started in the background has never been shown and isn’t ready (first-launch screen, a folder to approve), a message from the window or from the MCP server is refused: open the agent to see the interface.

## Opening new agents in this view

In Settings › Agents, group “New agents”, the “New agents’ view” choice, “Escouade” (the default) or “Harness terminal”, makes each new agent start in Escouade’s conversation or in this view. Each agent can then switch views from its header. Ticket agents always stay in Escouade’s view, and so do the agents of a harness that has no terminal view. See [the harness settings](settings-claude-code).

## Folder trust

The first time Claude Code’s interface launches in a folder, it asks whether you trust it. Escouade never answers for you. Worktrees created in the project’s default worktrees folder (`.claude/worktrees`) inherit the answer given for the project. A “Worktrees folder” outside the project doesn’t inherit it: the interface asks the question the first time each worktree is launched (see [worktrees](worktrees)).

## With Codex

An agent on Codex has the same view: the button is called “Codex view” and starts Codex’s interface (`codex`) in the agent’s folder, on its account, with its model and effort. Both views share the same Codex conversation: the terminal resumes the session (`codex resume`), and Codex’s app-server resumes it back in the Escouade view, in the same Codex folder. What differs from Claude Code:

- **Tracking.** Codex runs its hooks as commands: Escouade gives it `escouade-hook`, installed with the app. These hooks are declared for this interface alone, through the options of its command line (`-c hooks.…`), and so is their approval: nothing is written to your `config.toml`, and your own Codex sessions don’t see them. Without the token Escouade gives the interface, `escouade-hook` does nothing anyway. The agent’s card says when it works, when it waits for your permission and when it’s done; its conversation comes from the session’s log (`sessions/…/rollout-….jsonl` in Codex’s folder), read continuously.
- **Folder trust.** On a project Codex doesn’t know, its interface first asks whether you trust the folder. Escouade never answers for you, and types no message while the question is asked: “Codex asks whether you trust this folder: answer in its terminal, then send your message again.” Codex’s app-server, for its part, records that trust in your `config.toml` by itself the first time a Codex agent works in the Escouade view: a project where a Codex agent already ran no longer asks.
- **Modes.** “Auto” starts the interface with `--approve-for-me` (Codex’s automatic reviewer), “Bypass” with no requests and no sandbox, the others with `-a on-request` and the mode’s sandbox. Codex’s Plan mode is picked in its interface.
- **The agent’s added instructions** are only given to a new session: a resumed session keeps the ones it had.
- **Interrupting** sends `Esc`, as with Claude Code. An image pasted in the terminal (PNG or JPEG) is typed there by its path.
- **Not tracked in this view**: the “Plan” banner, subagents and cost (Codex gives none).

## With OpenCode

An agent on OpenCode has its view: the button is called “OpenCode view” and opens OpenCode’s interface (`opencode attach`) in a terminal, plugged into the agent’s session, from its conversation’s header or the right-click on its card, even mid-turn. Unlike the other views, it doesn’t replace the agent’s process: it adds to it.

- **Tracking.** Escouade keeps the thread: the conversation, the card’s state, the notifications and the cost stay up to date while you work in the interface, and a message typed there shows up with its whole turn. A permission answered in the interface closes in Escouade, and the other way round.
- **The password** of OpenCode’s server goes through the interface’s environment, never through its command line.
- **Going back to the Escouade view**, or closing the terminal, only stops the interface: OpenCode’s server goes on for the other agents.
- **An interface whose agent has no process anymore** (stopped, restarted on another server) stops, and “Restart” plugs it back in.

## With Copilot CLI

An agent on Copilot CLI has the same view: the button is called “Copilot CLI view” and starts Copilot’s interface (`copilot`) in the agent’s folder, on its account, with its model and effort. Both views share the same Copilot session: the terminal resumes it (`copilot --resume`), and Copilot’s server resumes it back in the Escouade view, in the same Copilot folder (`COPILOT_HOME`). What differs from Claude Code:

- **Tracking.** Copilot runs its hooks as commands: Escouade gives it `escouade-hook`, installed with the app, in a plugin of the agent’s own, given to this interface alone on its command line (`--plugin-dir`). Nothing is written to your Copilot folder (neither `hooks/` nor `settings.json`), and your own Copilot sessions don’t see them. Without the token Escouade gives the interface, `escouade-hook` does nothing anyway, even with your `GH_TOKEN` in its environment. The agent’s card says when it works, when it waits for your permission, your answer to a question or your verdict on a plan, and when it’s done; its conversation and its cost come from the session’s log (`session-state/<session>/events.jsonl` in Copilot’s folder), read continuously.
- **Folder trust.** On a folder Copilot doesn’t know, its interface asks at every start whether you trust the folder. Escouade never answers for you, and types no message until it knows the question is past: “Copilot asks whether you trust this folder: answer in its terminal, then send your message again.” “Yes, and remember this folder” spares you the question at the next starts; with “Yes” alone, nothing is recorded, and Escouade waits until you’ve typed a first message in the terminal yourself.
- **Modes.** “Bypass” starts the interface with `--allow-all`, “Plan” in plan mode, “Auto edits” with writes allowed (`--allow-tool=write`), “Auto” in autopilot with writes, `git add` and `git commit` allowed, as Escouade does in its view.
- **The agent’s added instructions** aren’t given to the interface: Copilot has no option for them, and only gets them back in the Escouade view.
- **A copy** (“Duplicate conversation”) is made by Copilot before the interface starts, then resumed by it.
- **Interrupting** sends `Esc` twice, as Copilot asks (“esc again to interrupt”).
- **Not tracked in this view**: the “Plan” banner (Copilot’s tasks).

## With Cursor CLI

An agent on Cursor CLI has its view too: the button is called “Cursor CLI view” and starts Cursor’s interface (`agent`) in the agent’s folder, with its model and mode, in the agent’s own configuration folder: its “For this agent” rules hold in both views. What differs from Claude Code:

- **Two conversations, linked.** Cursor keeps the conversations of its interface apart from those Escouade drives: the interface doesn’t pick up the history of the agent’s session, nor the other way round. Escouade links them: when the view opens, it writes what was said so far in a rule the interface reads (`.cursor/rules/escouade-relais.mdc` in the agent’s folder, kept out of git and taken away on the way back); back in the Escouade view, what was said in the interface goes with your next message. The interface keeps its own conversation from one opening to the next (`agent --resume`).
- **A limited following.** Cursor doesn’t tell Escouade through reliable hooks: Escouade reads the interface’s log (`~/.cursor/projects/<folder>/agent-transcripts/`). The agent’s card says when it works and when it’s done, with the end-of-turn notification, and its conversation shows your messages, its replies and its tools. Escouade doesn’t know when Cursor waits for your permission or your answer, nor the results of its commands and edits, nor the tokens: the “Limited following” band above the terminal says so. Watch the terminal.
- **Folder trust.** On a folder it doesn’t know, Cursor first asks whether you trust it (“Trust this workspace”). Escouade never answers for you, and types no message while the question is asked.
- **Modes.** “Plan” starts the interface in plan mode, “Bypass” in “Run Everything” (`--force`); “Ask” and “Accept edits” leave Cursor asking for its commands.
- **Cursor’s shell** stays PowerShell on Windows, even when Escouade was started from Git Bash.
- **Interrupting** sends `Ctrl+C`, as in the interface.
- **Not tracked in this view**: the “Plan” banner, subagents, permissions and questions waiting, tool results, tokens.

## With Mistral Vibe

An agent on Mistral Vibe has its view too: the button is called “Mistral Vibe view” and starts Vibe’s interface (`vibe`) in the agent’s folder, with its model and mode, in the agent’s `VIBE_HOME`. Both views share the same session: the interface resumes it (`vibe --resume`) with all its history, and its `vibe-acp` resumes it back in the Escouade view, with nothing lost and nothing to relay. What differs from Claude Code:

- **Following.** Escouade reads the session’s journal, where Vibe keeps it (`logs/session/unified/<session>/journal/` of the agent’s `VIBE_HOME`), from one segment to the next: your messages, its replies, its tools and their end, the permission or question it waits for, and the end of each turn. The agent’s card says when it works, when it waits for your answer in the terminal, and when it’s done. Its hooks (`pre_tool`, `post_tool`, `post_agent`) run `escouade-hook`, installed with the app, which tells Escouade the session the interface works in; they’re written in the agent’s `VIBE_HOME` when the view opens and taken away on the way back, never in your `~/.vibe`.
- **A session not written yet.** Vibe keeps a session only after a first message: an agent that said nothing yet isn’t resumed, the interface opens a new session, and its hooks give it to the agent.
- **Escouade’s MCP server** goes through the interface’s variables (`VIBE_MCP_SERVERS`, and the token in another variable), never on its command line nor in a file.
- **Folder trust.** Escouade writes it in the agent’s `VIBE_HOME`: Vibe doesn’t ask.
- **Modes.** “Ask” starts its `ask` agent, “Accept edits” `accept-edits`, “Bypass” `auto-approve`; “Plan” asks, as “Ask” does, Vibe having no plan mode. For a private project, the other projects’ paths stay refused to its file tools, and “Bypass” stays in `ask`.
- **Interrupting** sends `Esc`, as in the interface.
- **Not tracked in this view**: the “Plan” banner, cost and tokens, thinking (set by `/thinking` in the interface).

## With Antigravity CLI

An agent on Antigravity CLI has its view too: the button is called “Antigravity CLI view” and starts Antigravity’s interface (`agy`) in the agent’s folder, with its model, effort and mode, in the agent’s own home, where its conversation is kept. Both views share the same conversation: the interface resumes it (`agy --conversation`) with all its history, and `agy -p` resumes it back in the Escouade view, with nothing lost and nothing to relay. What differs from Claude Code:

- **Following.** Escouade reads the conversation’s transcript (`brain/<conversation>/.system_generated/logs/transcript_full.jsonl` under the agent’s `.gemini`), a line per step: your messages, its replies, its tools and their end, the question it asks you, and the end of each turn. The agent’s card says when it works, when it waits for your answer to its question, and when it’s done. Its hooks (`SessionStart`, `PreToolUse`, `PostToolUse`, `Stop`) run `escouade-hook`, installed with the app; they’re written in the agent’s folder (`.gemini/config/hooks.json`) when the view opens and taken away on the way back, never in your `~/.gemini`.
- **A conversation not written yet.** Antigravity keeps a conversation’s transcript only after a first message: an agent that said nothing yet isn’t resumed, the interface opens a new conversation, and its first hook gives it to the agent.
- **Escouade’s MCP server** is in the MCP config of the agent’s folder, its token written in that file (Antigravity expands no variable there), never on its command line.
- **Its first screens.** Escouade writes the color choice and the folder’s trust in the agent’s folder: Antigravity doesn’t ask. Your Google account’s address isn’t shown in its header.
- **Modes.** “Ask” is its default mode, “Accept edits” `accept-edits`, “Plan” `plan`, “Bypass” its rules that allow everything; a private project’s paths stay refused to its file tools.
- **Interrupting** sends `Esc`, as in the interface: what it wrote so far ends the turn.
- **Not tracked in this view**: a permission it asks you in the terminal (neither a hook nor the transcript tells it: the card stays “working” until you answer), the “Plan” banner, cost and tokens, thinking.

## With Gemini CLI

An agent on Gemini CLI has its view too: the button is called “Gemini CLI view” and runs Gemini’s interface (`gemini`) in the agent’s folder, with its model and mode, in the agent’s configuration folder (`GEMINI_CLI_HOME`), the one its session is kept in. Both views share the same session: the interface resumes it (`gemini --resume`) with all its history, and `gemini --acp` resumes it back in Escouade’s view. A session that has written nothing yet starts under the id Escouade gives it (`--session-id`). What changes from Claude Code:

- **The following, by its hooks.** Its log writes a message anew at each change: Escouade doesn’t read it. Its hooks run `escouade-hook`, installed with the app: `BeforeAgent` (your message), `BeforeTool` and `AfterTool` (each tool and its end), `Notification` (a permission it asks you in the terminal: the card waits for your answer), `AfterAgent` (the end of the turn, with its whole answer). They are written in the settings of the agent’s folder when the view opens and taken out on the way back, never in your `~/.gemini`.
- **Escouade’s MCP server** is in the settings of the agent’s folder (`mcpServers`), its token written in that file, never on its command line.
- **The modes.** “Ask” is its `default` mode, “Auto edits” `auto_edit`, “Plan” `plan`, “Bypass” `yolo`; a private project’s paths stay refused by its rules (`--policy`).
- **Interrupt** sends `Escape`, as in the interface; no hook tells that turn stopped, Escouade ends it itself.
- **What isn’t followed in this view**: its texts between two tools (its whole answer alone comes, at the end of the turn), a usage limit (its interface itself offers to wait or to change models), the “Plan” banner, the cost and tokens.

## On Windows with WSL

When Claude Code is installed in WSL, Escouade launches it through `wsl.exe` (see [the harness settings](settings-claude-code)). Live tracking of the Claude Code view then requires WSL’s mirrored networking: without it, the interface’s hooks don’t reach the app. The settings point it out under the `claude` path (“WSL’s network is NAT…”). To switch to mirrored mode, add this to `%UserProfile%\.wslconfig`, then run `wsl --shutdown`:

```ini
[wsl2]
networkingMode=mirrored
```
