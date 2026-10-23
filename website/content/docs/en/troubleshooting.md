# Troubleshooting

The problems Escouade runs into most often, each with its symptom, its cause, and its fix: Claude Code not found, WSL, proxy, quotas, usage limit, worktree, git, notifications. At the end, where to read the app’s log and how to report a bug.

## Claude Code

### Claude Code is not found

**Symptom.** The welcome screen says “Claude Code couldn’t be found on this machine”. The Kanban shows “Claude Code not found — no tickets will start”, and Settings › About shows “Claude Code not found”.

**Cause.** Escouade looks for `claude` in this order:

1. The path in Settings › Harness, “Executable path” field, when it is filled in. If it is filled in, it decides alone: a wrong path gives “not found”, even if `claude` is in the `PATH`.
2. Otherwise, the `PATH`, then the usual installer locations. On Windows: `%USERPROFILE%\.local\bin\claude.exe`, `%USERPROFILE%\.claude\local\claude.exe`, and `%USERPROFILE%\AppData\Roaming\npm\claude.cmd`. On macOS and Linux: `~/.local/bin/claude`, `~/.claude/local/claude`, `/opt/homebrew/bin/claude`, and `/usr/local/bin/claude`.
3. On Windows, failing that, a Claude Code installed in WSL (see below).

The message says so with the command that installs it again: “Claude Code can’t be found: install it again (…), or set its path in Settings › Harness.”, when signing in an account, when starting an agent and in Settings › Harness.

**Fix.** Install Claude Code and sign in, or give its path in Settings › Harness (see the [Harness settings](settings-claude-code)). The field accepts `~` for your home folder, and on Windows you can leave out `.exe` or `.cmd`. The line under the field answers right away: “Found: …” with the path it picked, or “Not found. Give the path to claude.exe, …”. Clear the field to go back to automatic detection (“blank = detect automatically”).

A Claude account can have its own executable (Settings › Accounts, “Executable” field, “The one in the settings” by default). A path that exists but won’t run gives “can’t start” followed by that path.

### Claude Code disappeared after an interrupted update

**Symptom.** Signing in any account stops right away, or Escouade says “Claude Code’s installation was interrupted (… is left over)”. In a terminal, the `claude` command no longer exists.

**Cause.** An update of Claude Code installed with npm (`npm install -g`) was cut halfway: npm had removed the old `claude` and never put the new one in place. A temporary link `.claude-XXXX` is left next to the old one (in `/opt/homebrew/bin` for example), or the old package moved aside (`@anthropic-ai/.claude-code-XXXX`). Up to 1.8.4, a `claude` launched by Escouade could start this update, and Escouade cut it by stopping that process; since then, its `claude` processes no longer update themselves and Escouade runs `claude update` apart (see the [Harness settings](settings-claude-code)).

**Fix.** In a terminal, run the installation again:

```sh
npm install -g @anthropic-ai/claude-code
```

If npm refuses (`ENOTEMPTY`), first delete what’s left (the path the message gives), then run it again. Escouade finds Claude Code again by itself.

### Claude Code is not found when the app starts from the Finder (macOS)

**Symptom.** `claude` answers in Terminal, but Escouade, opened from the Finder or the Dock, can’t find it, nor `git` or `node` if you installed them with Homebrew.

**Cause.** macOS gives apps launched from the Finder or the Dock a minimal `PATH`, not your terminal’s. At startup, Escouade completes it with your login shell’s (it asks the shell with `$SHELL -ilc`), then adds `/opt/homebrew/bin`, `/usr/local/bin`, and `~/.local/bin`. It waits for the shell for only five seconds: a profile that asks for input, or takes longer, cancels that step. The same goes for a desktop launcher on Linux.

**Fix.** Give the full path of `claude` in Settings › Harness (`which claude` in a terminal prints it), or lighten your shell profile so that it answers quickly.

### Claude Code is installed in WSL

**Symptom.** Claude Code is only installed in WSL, and Escouade can’t find it, or an agent can’t work in a folder.

