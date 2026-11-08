# Settings: the terminals (Advanced)

The “Terminals” group of the “Advanced” page tells Escouade where to find the shells used by the built-in terminals, launch commands, worktree commands, and tests.

## Open the page

Open the settings with `Ctrl+,` (`⌘,` on macOS), then choose “Advanced”, “Terminals” group. These are the app’s settings, shared by all projects, and they apply as you set them. The terminals themselves are described in [terminals](terminals).

The group holds a few paths. Escouade detects shells on its own; you only fill in a field to point to a shell it can’t find, or to prefer another one.

## Paths

Under the group’s title, the “Detected:” line lists the shells found, for example “Detected: PowerShell, Git Bash, WSL”, or “Detected: none”. It updates once the settings are saved.

On Windows, the group has three fields:

| Setting | Effect | Default |
|---|---|---|
| “PowerShell 7” | The path to `pwsh.exe`. | Empty (“blank = auto”) |
| “Git Bash” | The path to `bash.exe`, the bash of Git for Windows. | Empty (“blank = auto”) |
| “WSL distribution” | The name of the WSL distribution to use, for example `Ubuntu`. | Empty (“blank = default distribution”) |

On macOS, the group has a single field, “bash” (“blank = auto”), for the path to bash.

> [!NOTE]
> On Linux, the group shows the same three fields as on Windows, but only the “Git Bash” field counts: it then points to `bash`. The “PowerShell 7” and “WSL distribution” fields have no effect there.

An empty field lets Escouade search. A path that points to no file isn’t replaced by detection: the matching shell isn’t offered. For PowerShell, Escouade then falls back to Windows PowerShell if it’s installed.

## How shells are detected

### On Windows

Shells are listed in this order:

1. **PowerShell**: the path in the “PowerShell 7” field if it’s filled in. Otherwise `pwsh.exe` in the `PATH`, then the `PowerShell\7` and `PowerShell\7-preview` folders of Program Files, then the Microsoft Store alias (`WindowsApps\pwsh.exe`). Without PowerShell 7, Windows PowerShell (`powershell.exe`, shipped with Windows) is offered instead, as “Windows PowerShell”.
2. **Git Bash**: the path in the “Git Bash” field if it’s filled in. Otherwise the `bash.exe` in Git’s folder, found from `git.exe` in the `PATH` (`bin\bash.exe` or `usr\bin\bash.exe`), then `Git\bin\bash.exe` in Program Files.
3. **WSL**: offered as soon as `wsl.exe` exists. It’s named “WSL”, or “WSL (Ubuntu)” with the distribution from the “WSL distribution” field. The terminal opens in the right folder, in that distribution; with the field empty, it’s WSL’s default distribution.

### On macOS and Linux

The login shell (the `SHELL` variable) comes first if it’s zsh, bash, fish, or sh, then zsh, bash, and fish, searched in `/bin`, `/usr/bin`, and the `PATH`. `sh` is only offered if no other shell is found. The “bash” field overrides where bash is, except when `SHELL` already names bash: that bash is then the one used.

## Which shell is used for what

The order of the list matters: there’s no “default shell” setting, it’s the first shell detected.

- In the sidebar, the **+** button of the “Terminals” section (“New terminal”) opens a menu with every detected shell: you pick one.
- “Open a terminal” (an agent’s menu) and “Open a terminal here” (the editor’s file tree menu) use the first shell.
- A launch command or a worktree command has its own “Shell” choice, in the project’s settings (see [its pages](settings-projects)). A new command starts with the first shell. See [launch](launch) and [worktrees](worktrees).
- The Kanban’s “Test command” (its [“Kanban”](settings-board) page) and test launches ([testing](testing)) also use the first shell.

With no shell detected, the **+** button shows “No shell found (PowerShell 7, Git Bash, WSL). Check the settings.”

## What is set elsewhere

- **The proxy in terminals**: “Also apply the proxy to the built-in terminals” is in the [“Network”](settings-network) group, just above. It also applies to launch and worktree commands.
- **Claude Code in WSL**: on Windows, when the executable path ([“Accounts and harnesses”](settings-accounts) › Advanced) is empty and no `claude` is found on Windows, Escouade looks for one installed in WSL, in the distribution from the “WSL distribution” field (the default distribution if it’s empty).
