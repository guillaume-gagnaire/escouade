# Use

## Start an agent

Claude Code is a project’s default harness until you choose another (Settings › Projects › Project, “Default harness”). “New agent” starts it; once two harnesses are installed, the arrow beside “New agent” opens the “Harness” menu.

The agent starts on the first active Claude Code account that still has quota. It keeps its conversation from one restart to the next, and “Duplicate the conversation” copies it under a new session.

The models are those each account’s `claude` lists: before the first reading, Fable, Opus, Sonnet and Haiku. Effort goes from Low to Max, except for Haiku, which has none. Model, effort and mode change while the agent runs.

A message sent during a turn waits for the turn’s end. Attached images and PDFs go through, and so do Claude Code’s `/` commands. Claude Code is the only harness with remote control and background tasks.

## The modes

Escouade gives Claude Code its mode (`--permission-mode`), and Claude Code’s permissions decide. What Claude Code asks comes to Escouade as a card; what it doesn’t ask, it does.

| Mode | What Claude Code does without a card |
|---|---|
| Ask (`default`) | It reads in the agent’s folder without asking. Writes and commands ask, unless your Claude Code rules allow them. |
| Accept edits (`acceptEdits`) | It also writes the files of the agent’s folder without asking. Commands ask. |
| Plan (`plan`) | It studies without changing anything, then puts its plan on a card: “Approve the plan”, “Approve and accept edits” or “Keep planning”. Once approved, work resumes in the mode the card says. |
| Auto (`auto`) | Claude Code’s classifier accepts what it deems safe and asks for the rest. |
| Bypass (`bypassPermissions`) | It asks nothing. Its deny rules still hold. |

Your rules count: those of the account’s `settings.json` and the project’s (`.claude/settings.json`, `.claude/settings.local.json`) are read by Claude Code, as in a terminal.

On top of that, in every mode:

- a file you attached by its path is read without a card;
- the folders of the files attached to the agent and to tickets are given to it for reading (`--add-dir`).

A permission card has “Allow”, “Deny” and, when Claude Code suggests a rule, “Always allow”: the rule is then the one Claude Code suggested.

## The Claude Code view

The Claude Code view runs the agent in Claude Code’s own interface, in a terminal of the window, on the same conversation: Escouade stops one and resumes the session in the other. Its permissions are answered in that interface. A ticket’s agent stays in Escouade’s view.

## A discussion of the Docs tab

A discussion changes nothing: Claude Code runs there without its tools that write or run a command (`Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Bash`, `PowerShell`, `Monitor`, `EnterWorktree`), whatever the mode.

## Private projects

For a private project, Escouade writes deny rules (`Read` and `Edit`) in a settings file of the agent’s own: the paths of the other projects, and its own for the agents of the other projects. Claude Code applies them to its file tools and to the commands whose paths it reads (`cat`, `sed`, `Get-Content`…), even in Bypass.

Where Claude Code’s sandbox runs (macOS, Linux, WSL2), the same paths are refused to it too (`sandbox.filesystem`): once that sandbox is on, they hold for all its commands.

Its limits:

- on Windows without WSL, no sandbox: a script that opens a file by itself gets through;
- Claude Code’s sessions, memory and history are the account’s: so that a private project doesn’t share them, give it an account reserved to it alone.
