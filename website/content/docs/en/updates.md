# Updates

Escouade updates itself from GitHub releases: it downloads in the background, then installs at the restart you choose or, if you allow it, by itself when it is at rest. This page explains how, system by system, what to do if it fails, and where to read what’s new.

## How it works

- Escouade looks for a new version shortly after it starts, then five minutes after each check ends. It reads the `latest.json` file of the latest release published on [GitHub](https://github.com/guillaume-gagnaire/escouade/releases). A release is only published once the files for all three systems are uploaded and checked, so Escouade never meets a half-built one.
- When a newer version exists, it downloads it right away, without asking, and checks its signature. During the download, the status bar shows “Update 1.8.0…” (with the version number).
- Once downloaded, the update waits: the status bar shows “Update 1.8.0 ready · Restart”. It installs when you restart the app, when the app restarts by itself, or when you quit it.
- Installation happens without an installer window. The proxy and certificate verification set in Settings › Advanced › Network apply to the checks and downloads too. When something fails, see [“When it fails”](#when-it-fails).

## Beta versions

A beta is an early version, published so that volunteers can try it before everyone else: it carries the number of the version it prepares followed by `-beta` and a number, for example “2.2.0-beta.1”. It may have flaws the stable version won’t have.

The “Receive beta versions” option (Settings › General, unticked by default) decides whether you get them:

- **Unticked**, Escouade ignores every beta and reads only `latest.json`, which never holds one. If you are already on a beta, you stay on it until the stable version that follows it (2.2.0 after 2.2.0-beta.3); an older version, such as a 2.1.5, doesn’t replace it.
- **Ticked**, Escouade first reads `beta.json`, in the latest stable release, which gives the most recent version, beta or stable; if it’s missing, it reads `latest.json`. Ticking it starts a check at once, and the setting is saved right away, without “Save”.

A beta shows wherever Escouade says a version: “Escouade 2.2.0-beta.1 (beta)” in Settings › General and at the bottom of the settings bar, “Update 2.2.0-beta.1 (beta)…” in the status bar. Its notes are in the update window as for any version; it isn’t announced on Discord.

## Installing an update

### Restart yourself

Clicking “Update 1.8.0 ready · Restart” opens the “Escouade 1.8.0 is ready” window, which shows the release notes. It has two buttons: “Later” and “Restart now”.

The window warns you about what a restart cuts off:

- Editor files aren’t saved: “Save your files first”. The “Restart now” button stays disabled as long as some remain.
- Agents are working or waiting for your answer: their current turn is interrupted. A ticket agent picks up again by itself; the others wait for your next message.

Escouade stops cleanly before installing: the agents’ processes and the terminals stop, and each agent’s session and the app’s state are saved. After the restart, the window comes back as it was (in front, behind another window, minimized, or closed to the notification area), and a message in the window says “Escouade 1.8.0 is installed.”, with a “See what’s new” button.

### Restart automatically

The “Install updates automatically” setting (Settings › General, on by default) lets Escouade restart by itself: “Escouade restarts itself when no agent is working and everything is saved.”

The app is at rest when none of this is true:

- An agent is working or waiting for your answer (a question or a permission).
- An agent has a subagent, a workflow or a background task (a command, a monitor…) running: a restart would stop them with its process.
- A worktree is being set up.
- A ticket is being approved (merge, pull request, push), or other work is under way outside an agent’s turn, for example a ticket starting or a merge you asked for by hand.
- A launch command or a step of a test launch is running, or a test launch is being prepared.
- A built-in terminal has received or displayed something in the last five minutes. A terminal that has been silent longer doesn’t count.
- The editor holds unsaved files.
- A dialog (settings, commit…) is open.
- You have touched the keyboard or the mouse in the window in the last five minutes. This only counts if the window is on screen: minimized or closed to the notification area, it no longer holds the restart back.

When the app is at rest, Escouade warns you thirty seconds before restarting: the status bar shows “Restarting in 30 s” with a “Later” button, and a system notification says “Escouade is restarting to update”, whatever the “System notifications” setting says (on Windows, with the same “Later” button; on Linux, only the status bar warns you). If the app stops being at rest during that countdown (an agent gets back to work, you touch the keyboard), the restart is canceled and waits for the next rest. “Later” pushes it back by thirty minutes.

### Quitting the app

Closing the window doesn’t quit Escouade: it stays in the notification area. If you quit it (“Quit” in the menu of its icon, or `⌘Q` on macOS) while an update is ready, the update installs and the app stays closed.

### If the installation fails

The app stays on the old version and says so: “The update to 1.8.0 couldn’t be installed.” with a “Retry” button, which reopens the update window. It no longer restarts by itself for that version; it tries again on request, or when you quit the app.

## Checking by hand

Settings › General shows the installed version (“Escouade 1.7.5”, for example) and a “Check for updates” button. The answer is “No update available.” (only when the check really got an answer), or the update appears in the tab, “Escouade 1.8.0 available” then “Escouade 1.8.0 ready” with “Restart now”, as in the status bar. When you check yourself, a failure is always shown, with its reason: “Couldn’t check for updates” or “Couldn’t download the update”, followed by the cause. An automatic check is more patient: see [“When it fails”](#when-it-fails). Behind a proxy or a corporate certificate, see [“Behind a company proxy”](#behind-a-company-proxy) and [troubleshooting](troubleshooting).

## When it fails

Escouade doesn’t bother you over a passing hiccup.

- **It retries by itself.** A passing failure (network down, a 403, 404, 408, 425, 429 or 5xx answer from the server, an unreadable answer) is retried once, three seconds later. A download that fails first re-reads `latest.json` and, if the same version is still listed, retries once with what the file says: a file’s link may have expired.
- **It stays quiet at first.** For an automatic check or download, the error only appears after at least three failures in a row, spread over at least an hour, and only once per series of failures. A series whose last failure is more than thirty minutes old starts over from zero: one failure before the computer goes to sleep and two after it wakes up make no alert. Checking and downloading each have their own series, and a success ends its own: a proxy that refuses the installer every five minutes ends up being reported, even while the checks go through.
- **It says so right away when you check yourself.** “Check for updates” always shows the failure, with its reason.

The error says what the request ran into:

- the HTTP code, for example “403 Forbidden”;
- the domain that answered, once redirects are followed (`github.com`, `release-assets.githubusercontent.com`…);
- the proxy the request went through, if there is one, for example `127.0.0.1:9000`;
- what the answer says about itself: the `Proxy-Agent` header, or a `Server` header or a block page that names a known proxy (Zscaler, Netskope, Squid, Blue Coat, Forcepoint, Fortinet, Palo Alto, Umbrella, Sophos, Skyhigh…), the start of which it quotes. A bare `Via` header isn’t enough to name a proxy: some add it to all their answers, even to a site’s refusal.

For a 403, it adds “blocked by the network or a proxy?”. For example: “403 Forbidden from release-assets.githubusercontent.com, through the proxy 127.0.0.1:9000 (Zscaler/6.2): blocked by the network or a proxy?”. For a 407, it says the proxy asks for authentication, and that you need to put the username and password in the proxy’s address, in Settings › Advanced › Network.

Escouade announces itself with an honest User-Agent, `Escouade/<version> (<system>; <architecture>)`, for the check and for the download. It never passes itself off as a browser.

## Behind a company proxy

A company proxy (Zscaler, Netskope, Squid…) often lets the check through, a small file, but refuses the installer to any client that isn’t a browser. Escouade doesn’t get around that rule: it says so, and gives you a way out.

### What the message says

When a company proxy that identifies itself refuses the installer, the message names it: “your company’s proxy (Zscaler/6.2) refused the installer (403 Forbidden from release-assets.githubusercontent.com, through the proxy 127.0.0.1:9000). Download it in the browser: you may have to confirm the proxy’s warning.”

When the proxy refuses the request (403, 407, refused tunnel), Escouade doesn’t make it again without the proxy: it gets around no rule of your company, and the message explains the refusal. For a 407, it says the proxy asks for authentication: put the username and password in the proxy’s address, in Settings › Advanced › Network (for example `http://user:password@proxy:3128`).

Only a proxy that can’t be reached (set but switched off, or a computer outside the company network) leads to a second try, without the proxy. [The app’s log](troubleshooting#where-the-app-s-log-is) then says whether direct access worked, and if both tries fail, the failure message gives both.

### Download in the browser

A failed download offers a “Download in the browser” button, in a message that stays up for thirty seconds. It opens the release’s page on GitHub, where you take the file for your system: the `.exe` installer on Windows, the `.dmg` on macOS, the `AppImage` or the `.deb` on Linux. The browser is allowed where Escouade isn’t, though the proxy may ask you to confirm a warning before it lets the file through. Install it as you did the first time (see [installation](installation)); your settings, projects, and conversations stay where they are.

### What to ask your IT department

Ask them to allow:

- the domains `github.com` (the check and the release’s page) and `release-assets.githubusercontent.com` (where GitHub serves the release files);
- the download of the release files for the client whose User-Agent starts with `Escouade/`: the `.exe` (Windows), the `.app.tar.gz` (macOS, for the update; the `.dmg` is for installing), the `AppImage` and the `.deb` (Linux).

### The proxy setting

If your company gives you the address of its proxy, enter it in Settings › Advanced › Network: the check and the download then go through it. If the proxy decrypts traffic with a company certificate, install that certificate on the system. See [the network settings](settings-network).

## By system

### Windows

The installer runs without a window, and Escouade restarts on the new version.

### macOS

The app is signed with an Apple Developer ID certificate and notarized by Apple (since 1.5.4): macOS opens it without asking for permission. The update replaces the app in place.

The automatic restart, and the installation when the app quits, require that your user can modify Escouade.app and the folder that holds it. Otherwise the update waits for you to click “Restart now”, and macOS may ask for an administrator password.

### Linux

The built-in update replaces the `AppImage` you launched. The `.deb` (Debian, Ubuntu) is first installed with `sudo apt install ./Escouade_*_amd64.deb`; after that, the built-in update downloads the release’s new `.deb` and installs it with `dpkg`: the system then asks for your administrator password (through `pkexec`, otherwise through `sudo`).

## From Claude Code Manager to Escouade

Escouade used to be called “Claude Code Manager”. The update that renamed it replaces the old installation and keeps your settings, projects, and conversations. On Windows, the installer first removes the old installation, keeping your data, then creates Escouade’s shortcuts. The data folder changes name: see [local data](data).

## Reading what changed

- **In the app.** The window of a ready update shows its notes. After the installation, “See what’s new” opens “What’s new in Escouade 1.8.0”.
- **On GitHub.** The [changelog](https://github.com/guillaume-gagnaire/escouade/blob/main/CHANGELOG.md) lists, version by version, what changes. The notes of each release reuse that version’s section.