**Cause.** On Windows, Escouade only looks in WSL when Windows has no `claude`. It looks in the distribution set in Settings › Terminals (“WSL distribution” field, “blank = default distribution”), at `~/.local/bin/claude`, `~/.claude/local/claude`, `/usr/local/bin/claude`, and `/usr/bin/claude`. That search can start the distribution and take a few seconds.

**Fix.** Give the path in Settings › Harness, in one of these forms:

- `~/.local/bin/claude`
- `/home/<user>/.local/bin/claude`
- `\\wsl.localhost\<distribution>\home\<user>\.local\bin\claude`

The line under the field confirms it: “Found in WSL (Ubuntu): /home/ada/.local/bin/claude”, for example. Escouade then runs `claude` through `wsl.exe`, in the agent’s folder (seen from WSL, `/mnt/c/…`), and reads its sessions and its sign-in in the Linux home folder. A project folder that WSL can’t reach (a network share, another distribution’s folder) is refused: “The folder … can’t be reached from WSL (…): the Claude Code installed there can’t work in it.”

### MCP tools and the Claude Code view tracking don’t work from WSL

**Symptom.** Under the path field, Settings › Harness shows: “WSL’s network is NAT: Escouade’s MCP server and the tracking of the Claude Code view can’t reach the app.”

**Cause.** In NAT mode, WSL isn’t on Windows’ network: Escouade’s MCP server and the Claude Code view’s hooks, which listen on Windows’ `127.0.0.1`, are out of its reach.

**Fix.** Switch WSL to mirrored networking. In `%UserProfile%\.wslconfig`:

```ini
[wsl2]
networkingMode=mirrored
```

Then, in a Windows terminal, run `wsl --shutdown` and restart Escouade.

### A variable from the shell profile doesn’t reach Claude Code in WSL

**Symptom.** Claude Code, in WSL, doesn’t see a variable set in `~/.profile` or `~/.bashrc`.

**Cause.** Escouade starts it without the shell profile: those files aren’t read.

**Fix.** The proxy from Settings › Network, on the other hand, is passed to Claude Code in WSL. For another variable, define it on the Windows side (in your account’s environment variables), add its name to the Windows `WSLENV` variable (names are separated by `:` there), and restart Escouade. Escouade keeps your `WSLENV` value and adds the variables it passes itself.

## Network

### Escouade is behind a proxy

**Symptom.** Quotas, integrations, updates, or Claude Code can’t connect from a corporate network.

**Fix.** In Settings › Network, fill in “HTTP(S) proxy” (for example `http://user:password@proxy:3128`) and, if needed, “Exclusions” (the `NO_PROXY` list, separated by commas). The proxy is passed to Claude Code processes, quota reads, integrations, and updates. For an agent, it applies the next time its process (re)starts. For the built-in terminals, the launch commands, and the worktree setup and teardown commands to use it too, turn on “Also apply the proxy to the built-in terminals” (off by default). See the [Network settings](settings-network).

### A company proxy refuses the update

**Symptom.** The message “Couldn’t download the update” or “Couldn’t check for updates” mentions a “403 Forbidden”, asks whether it is “blocked by the network or a proxy?”, says that “your company’s proxy” refused the installer, or says the proxy asks for authentication.

**Cause.** A company proxy (Zscaler, Netskope, Squid…) often lets the check through but refuses the installer to a client that isn’t a browser. Escouade announces itself honestly (`Escouade/<version>`) and doesn’t pass itself off as a browser; when the proxy refuses the request, it doesn’t make it again without the proxy. An automatic check only shows its error after several failures in a row, over more than an hour.

