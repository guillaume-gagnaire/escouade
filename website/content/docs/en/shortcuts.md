# Keyboard shortcuts

Every Escouade shortcut, grouped by use: navigation, conversation, questions and permissions, editor, terminals, panels, tickets, and settings. Each one says where it works; the page also covers what changes on macOS and what stays with the shell inside a terminal.

## Reading this page

Shortcuts are written the way the app writes them in English: `Ctrl+Shift+A`, `Ctrl+Enter`, `Esc`. The “Where” column says where each one works:

- “Everywhere”: anywhere in the window, as long as no dialog (settings, commit, ticket…) is open. On Windows and Linux, though, a terminal that has focus keeps some of these keys for its shell: see [Terminals](#terminals).
- “Everywhere, terminals included”: even when a terminal has focus.
- A specific place (the message field, the editor, a form…): the shortcut only works there.

`Esc` closes the open dialog. Two notes on the keyboard:

- Global shortcuts ignore any combination with `Alt`, because `Ctrl+Alt` is `AltGr` on a French keyboard, except `Ctrl+Alt` and an arrow, which types nothing, to go from pane to pane of the [split screen](split-screen). The shortcuts that use `Alt` belong to one area: `Alt+1` … `Alt+9` (questions), `Alt+←` and `Alt+→` (editor), `Alt+↑` and `Alt+↓` (worktree settings), `Alt+click` (editor).
- The digits of `Ctrl+1` … `Ctrl+9` and `Alt+1` … `Alt+9` are read from the physical key: on an AZERTY keyboard, you don’t need `Shift`.

## On macOS

`Ctrl` becomes `⌘`, and `Ctrl+Shift` becomes `⇧⌘`: `Ctrl+N` is written `⌘N`, `Ctrl+Shift+A` is written `⇧⌘A`. The exceptions:

| Windows and Linux | macOS |
|---|---|
| `Ctrl+Tab`, `Ctrl+Shift+Tab` | Unchanged: `⌘Tab` switches applications. |
| `Alt+1` … `Alt+9`, `Alt+↑`, `Alt+↓`, `Alt+click` | Unchanged: `Alt` is the Option key (`⌥`). |
| `Alt+←`, `Alt+→` (editor) | `⌃-`, `⌃⇧-`, with the Control key rather than `⌘`. |
| `Ctrl+click` (editor) | `⌘+click`. |
| `Ctrl+Y` (editor, redo) | `⇧⌘Z`. |
| `Del` (editor file tree) | `⌘⌫`; the `Del` key works too. |
| `Ctrl+C`, `Ctrl+V` (terminal) | `⌘C`, `⌘V`. The `Ctrl` keys go to the shell. |

## Navigation

| Shortcut | Action | Where |
|---|---|---|
| `Ctrl+1` … `Ctrl+9` | Go to project n, in the order of the tabs. | Everywhere, terminals included |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous agent of the project on screen. | Everywhere, terminals included |
| `Ctrl+Shift+A` | Open or close the [overview](overview), which gathers the agents of all projects. | Everywhere, terminals included |
| `Ctrl+K` | Search the conversations: what you and your agents wrote, commands, files, and tool outputs, ignoring case and accents. A result opens the agent on the message. | Everywhere |
| `Ctrl+P` | Open a file by its name. `name:42` opens it at line 42, `name:42:7` at line 42, column 7. The file is looked up in the editor’s source if it is open, otherwise in the worktree of the agent on screen, otherwise in the project folder. | Everywhere |

## Palettes and lists

| Where | Keys |
|---|---|
| `Ctrl+K` and `Ctrl+P` | `↑` / `↓` choose, `Enter` opens, `Esc` closes. With nothing typed, `Ctrl+P` offers the files opened last: `Enter` reopens the one before the file on screen. |
| Overview | `↑` / `↓`, `Home` and `End` change line. `Enter` or `Space` opens the agent on the line. `Esc` goes back to the view you came from. |
| Branch picker (status bar) | `↑` / `↓` choose, `Enter` confirms the chosen line, `Esc` closes. |
| Branch in the picker, commit in the git graph | `Menu` or `Shift+F10` opens the menu that a right click opens. |
| Context menu | `↑` / `↓`, `Home` and `End` choose, `Esc` closes. |
| Settings tabs | `↑` / `↓`, `Home` and `End` change tab. |
| Kanban ticket card | `Enter` or `Space` opens it: the ticket form if it is in “To do”, otherwise its agent. |

## Agents and conversation

| Shortcut | Action | Where |
|---|---|---|
| `Ctrl+N` | New agent in the project on screen. | Everywhere |
| `Ctrl+J` | Put the focus on the card of the next agent that is waiting for your answer or needs a look (a question, or an end or error you haven’t seen yet), across all projects, without leaving the conversation on screen; `Enter` opens it. | Everywhere, terminals included |
| `1`, `2`… | Answer from the list of agents: the answer or decision of that number, on the card that has the focus, else on the first one waiting. | Outside a text field, a terminal and the editor |
| `Enter` | Send the message. While Claude is working, it takes the message into account at its next step. | Message field |
| `Shift+Enter` | New line. | Message field |
| `Esc` | Interrupt Claude. | Message field, while the agent is working |
| `↑` | Recall the last message sent, to send it again as is or edited (the original stays in the conversation). | Empty message field |
| `Ctrl+Shift+Space` | [Dictate](chat#dictating-a-message): a short press starts and another ends, held down it dictates while it is pressed; the text goes at the cursor, not sent. Changed in [Settings › Voice](settings-voice#use). | The message field, a ticket’s description, the terminal of the Claude Code view |
| `Esc` | Cancel the dictation under way, writing nothing. | During a dictation |
| `Esc` | Leave the [voice conversation](chat#talking-with-the-agent-voice-conversation): the microphone closes, as with “Stop”. | During a voice conversation |
| `↑` / `↓`, `Enter` or `Tab`, `Esc` | Choose, accept, or close an `@file` or `/command` suggestion. | Message field, list open |

## Questions and permissions

When Claude asks a question or requests a permission, you can handle the card from the keyboard:

| Shortcut | Action |
|---|---|
| `Ctrl+Enter` | “Allow” the pending request (“Approve the plan” for a plan). For a question, submits the answer once it is complete (“Submit”). |
| `Ctrl+Shift+Enter` | “Always allow” (“Approve and accept edits” for a plan), only if the card offers that button. |
| `Alt+1` … `Alt+9` | Pick option n of the pending question; for a multiple choice, tick or untick it. |
| Text in the message field, then `Enter` | Deny the request with that text as the message, or answer the question freely. |

These keys work in the conversation, message field included, or when nothing else has focus. They don’t work in a terminal, in the editor, or while a dialog is open. The details:

- Several requests can be waiting: the keyboard answers the first one.
- A request that has just arrived ignores these keys for half a second, `Enter` in the message field included: the text you have already typed stays there. So a `Ctrl+Enter` meant to send your message doesn’t answer a request that arrives at the same moment, and a second `Ctrl+Enter` doesn’t answer the next request. A key held down answers only once.
- A request that Claude Code would deny by default can only be allowed with its button.
- A single-choice question is sent as soon as you pick an option. With several questions or a multiple choice, `Ctrl+Enter` moves on to the next question that has no answer, then submits once all of them have one.
- In the [overview](overview), `Ctrl+Enter` does nothing: a permission that can be read in full there is allowed or denied with its buttons.

## Editor

The built-in [editor](editor). Its shortcuts work as long as it is open.

### The text

| Shortcut | Action | Where |
|---|---|---|
| `Ctrl+S` | Save the file. | Editor open |
| `Ctrl+Shift+F` | Search all the files of the source. | Editor open |
| `Ctrl+F` | Search in the file. `F3` or `Ctrl+G` goes to the next match, with `Shift` to the previous one. | File text |
| `Ctrl+click` or `F12` | Go to what the word points to: an imported file, a link, a path, the definition of a name. If there are several definitions, a list opens. | File text |
| `Shift+F12` | Find the references of the name under the cursor. | File text |
| `Alt+←` / `Alt+→` | Go back to where you were before a jump / go forward again. The mouse’s “back” and “forward” buttons do the same. | File text |
| `Alt+click` | Add a cursor. | File text |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo. | File text |
| `Tab` / `Shift+Tab` | Indent / unindent. | File text |

The text is a CodeMirror editor: its other usual shortcuts work there too.

### The file tree

These keys apply when a row of the file tree has focus.

| Shortcut | Action |
|---|---|
| `↑` / `↓`, `Home`, `End` | Change row. |
| `→` | Expand the folder, then go to its first item. |
| `←` | Collapse the folder, or go to the parent folder. |
| `Enter` or `Space` | Open the file, or expand the folder. |
| `F2` | Rename. |
| `Del` | Delete to the trash, after confirmation. |

In the results of “Search in files”, `Enter` in the field runs the search, `↓` moves to the results, and `↑` from the first one goes back to the field.

## Terminals

| Shortcut | Action | Where |
|---|---|---|
| `Ctrl+T` | New [terminal](terminals) in the project on screen. | Everywhere |
| `Ctrl+Shift+F` | Search the output. `Enter` goes to the next match, `Shift+Enter` to the previous one, `Esc` closes the search. | Terminal, log of a launch command |
| `Ctrl+C` | Copy the selection; with no selection, the key goes to the shell (interrupt). | Terminal, on Windows and Linux |
| `Ctrl+V` | Paste. | Terminal, on Windows and Linux |
| `Ctrl+Shift+C` | Copy the selection. | Terminal, on Windows and Linux |
| `⌘C` / `⌘V` | Copy / paste. | Terminal, on macOS |
| `Ctrl+V` | Goes to the shell; in the [Claude Code view](claude-code-view), it is Claude Code’s key to read the clipboard’s image. | Terminal, on macOS |
| `Shift+PageUp`, `Shift+PageDown`, `Shift+Home`, `Shift+End` | Scroll the log. | Log of a launch command (read-only) |

The log of a launch command is read-only: apart from copying and scrolling, all its keys go to the app, `Ctrl+K` and `Ctrl+P` included.

### What stays with the shell

On Windows and Linux, a terminal that has focus keeps `Ctrl` plus a letter for its shell, including `Ctrl+K` (cut to the end of the line), `Ctrl+P` (previous command), `Ctrl+N`, and `Ctrl+T`: to search the conversations, open a file, or create an agent or a terminal, click outside the terminal first. The app only takes `Ctrl+Tab` and `Ctrl+Shift+Tab`, `Ctrl+1` … `Ctrl+9`, `Ctrl+J` (to a shell it is only a line feed, which `Enter` already sends), `Ctrl+,`, `Ctrl+Shift+A`, and the zoom (`Ctrl+=`, `Ctrl+-`, `Ctrl+0`). `Ctrl+Shift` plus a letter sends nothing to the shell: `Ctrl+Shift+B` and `Ctrl+Shift+L` work in a terminal too, where `Ctrl+Shift+F` searches its output.

On macOS, the app’s shortcuts use `⌘`, which the shell never receives: they all work in a terminal, except `⌘C`, `⌘V`, and `⌘A` (select all), which are the terminal’s own. The `Ctrl` keys go to the shell.

## Panels and layout

| Shortcut | Action | Where |
|---|---|---|
| `Ctrl+Shift+L` | Switch between the classic layout and “Conversation and files side by side”. | Everywhere, terminals included |
| `Ctrl+Shift+B` | Show or hide the panel of uncommitted files. | Everywhere, terminals included, in the classic layout (side by side, the panel is always shown) |
| `Ctrl+\` | [Split screen](split-screen): split what is shown with the next agent, or go back to one pane. On a French AZERTY keyboard: `Ctrl+*`. | Everywhere, terminals included |
| `Ctrl+Shift+\` | Turn the active pane’s split. | Split screen, terminals included |
| `Ctrl+Alt+←` `→` `↑` `↓` | Go to the pane on that side. | Split screen, terminals included |
| `Ctrl+Shift+W` | Close the active pane. | Split screen, terminals included |
| `Ctrl+click` | On an agent’s card, a terminal’s or a launch’s row: open it beside. | Sidebar |

A separator that has focus (with `Tab`) can be adjusted from the keyboard:

- The editor’s files column: `←` / `→` narrow or widen it, `Home` and `End` go to the minimum and maximum width.
- The task list of the “Plan” banner: `↑` / `↓` change its height, `Home` and `End` go to the minimum and maximum, `Enter` or `Esc` restore the usual height.

A double click on a separator restores its size too.

## Tickets

| Shortcut | Action | Where |
|---|---|---|
| `Ctrl+Enter` | Add or save the ticket, from any field. | Ticket window |
| `Enter` | Add or save the ticket. In multi-line fields, `Enter` starts a new line. | Ticket title |
| `Esc` | Close the window; if you have typed something, the app first asks whether to discard it. | Ticket window |

## Settings

| Shortcut | Action | Where |
|---|---|---|
| `Ctrl+,` | Open the settings. | Everywhere, terminals included |
| `Ctrl+=` (or `Ctrl++`) / `Ctrl+-` | Zoom the whole interface in / out one step (see [“Appearance”](settings-application#appearance)). | Everywhere, terminals included, dialog open or not |
| `Ctrl+0` | Go back to a 100% zoom. | Everywhere, terminals included, dialog open or not |
| `Esc` | Close the settings. | Settings open |
| `Alt+↑` / `Alt+↓` | Move a setup or teardown command up / down, from any field of the command. | Project settings, worktree command lists |
