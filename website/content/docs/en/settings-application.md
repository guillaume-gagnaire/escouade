# Settings: the window and General

This page describes the settings window (its bar, its search, its amber dots, how it saves as you go) and its “General” page, which sets the appearance (the theme, the interface zoom, the text size of the terminals and of the editor) and picks the language of the interface and of the texts Claude writes.

## The settings window

Escouade’s settings live in a single window: a bar on the left, one page at a time on the right. Open it with `Ctrl+,` (`⌘,` on macOS) or with the “Settings” button at the bottom of the [projects rail](interface). It opens on “Accounts and harnesses”, unless a button opens it right on the page it is about: the ⚙ button in the [Kanban](board) header opens its project’s “Kanban” page, a project’s menu its “Project” page, “Configure” in the Launch section its “Launch” page. The shortcut does nothing while another window is open.

### The bar

From top to bottom: the “Search settings” field, Escouade’s pages, each project, “All projects”, then “About” with Escouade’s version. The title and description of the page you are on sit at the top of the right-hand side. When the focus is on the bar, the up and down arrows go from one entry to the next and open it, `Home` and `End` go to the first and the last.

| Page | What it sets |
|---|---|
| “General” | Appearance, languages, updates, your name and usage statistics |
| [“Agents”](settings-claude-code) | New agents, written texts, remote control, the MCP server |
| [“Accounts and harnesses”](settings-accounts) | Each harness and its accounts, and what happens when a quota runs out |
| [“Connected services”](settings-integrations) | Code hosts and the accounts of the ticket sources |
| [“Voice”](settings-voice) | Dictation and voice conversation |
| [“Notifications”](settings-notifications) | Visual and sound alerts |
| “Plugins” | The plugins installed, the ones Escouade proposes, security |
| “Advanced” | [Network](settings-network), [terminals](settings-terminals), attachments, search by meaning, local data |
| [“About”](settings-about) | Version, credits and links |

Under “Projects”, each project has its color before its name, and a padlock if it is private. The project chosen unfolds on its seven pages: [“Project”](settings-projects), “Git and worktrees”, “Launch”, [“Kanban”](settings-board), [“Imported tickets”](settings-integrations), “Team” and “Privacy”. Going from one project to another keeps the same page. On a page of a project, the window takes that project’s color; on a page of Escouade, Escouade’s teal. With no project open, a project’s page says “No project open.”.

At the foot of a page, “Advanced” folds the settings you rarely touch into a line that names them; a click unfolds them in place.

### Searching for a setting

The “Search settings” field, at the head of the bar, finds a setting by its name or its description, and a page by its name; `Ctrl+F` (`⌘F` on macOS) goes to it from anywhere in the window. It searches in English and in French whatever the language of the interface, ignoring case and accents: “bêta” finds “Receive beta versions”, “proxy” finds “HTTP(S) proxy”.

While the field holds text, its results take the place of the bar’s entries, each with the page it is on: “Advanced”, “General › Advanced”, “demo-api › Git and worktrees”. A project’s setting comes once per project, the project chosen first. The up and down arrows go from one result to the next; `Enter` or a click opens its page, unfolds “Advanced” if it is there, and lights the setting for a moment. `Esc` empties the field; a second `Esc` closes the window.

### The amber dots

An amber dot at the end of an entry of the bar says a page is waiting on you; its tooltip says what for:

- “Accounts and harnesses”: a harness installed with no account (“Codex has no account: no agent or ticket starts on it”);
- “Connected services”: a code host’s token expired or revoked, or a token the system keychain can’t read;
- “Plugins”: a plugin disabled by Escouade, with an invalid manifest, or stopped after its crashes;
- a project: no active account of its harness serves it. The dot is on the project and on its “Project” page, where its account is chosen.

### Everything saves as you go

There is no “Cancel” and no “Save”: as in the settings of macOS or VS Code, a choice (a switch, a menu, a command added) applies and saves at once, a text when you leave its field or after a short pause in the typing. Closing the window (the cross, `Esc`, or a click on the backdrop) saves what was still waiting.

An entry that can’t be saved says so on its row, and the value from before stays until it is fixed: a project without a name, a launch command without a name or a command line, a worktrees folder or a branch prefix refused, a number emptied.

## The “General” page

The page has four groups, “Appearance”, “Language”, “Updates” and “You” (see [About](settings-about) for the last two), then “Advanced”: the editor’s text size and your identifier.

### Appearance

These four settings apply right away, with no restart, and stay from one launch to the next. The editor’s text size is in “Advanced”.