**Fix.** If the message says the proxy asks for authentication, put the username and password in the proxy’s address, in Settings › Network (for example `http://user:password@proxy:3128`). Otherwise, click “Download in the browser” in the message: the release’s page opens, and the proxy may ask you to confirm a warning. For the built-in update to go through, ask your IT department to allow `github.com` and `release-assets.githubusercontent.com`, and the download of the release files for the client whose User-Agent starts with `Escouade/`. The details are in [“Behind a company proxy”](updates#behind-a-company-proxy).

### A corporate certificate makes connections fail

**Symptom.** An “UnknownIssuer” error, or another TLS failure, for integrations, quotas, updates, or Claude Code.

**Cause.** A corporate proxy decrypts the traffic with a certificate nobody recognizes. Escouade’s own requests (integrations, quotas, updates) trust recognized certificates and those installed on the system, usually a corporate proxy’s. Claude Code has its own list.

**Fix.** Install the company certificate in the system: Escouade then trusts it for its own requests. Claude Code has its own list: for it, or if you can’t install the certificate, turn on “Skip TLS certificate verification” (Settings › Network, “Certificates” group). The setting applies to integrations, quotas, updates, Claude Code processes (the next time their process starts), and the commands agents run (npm, node…).

> [!WARNING]
> With this setting, an intercepted connection, tokens included, would no longer be detected. Keep it for a network you trust.

## Quotas and usage limit

### Quotas don’t show

**Symptom.** In the status bar, `5h` and `W` have an empty bar and a “—” in place of the countdown, and the tooltip says “Quota unavailable”. In Settings › Accounts, the account says “Not signed in”, “Not signed in with a claude.ai account (API key?)”, or “Sign-in expired: run Claude Code again for this account.”

**Possible causes.**

- Quotas are those of a claude.ai subscription: Escouade reads them with the sign-in that Claude Code keeps for the account. A Claude Code used with an API key has none.
- The sign-in has expired.
- Anthropic can’t be reached (network, proxy, certificate: see above). The last values stay displayed, and Escouade tries again at the next read, about every five minutes.
- On macOS, Claude Code keeps its sign-in in the keychain. If you deny access when macOS asks, Escouade doesn’t ask again for ten minutes.

**Fix.** In Settings › Accounts, “Sign in…” (or “Sign in again…”) opens a terminal where Claude Code signs you in (use the `/login` command if it doesn’t offer it). For Main, it is your own Claude Code’s sign-in. See [Claude accounts](accounts).

### An agent stopped on the usage limit

**Symptom.** The “The turn ended with an error” card shows Claude Code’s message about the usage limit, and the agent stops.

**Cause.** The 5-hour window or the week of your subscription is used up.

**Fix.** What the app does depends on your settings:

- **Auto-resume.** The “Auto-resume after the usage limit” setting (Settings › Harness, on by default) sends “continue” once the quota has reset. When Claude Code tells the reset time, the card says “Auto-resume” followed by the time, with a “Cancel auto-resume” button.
- **Another account.** With several accounts, the “Resume on Pro” button (with the name of another account), and its “Resume on another account” menu, restart the agent with its session on another account. Accounts that are inactive, not signed in, or above “Pause above quota” aren’t listed.
- **Ticket agents** wait for their account to reset, unless you turn on “Resume a ticket’s agent stopped by the usage limit on another account” (Settings › Harness, with several accounts).

To avoid reaching the limit, the [autopilot](autopilot) pauses before, at the “Pause above quota” threshold.

### An agent stops on “OAuth session expired and could not be refreshed”

**Symptom.** A turn stops on “Failed to authenticate: OAuth session expired and could not be refreshed”, even though the account is signed in (`/login` works).

**Cause.** The Claude Code processes of an account share its sign-in. When the token expires, one of them refreshes it, under a lock in the account’s folder. If the server refuses the refresh token, for instance because another process already used it, Claude Code keeps it as dead in memory. That process then fails every turn, for up to 30 seconds, even if a good sign-in has reached the disk meanwhile.

**Fix.** Escouade handles it. When such a turn ends, it reads the account’s sign-in again:

- **It’s good**, or it isn’t the one the process started with: Escouade restarts the agent’s process on its session and sends it “continue”, twice at most (after 3, then 10 seconds). The conversation says so in one line: “Account session refreshed: the agent goes on.”
- **It expired without being refreshed, or it’s gone** (Claude Code empties the sign-in when its refresh token is refused), or both restarts failed: the “Sign in again” card names the account. Its “Sign in again” button opens the account, with “Sign in…”. Once you’re signed in, the agent goes on by itself within 20 seconds: “Main account signed in again: the agent goes on.”

A ticket’s agent goes on the same way: its ticket isn’t blocked. To make these refusals rarer, the launches of an account whose token expired go one after the other: the first refreshes the token, the others wait for the new one, 20 seconds at most.

## Agents

### An agent stopped

**Symptom.** The agent goes to “Error”. The conversation says “Claude Code stopped (code N)”, followed by what Claude Code wrote on its way out, and a banner adds “The agent stopped. Send a message to restart Claude on the same session.”

**Cause.** The agent’s Claude Code process ended on its own. The message it left usually says why (sign-in, proxy, version…).

**Fix.** Send a message: Escouade restarts Claude on the same session. A process stopped on purpose isn’t an error: Escouade stops idle processes after a delay (Settings › Harness, “Stop idle agent processes after”, 30 minutes by default, 0 for never) and restarts them at the next action.

### Claude session not found

**Symptom.** A notice says “Claude session not found: a new session will start with your next message.”

**Cause.** Escouade restarted the agent on its session, but Claude Code no longer knows it (for example because the account’s configuration folder was emptied or changed).

**Fix.** There is nothing to do: the next message starts a new session. The conversation stays displayed in Escouade, but Claude no longer has its context.

## Worktrees

### A worktree setup fails

**Symptom.** The agent’s conversation shows “The worktree setup failed on `npm ci` (code 1).”, followed by the last lines of the output and by “Full output” with a path.

**Cause.** A setup command of the project failed (Settings › Projects, “Worktrees” group, “When a worktree opens” section). Escouade runs them in order and stops at the first one that fails. The reason is in parentheses: “code 1”, “stopped”, “not done in 20 min” (a command gets 20 minutes), “doesn’t start” followed by the cause, or “the folder … doesn’t exist in the worktree”.

**Fix.** Read the full output in `~/.escouade/logs/<agent>-setup.log`: each step has its name and its ending there. Fix the command in the project’s settings (see the [project settings](settings-projects)), or run it by hand in a terminal opened in the worktree. The commands receive `ESCOUADE_PROJECT_DIR`, `ESCOUADE_WORKTREE_DIR`, and `ESCOUADE_BRANCH`. The agent remains usable; for a ticket agent, the failure is added to its first message, and Claude has to do what it takes to be able to work and test. The log is replaced at each new setup, and goes away with the agent.

When an agent is deleted, a teardown command that fails is reported the same way: “Agent deleted, but the worktree teardown failed on …”. The agent is deleted all the same.

## Git

### A git command is interrupted

**Symptom.** A message says “git status stopped: still not done after 120 s” (the command and the time change), in a commit preview, a merge, a sync…

**Cause.** Escouade stops a git command that doesn’t end, so that the window doesn’t stay stuck: a read (status, log, diff…) after 2 minutes, a command that reaches the remote (push, pull, fetch) after 3 minutes (1 minute for the fetch run in the background), and a command that runs git’s hooks (commit, merge, rebase, switching branch) after 30 minutes.

**Fix.** Run the same command by hand in a terminal opened in the project’s folder to see what blocks it (a hook, the remote’s credentials, the network), then do the action again in Escouade.

## Notifications

### Notifications don’t appear

**Symptom.** No notification, or no sound, when an agent asks a question, finishes, or fails.

**Possible causes.**

- The type is turned off. In Settings › Notifications, “Notify me about” lists “Questions and permissions”, “Finished tasks”, “Errors”, and “Tickets (ready to review, blocked)”. A type that is off gets no system notification and no chime; the tab, the card, and the taskbar (the Dock on macOS) still flag the agent.
- The system notification only appears if Escouade’s window isn’t in the foreground (shown, active, and not minimized): in the foreground, only the chime sounds. It also needs the switch of the “Channels” group to be on (“Windows notifications when the app is not in the foreground”, “macOS notifications…” on macOS).
- The sound is off: the “♪” button in the status bar and the “Sound on” setting control it, and “Test the sound” tries it.
- On Linux, Escouade doesn’t send system notifications. The chime is played by `paplay`: without it, no sound.
- Also check that the system allows Escouade’s notifications, and that a “do not disturb” mode isn’t hiding them.

See [notifications](notifications).

## Data and tokens

### A data file is unreadable

**Symptom.** Escouade starts without its projects, or with its default settings.

**Cause.** `state.json` or `settings.json` (or `integrations.json`, `sync-queue.json`) wasn’t valid JSON. Escouade kept a copy under a name like `state.broken.json` and started with the defaults for that file.

**Fix.** Open the copy to recover what it held, such as the list of projects. `app.log` says which file was the problem. See [local data](data).

### The system keychain is unreadable

**Symptom.** Settings › Integrations says “System keychain unreadable: restart Escouade or reconnect the account.” or “System keychain unavailable: the token stays in ~/.escouade/integrations.json.”

**Fix.** Restart Escouade; if the message comes back, reconnect the account (the token is then saved again). In the second case, the token stays in `integrations.json`, readable only by you on macOS and Linux.

## Terminals

### A terminal is slow

**Symptom.** A letter typed or a text pasted in a [terminal](terminals) takes a while to show, when a terminal usually answers in a few hundredths of a second.

**Cause.** The time goes into one of the steps a keystroke takes: the window sending it to the backend, the terminal’s program (the shell, or Claude Code in the Claude Code view) sending the echo back, the way back into the window, then the drawing. A paste of a few thousand characters into PowerShell takes a tenth of a second or more by itself: PSReadLine redraws the line as it grows, in Windows Terminal as in Escouade.

**Fix.** Open the app’s log (see “Where the app’s log is”, below): every echo that took more than 300 ms to show is noted there (at most once every 10 seconds per terminal), with the time of each step, for example “echo of a 1-byte write shown after 837 ms: written 0 ms after the backend got it, first output 835 ms after, the echo 835 ms after, to and from the window 1 ms (answer to the write 2 ms), painted 1 ms later”. Here nearly everything comes before “the echo”: the program was slow. A long “to and from the window” comes from the window, a long “painted” from its drawing. When each terminal opens, the log also says what draws it: “renderer webgl” followed by the graphics card’s name (a name like SwiftShader means drawing without a graphics card, which is slower), or “renderer dom”. Report it with these lines (see “Reporting a bug”).

**Reading the line.** One step carries nearly all the time:

- **Long “first output” and “the echo”**: the terminal’s program was slow to answer. Open the same shell outside Escouade (in Windows Terminal, for example): if it is slow there too, the cause is the shell or the machine, not Escouade. In PowerShell, try `pwsh -NoProfile` (a module of the profile can do work at every key) and `Set-PSReadLineOption -PredictionSource None` (a prediction from a module, such as `Az.Tools.Predictor`, is computed at every key).
- **Long “written … after the backend got it”**: the program had stopped reading what was typed, busy elsewhere.
- **Long “to and from the window”**: the time was lost between the window and the backend. Report it.
- **Long “painted”**: the drawing. Look at the “renderer” line of the same terminal.

On this path a keystroke usually takes 10 to 40 ms, and seldom more than 150 ms, even with agents at work, a test suite and a local model keeping the processor busy.

**On a work machine.** What a managed machine adds can slow every keystroke down without loading the processor:

- **An EDR or an antivirus** (Microsoft Defender for Endpoint, CrowdStrike, SentinelOne…) inspects the processes the terminal starts and what they exchange with Windows’ pseudo-console. The time then shows in “first output” and “the echo”. If the same shell is slow outside Escouade too, ask your IT department to look at what the tool does to `escouade.exe`, `conhost.exe` and the shell (`pwsh.exe`, `bash.exe`), or to leave them out of real-time inspection.
- **A remote desktop** (RDP, Citrix, a virtual desktop) sends the picture over the network: the echo is drawn on time in Escouade but reaches your screen late. The log then notes nothing, since nothing was late in the app, and a terminal outside Escouade, in the same session, is just as slow. Without a graphics card, the “renderer” line names SwiftShader or says “renderer dom”: the processor does the drawing.
- **The shell’s profile** your company provides (modules loaded, a prompt that asks the network or git): `pwsh -NoProfile` shows it at once.

If you see the delay and the log notes nothing, the time was not spent in Escouade: it is in the machine’s display (a remote desktop), or the echo took more than 10 seconds (a keystroke left unanswered is then forgotten). Only a visible window measures its keystrokes.

## Voice

### Dictation or the conversation hears nothing

**Symptom.** Dictation or the [voice conversation](chat#talking-with-the-agent-voice-conversation) stops on a message instead of listening, or a dictation ends on “Nothing heard: speak a little closer to the microphone, then try again.”

**Cause and fix.** The message says which:

- “No microphone found. Plug one in, or pick one in Settings › Voice.”: no microphone is plugged in, or the one chosen in [Settings › Voice](settings-voice#microphone) is gone along with the system’s.
- “The system keeps the microphone from Escouade.” or “No sound came from the microphone: it is muted, or the system keeps it from Escouade.”: the message gives your system’s steps.
  - **macOS** asks for permission at the first dictation, and refuses nothing afterwards: an app kept from the microphone gets silence. Open System Settings › Privacy & Security › Microphone, tick Escouade, then restart the app.
  - **Windows**: Settings › Privacy & security › Microphone, turn on “Microphone access”, then “Let desktop apps access your microphone”.
  - **Linux**: check that PulseAudio or PipeWire is running, that the microphone is not muted (pavucontrol, or the sound settings), and that your account may use it (the `audio` group).
- “The microphone is taken by another app.”: close it, or pick another microphone in Settings › Voice.
- “No sound system answers.” (Linux): start PulseAudio or PipeWire (or check ALSA), then try again.
- “Nothing heard”: the microphone works, but Whisper found no words. Speak closer to the microphone, and check in Settings › Voice that “Microphone used” is the right one and that “Spoken language” is yours.

### Online dictation fails

**Symptom.** With “Transcribed by” on “Online”, the dictation ends on a message and nothing is written.

**Fix.** The message names the service:

- “No OpenAI key is saved.”: paste your key in [Settings › Voice](settings-voice#online-dictation), or go back to “Local (Whisper)”. On Linux, the key cannot be saved: stay local.
- “OpenAI refuses the key.”: check it with the service, then “Replace”.
- “OpenAI does not answer”: check the network and the proxy ([Settings › Network](settings-network)), or go back to local.
- “OpenAI did not transcribe the dictation”, with the code and reason the service gives: settle what it says on your account with it, or go back to local.

### The voice conversation does not start

**Symptom.** A click on “Conversation” shows “Voice conversation is not available: the voice’s engine (ONNX Runtime) is missing from the app. Reinstall Escouade.”, or the download of a model or a voice fails.

**Fix.** ONNX Runtime ships with the app: if it is missing, reinstall Escouade from [the download page](installation). A download that fails goes through the proxy of the network settings: behind a corporate proxy, check [Settings › Network](settings-network) and that `huggingface.co` can be reached. What downloads, and where, is in [Settings › Voice](settings-voice#what-downloads-and-where).

## Log and reports

### Where the app’s log is

The log is `~/.escouade/app.log` (`C:\Users\<you>\.escouade\app.log` on Windows). Each line carries the date, the level, and the text: startup and version, agent stops, errors, migrations, updates. When it goes over about 5 MB, the next startup files it under the name `app.old.log`. Any text editor opens it.

### Asking a question

For a question, an idea or a hand, join [Escouade’s Discord](https://discord.gg/m3KYtN8gxP).

### Reporting a bug

Open an issue on [GitHub](https://github.com/guillaume-gagnaire/escouade/issues) with:

- Escouade’s version (Settings › About, “Escouade 1.7.5” for example) and your system (Windows, macOS, or Linux, and its version);
- what you were doing, what you expected, what happened, and a screenshot if it helps;
- the useful lines of `app.log`. Read them before attaching them: they can mention the paths of your folders and project names. Never paste a token.
