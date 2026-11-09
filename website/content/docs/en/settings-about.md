# Settings: About

The “About” page shows Escouade’s version, the harnesses detected, the credits and the links. Since 2.2, checking for updates, installing them automatically and the usage statistics are on the “General” page, and the local data in “Advanced”: this page describes them in their new place.

## Open the page

Open the settings with `Ctrl+,` (`⌘,` on macOS), then choose “About”, at the bottom of the bar. The version is written there after the word “Escouade”, whichever page is open. These are the app’s settings, shared by all projects, and they apply as you set them.

## Escouade

On the “General” page, the “Updates” group, with nine lines, plus that of an update found, under “Install updates automatically”, until it is installed (see [“Check for updates”](#check-for-updates)).

| Line | What it shows or does | Default |
|---|---|---|
| “Escouade” followed by the version | The installed version, followed by “(beta)” for a beta. Below it, the harnesses Escouade finds on your machine (“Harnesses detected: Claude Code, Codex”), or “No harness found”. On the right, the “Check for updates” button. | — |
| “Install updates automatically” | “Escouade restarts itself when no agent is working and everything is saved.” | On |
| “Receive beta versions” | “Betas come before everyone else’s update and may have flaws. Unticked, Escouade stays on its beta until the stable version that follows it.” Saved at once; ticked, it starts a check (see [“Beta versions”](updates#beta-versions)). | Off |
| “Usage statistics” | “The features used, the version of Escouade and the system, under an id drawn at random: nothing that identifies you, nor your projects or your code.” On the right, the “In detail” button (it opens the [Privacy](/en/privacy/) page) and a switch. | Off until you accept the first-start question |
| “Local data” | The folder where Escouade keeps its data: `~/.escouade/`. | — |
| “Credits and licenses” | The components and voices Escouade ships, with their licenses. The “Open” button shows the list in your browser. | — |
| “Website” | “escouade.dev: the documentation, what’s new and the roadmap”. The “Open” button opens the site in your browser, in the language of the interface. | — |
| “Discord” | “Ask a question, report a problem, follow what’s coming”. The “Join” button opens the invitation to Escouade’s Discord server. | — |
| “Support Escouade” | “Escouade is free and open source: a coffee helps it move forward”. The “Buy a coffee” button opens the [payment page](support:) in your browser, for the amount you choose (see [“Support Escouade”](introduction#support-escouade)). | — |

If an installed harness isn’t found, give its path on the [“Accounts and harnesses”](settings-accounts) page.

## Check for updates

Escouade looks on its own: once a few seconds after it starts, then five minutes after each check ends. The “Check for updates” button (it reads “Checking…” during the call) starts a check right away. With no newer version, a message says “No update available.”; if the check fails, an error message gives the reason. Only a check started with this button shows it right away: an automatic check waits for several failures in a row (see [“When it fails”](updates#when-it-fails)).

A version found, by the button or by the automatic check, is downloaded right away and its signature is checked, then it waits. The “General” page shows it under “Install updates automatically”: “Escouade 1.8.0 available” with “Downloading…”, then “Escouade 1.8.0 ready” with the “Restart now” button and the “See what’s new” link, which unfolds its notes. During the countdown of an automatic restart, the line shows it too (“Restarting in 20 s”) with a “Later” button. If the download fails, the “Update 1.8.0” line takes its place and gives the reason (“Download failed: …”), with “Download in the browser” when it can; a newer version found later takes the place back.

When it’s ready, the status bar also shows a button, for example “Update 1.8.0 ready · Restart”. It, like the tab’s “Restart now”, opens a window, for example “Escouade 1.8.0 is ready”, which shows the version’s notes. “Restart now” installs the update. It’s grayed out while a file in the editor isn’t saved (the window tells you), and the window warns you when agents are working, since the restart interrupts their turn. “Later” closes the window without installing anything (opened from “General”, it takes you back there).

Checking and downloading go through the proxy of [“Advanced › Network”](settings-network). The whole flow is in [updates](updates).

## Install updates automatically

On by default. A downloaded update then restarts Escouade by itself, as soon as the app is at rest. Off, the update stays ready: you start it with “Restart” in the status bar, or it installs when you close the app (which then stays closed).

Escouade is at rest when none of this is true:

- an agent is in the middle of a turn, or is waiting for your answer or a permission;
- a worktree is being set up;
- a ticket is being approved, or other work is under way (starting a ticket, cleaning up…);
- a launch command or a step of a test launch is running, or a test launch is being prepared;
- a built-in terminal has received or produced text in the last 5 minutes;
- a file in the editor isn’t saved;
- a window is open over the app;
- you’ve used the keyboard or the mouse in the window in the last 5 minutes (a hidden or minimized window doesn’t count).

Once it’s at rest, Escouade warns you 30 seconds ahead: a system notification (not on Linux), even if system notifications are off on the [“Notifications”](settings-notifications) page, says “Escouade is restarting to update” with a “Later” button, and the status bar counts down (“Restarting in 20 s”). “Later” postpones the restart by 30 minutes. It is in the status bar, next to the countdown, and in the notification on Windows only (the macOS notification has no button). If, during those 30 seconds, you pick the app back up or an agent starts working again, the restart is canceled.

If a version couldn’t install, Escouade no longer restarts by itself for it: you can try again by hand.

> [!NOTE]
> On macOS, the automatic restart only happens if Escouade can replace its app without an administrator password. Otherwise, restart it yourself when the update is ready.

## Usage statistics

Escouade can send usage statistics, with nothing that identifies you: the features used, the version of Escouade and the system, under an id drawn at random. They go to a server of Escouade’s hosted in France (Aptabase, open source), and to no third party.

They are off until you accept. At the first start, including on an install that existed before this feature, a “Help improve Escouade?” dialog asks: “Share statistics” or “No thanks”. Nothing is sent before you answer. If you close the dialog without answering (with `Esc`), it comes back at the next start.

The line’s “In detail” button, like the dialog’s “What is sent, in detail” link, opens the [Privacy](/en/privacy/) page: the events sent and their props, what is never sent, how long it is kept (25 months), the legal basis and your rights.

When you turn the switch off, the install id is forgotten, its `telemetry-id` file is deleted, and events still waiting are dropped. If you turn it on again, a new id is drawn: the install counts as a new one. An instance started with a separate data folder (see [local data](data)) never sends anything.

## Local data

The “Local data” line shows `~/.escouade/`, in your home folder (`C:\Users\<you>\.escouade` on Windows). The page only shows where the folder is: it has no button to open it, so open it with your file manager. What it holds is described in [local data](data).
