# Installation

The requirements of each system, installing on Windows, macOS and Linux, using a Claude Code installed in WSL, the PATH on macOS, then what you see at first launch and where updates come from.

## Requirements

| Item | Windows | macOS | Linux |
|---|---|---|---|
| System | Windows 10 or 11 (WebView2, present by default on Windows 11) | macOS 11 (Big Sur) or later, on Apple Silicon or Intel Macs | x86-64, glibc 2.35 or later (Ubuntu 22.04, Debian 12, Fedora 36 and later), with WebKitGTK 4.1 |
| Git | Git for Windows | The Xcode command-line tools (`xcode-select --install`) or Homebrew’s | Your distribution’s |

On top of that:

- **A harness**, installed and signed in to your account, the way you would for using it in a terminal: Claude Code (see the [Claude Code documentation](https://code.claude.com/docs/en/overview)), or Codex, OpenCode or GitHub Copilot CLI (see [below](#the-other-harnesses)). Escouade starts the one on your machine.
- **Git**: Escouade uses it for diffs, commits, branches and worktrees. If a project’s folder is not a repository, Escouade initializes one when you create the project.
- **Optional on Windows**: PowerShell 7 (without it, the built-in terminals use Windows PowerShell) and WSL.

### Where Escouade looks for Claude Code

Escouade looks for `claude` in this order:

1. the path in Settings › Harness › “Executable path”, if it is not blank (it is then the only place tried);
2. the `PATH`;
3. the places where the Claude Code installers put it (table below);
4. on Windows, when there is none, a Claude Code installed in WSL (see [below](#claude-code-in-wsl)).

| System | Places tried |
|---|---|
| Windows | `%UserProfile%\.local\bin\claude.exe`, `%UserProfile%\.claude\local\claude.exe`, `%UserProfile%\AppData\Roaming\npm\claude.cmd` |
| macOS and Linux | `~/.local/bin/claude`, `~/.claude/local/claude`, `/opt/homebrew/bin/claude`, `/usr/local/bin/claude` |

The path in the settings accepts `~` for your home folder, and on Windows it can leave out `.exe` or `.cmd`. Leave the field blank to detect it automatically. The line under the field says where Escouade found `claude` (“Found: …”) or, when it is not found, the path forms it accepts.

Without any harness, the welcome screen says “No harness found” with each one’s install command, and the Kanban starts no tickets (“No harness found — no tickets will start”); an agent whose harness is gone says so in its conversation. Install one or set Claude Code’s path in the settings: when you save another path, these messages update without restarting Escouade.

### The other harnesses

Claude Code isn’t required: Escouade also runs its agents on Codex, OpenCode and GitHub Copilot CLI, and a single harness is enough. Install the ones you want and sign in to them; they then show in Settings › Harness, with their version. Then add each one’s account in Settings › Accounts: only the accounts added start agents (see [the accounts](accounts)).

| Harness | Installation | Sign-in | Tested with |
|---|---|---|---|
| Codex | `npm install -g @openai/codex` | `codex login` (ChatGPT account) | 0.163.0 |
| OpenCode | `npm install -g opencode-ai` | `opencode auth login` (a provider, or Ollama’s local models) | 1.19.3 |
| Copilot CLI | `npm install -g @github/copilot` | `copilot login` (GitHub account with Copilot) | 1.0.98 |

Escouade starts the native binary the npm package installs (on Windows, not its launcher). A version older than the tested one is refused, with the command that updates it; a newer one is used, without a warning. The Codex desktop app often installs an older version, which the `PATH` may find first: install npm’s then. OpenCode 2 isn’t supported yet.

What each one does, its modes and what protects it, its cost and quota: see [the harnesses](harness).

## Installing Escouade

The installers are on the [GitHub releases](https://github.com/guillaume-gagnaire/escouade/releases/latest) page.

### Windows

1. Download the `.exe` installer of the latest release.
2. Run it. It installs Escouade for your user account, without asking for administrator rights, and it speaks the system language (French or English).
3. Open Escouade from the Start menu.

> [!NOTE]
> Escouade used to be called “Claude Code Manager”. Installed over an old version, it replaces it and keeps your settings and conversations.

### macOS

1. Download the `.dmg` of the latest release. It is universal: the same file works on Apple Silicon and Intel Macs.
2. Open it and drag Escouade into the Applications folder.
3. Launch Escouade.

The app is signed with an Apple Developer ID certificate and notarized by Apple: macOS opens it without asking for any authorization.

### Linux

The `AppImage` runs on any distribution that meets the requirements:

```sh
chmod +x Escouade_*.AppImage
./Escouade_*.AppImage
```

It is the one the built-in update replaces. On Debian or Ubuntu you can also install the `.deb` package:

```sh
sudo apt install ./Escouade_*_amd64.deb
```

On Linux, the tokens of the integrations (Jira, Trello, GitHub, Linear) are kept in `~/.escouade/integrations.json`, because no system keychain is supported.

## Claude Code in WSL

On Windows, Escouade can run a Claude Code installed in WSL.

1. Install Claude Code in your WSL distribution and sign it in.
2. Open Settings › Harness. In “Executable path”, enter the path of `claude` in one of these forms:
   - `~/.local/bin/claude`;
   - `/home/<user>/.local/bin/claude`;
   - `\\wsl.localhost\<distribution>\home\<user>\.local\bin\claude`.
3. Leave the field blank if Windows has no `claude`: Escouade then looks for its own in WSL, in the usual places (`~/.local/bin/claude` first).
4. Read the line under the field: “Found in WSL (Ubuntu): …” confirms that `claude` is found, and in which distribution.

The distribution used is the one the path names (`\\wsl.localhost\<distribution>\…`), otherwise the one in Settings › Terminals › “WSL distribution”, otherwise WSL’s default distribution.

Escouade starts `claude` through `wsl.exe`, in the agent’s folder (seen from WSL under `/mnt/c/…`). Its sessions, sign-in and quota are read from the Linux home folder. A project folder that WSL can’t reach (a network share, another distribution’s folder) is refused.

> [!WARNING]
> Escouade’s MCP tools and the live tracking of the [Claude Code view](claude-code-view) need WSL’s mirrored networking. Add `networkingMode=mirrored` under `[wsl2]` in `%UserProfile%\.wslconfig`, then run `wsl --shutdown`. When WSL’s network is in NAT mode, the settings say so under the path field.

Claude Code is started in WSL without the shell profile: a variable set in `~/.profile` or `~/.bashrc` does not reach it.

## The PATH on macOS

An app started from the Finder or the Dock does not receive your terminal’s `PATH`: without a precaution, a `claude` or a `git` installed with Homebrew would not be found. So at startup Escouade asks your login shell for its `PATH` (`$SHELL -ilc`, with 5 seconds to answer) and merges it with the one it received: the shell’s folders first, then those already there, without duplicates. It adds `/opt/homebrew/bin`, `/usr/local/bin` and `~/.local/bin` if they are not there. This `PATH` is the one of every process it starts: Claude Code, git and the commands of the agents.

On Linux the same mechanism applies, which matters when Escouade is started from the desktop launcher rather than from a terminal.

If `claude` is still not found, set its path in Settings › Harness.

## First launch

As long as no project exists, Escouade opens a welcome screen: its logo, the sentence “Add a project to run agents, follow their questions, and open terminals.” and the “+ Add a project” button.

- If no harness is found, a warning says so: “No harness found”, with each one’s install command. Then open the settings with the ⚙ button of the status bar or `Ctrl+,`, “Harness” tab. On macOS, `Ctrl` becomes `⌘`.
- As long as no account is added, the screen says “No account”: “Add an account: the one already signed in on this machine, or a new one. Only the accounts added start agents.” The “Add an account…” button offers “This machine’s account”, your harness already signed in, or “A new account” (see [the accounts](accounts#add-an-account)).
- Click “+ Add a project” to open the “New project” window. The [quick start](quick-start) describes it field by field and goes all the way to the first commit.
- Escouade also asks, in a “Help improve Escouade?” dialog, whether it may send usage statistics: “Share statistics” or “No thanks”. Nothing is sent before you answer. The detail is on the [Privacy](/en/privacy/) page, and the choice can be changed in Settings › About (see the [“About” settings](settings-about)).

The window opens at 1440 × 900 (1000 × 600 at the minimum); Escouade then remembers its size and position. Your projects, conversations and settings are written in `~/.escouade/`: see [local data](data).

## Updates

Updates come from Escouade’s GitHub releases. The app downloads the new version in the background and installs it without an installer window. The status bar then shows “Update X ready · Restart”: you restart when you want, or Escouade restarts by itself when no agent is working and everything is saved. That setting, “Install updates automatically”, is on by default; it is in Settings › About, where “Check for updates” runs a check right away.

See [updates](updates) and the [“About” settings](settings-about).
