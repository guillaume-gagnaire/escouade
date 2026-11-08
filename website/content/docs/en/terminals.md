# Terminals

Escouade has real built-in terminals, one per shell you open, listed by project in the sidebar. This page explains how to open them, which shells are offered, how to copy, paste and search, and which shortcuts stay with the shell.

## Opening a terminal

A terminal is a real shell, in a pseudo-terminal (ConPTY on Windows, pty on macOS and Linux) displayed by xterm.js. Its history, its prompt and its completion are therefore the shell’s own: the `Tab` key completes as in any terminal. `http://` and `https://` addresses that appear in the output open in your browser with a plain click, no key to hold. The terminal keeps 10,000 lines of history.

| To open… | Do this |
|---|---|
| A terminal in the project folder | `Ctrl+T` (`⌘T` on macOS), or the + button of the sidebar’s “Terminals” section. The shortcut uses the first shell in the list; the + button offers each shell. |
| A terminal in an agent’s worktree | “Open a terminal” in the “…” menu of its conversation’s header; or the + button of the “Terminals” section, then “In the worktree of…” and the agent’s name; or a right-click on the agent’s card, then “Open a terminal”. The button and the + menu only concern agents that have a worktree and aren’t archived (“In the worktree of…” doesn’t appear when there is none); without a worktree, the right-click opens the terminal in the project folder. See [worktrees](worktrees). |
| A terminal in a folder of the editor | Right-click a folder in the [editor](editor)’s file tree, then “Open a terminal here”. On the free space, it is the source’s root (the agent’s worktree or the project). |

A terminal is named after its shell and a number (`pwsh-1`), or after its shell and the agent or folder it comes from (`pwsh · refacto-auth`, `pwsh · src`). It is added to the project’s “Terminals” section, along with the number of open terminals: clicking its row displays it, the row’s × closes it. When the shell has stopped, the row carries “exited”.

Without a shell found, a message says so: “No shell found (PowerShell 7, Git Bash, WSL). Check the settings.”

## The available shells

Escouade detects the shells installed on your machine.

| System | Shells offered |
|---|---|
| Windows | “PowerShell” (PowerShell 7, looked for in the `PATH` then in `Program Files`); “Windows PowerShell” instead when PowerShell 7 is missing; “Git Bash”, the one from Git for Windows; “WSL” (“WSL (Ubuntu)” when a distribution is set). |
| macOS and Linux | Your login shell (`$SHELL`) first, then zsh, bash and fish when they are installed. `sh` is the last resort when none of the others is found. |

The first one in the list is what `Ctrl+T` opens. On macOS and Linux, shells start as login shells (`-l`), as in an ordinary terminal. WSL opens in the project folder (or the agent’s) and takes the default distribution, or the one in Settings › Advanced › Terminals.

The paths of PowerShell 7 and Git Bash, the WSL distribution and, on macOS, the path of bash are set in Settings › Advanced › Terminals, which also shows the detected shells (on Linux, only the “Git Bash” field counts, for bash): see [the terminals settings](settings-terminals).

## In the terminal

The terminal fills the whole main area. Its header gives its name, its shell and the project folder, and it has three buttons: the magnifier (see [below](#searching-clearing-closing)), “Clear”, which empties the screen, and “Close terminal”. Closing the terminal stops the shell. On Windows, it also stops the programs the shell started, except those that opened a window (a browser, an editor), which stay open.

The terminal keeps its contents when you display something else, for example a conversation, and then come back.

When the shell stops (an `exit`, for example), a bar says “Process exited.” or “Process exited (code 1).”, with “Restart”, which closes this terminal and opens a new one with the same shell, in the project folder, and “Close”.

### Copying and pasting

| System | Copy | Paste |
|---|---|---|
| Windows and Linux | Select with the mouse, then `Ctrl+C` (or `Ctrl+Shift+C`). Without a selection, `Ctrl+C` interrupts the program as usual. | `Ctrl+V` |
| macOS | `⌘C` on the selection | `⌘V` |

On macOS, the `Ctrl` keys all go to the shell (except `Ctrl+Tab`).

### Escouade’s shortcuts

A terminal keeps almost all keys for the shell. The behavior depends on the system.

- On Windows and Linux, these shortcuts go through to Escouade: `Ctrl+Tab` and `Ctrl+Shift+Tab` (next and previous agent), `Ctrl+Shift+A` (the [overview](overview)), `Ctrl+1` to `Ctrl+9` (go to project n), `Ctrl+,` (the settings) and `Ctrl+J` (the next waiting agent). `Ctrl+Shift+B` (the files panel) and `Ctrl+Shift+L` (the layout) work too: the terminal sends nothing to the shell for `Ctrl+Shift` plus a letter. The other `Ctrl` plus a letter keys, including `Ctrl+K`, `Ctrl+P`, `Ctrl+N` and `Ctrl+T`, goes to the shell: a `Ctrl+T` typed in a terminal therefore doesn’t open a new terminal.
- On macOS, all of Escouade’s `⌘` shortcuts work in a terminal, and `⌘C`, `⌘V` and `⌘A` (select all) are the terminal’s own. `Ctrl+Tab` goes to the next agent.

### Searching, clearing, closing

- The header’s magnifier, or `Ctrl+Shift+F` (`⇧⌘F` on macOS) when the terminal has the focus, opens a “Search…” field: `Enter` goes to the next match, `Shift+Enter` to the previous one and `Esc` closes the field and gives the focus back to the terminal.
- “Clear” empties the terminal’s screen.
- “Close terminal” closes it.

## The proxy

In Settings › Advanced › Network, the “Also apply the proxy to the built-in terminals” switch (off by default) gives terminals the `HTTPS_PROXY`, `HTTP_PROXY` variables (and their lowercase forms) with the proxy’s address, and `NO_PROXY` with the exclusions. It applies to terminals opened after the change, and also to launch commands and to worktrees’ setup and teardown commands. See [the network settings](settings-network).

## Escouade’s other terminals

Escouade uses the same terminal elsewhere: each launch command runs in its own, read-only (see [running the project](launch)), and an agent’s Claude Code view is Claude Code itself in a terminal (see [the Claude Code view](claude-code-view)), and the window that signs in a Claude account shows the account’s `claude` in a terminal (see [Claude accounts](accounts)).