| Setting | Effect | Default |
|---|---|---|
| “Theme” | “System”, “Light”, or “Dark”. “System” follows the Windows, macOS, or Linux theme, and its changes, as they happen. The title bar and the native controls (scroll bars, menus) follow too. | “System” |
| “Interface zoom” | Makes the whole window bigger or smaller, like a browser’s zoom: text, spacing, and icons together, terminals and editor included. Steps: 80%, 90%, 100%, 110%, 125%, 150%, 175%, and 200%. The settings offer the usual ones, 90% to 150%, in segments (and the current one when it is another); the keyboard reaches every step. | 100% |
| “Terminal text size” | The font size of the terminals and of the [Claude Code view](claude-code-view), in pixels, from 9 to 24 in half-pixel steps. The zoom adds to it. | 12.5 |
| “Editor text size” | The font size of the [editor](editor), in pixels, from 9 to 24; the lines grow with it. The zoom adds to it. | 12.5 |

From the keyboard, anywhere, even in a terminal or with a dialog open: `Ctrl+=` (or `Ctrl++`) zooms in one step, `Ctrl+-` zooms out one step, `Ctrl+0` goes back to 100% (`⌘=`, `⌘-`, `⌘0` on macOS). The `+` and `-` keys of the numeric keypad work too.

A terminal whose font changes resizes at once to fit its area, and its program is told its new size in columns and rows: nothing spills over or gets cut.

> [!NOTE]
> The interface is laid out for at least 1000 × 600 pixels, the window’s minimum size. A step that would not leave that much room is grayed out, with the tooltip “Too big for the size of the window”, and `Ctrl+=` stops before it. If the window shrinks below the chosen step, the interface switches to the biggest step it can hold, and the page says so; the chosen step comes back as soon as the window is big enough.

### Language

Both choices take effect at once, with no restart: the window, the app’s menus, and the texts Escouade writes switch to the new language.

| Setting | Effect | Default |
|---|---|---|
| “Interface language” | The language of the window, the app’s menus, and the messages Escouade shows. | “System” |
| “Language of texts written by the agents” | The language of what Claude writes for you: commits, pull requests, comments, instructions to agents. | “Same as the interface (default)” |

### Interface language

Three choices, each written in its own language so you can find yours whatever language the interface is in:

- “System (French)” follows the computer’s language. Escouade comes in French and English: it picks French when the first preferred language of the system is French, English otherwise. The system’s language is named in the interface’s language, so a French system with an English interface gives “System (French)”.
- “English”.
- “Français”.

The change also reaches the app’s native menus and the tooltip of the taskbar icon.

### Language of texts written by the agents

This choice decides which language Escouade asks Claude to use for what leaves the app:

- the proposed commit messages and pull request descriptions, see [Git](git);
- the comments posted to Jira, Trello, GitHub, Linear, ClickUp, GitLab, and Azure Boards, see [the integrations](integrations);
- the instructions Escouade gives to agents.

The three choices are “Same as the interface (default)”, “English”, and “Français”. With “Same as the interface”, the proposed commit message follows the language of the repository’s latest commits, to blend in. With “English” or “Français”, that language is the one asked for.

> [!TIP]
> Many teams commit in English with an interface in French: pick “Français” for the interface and “English” for the texts Claude writes.

## Written texts

This group is on the “Agents” page. “Model for written texts” chooses the model that writes the short texts Escouade has drafted outside an agent’s conversation:

- the name given to a new agent;
- the proposed commit messages and pull request descriptions, see [Git](git);
- the worktree commands “✦ Fill in automatically” proposes after reading the project.

Each question gets 90 seconds for its answer; past that, the process is stopped with everything it started, and the answer is read up to a bounded size. Only the installed [harnesses](settings-claude-code) are offered. By default, it’s “Claude Haiku” when Claude Code is installed, otherwise the first harness found. “Codex (GPT-6-Luna)” writes them with `codex exec`, on the Codex account of the agent or project concerned: with no session kept (`--ephemeral`), none of your settings nor MCP servers, in a read-only sandbox, and with no tool for a name or a message. “Copilot (GPT-5 mini)” writes them with `copilot -p`, on the Copilot account of the agent or project: the question goes through its input, never its command line, with no tool for a name or a message, none of your MCP servers nor instructions, and its session is removed once the answer is read; a Copilot Free account takes no named model, and the question then goes through `auto`. A harness chosen then uninstalled gives way to that default. With no harness installed, the group doesn’t show.
