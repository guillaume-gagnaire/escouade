# First steps

What Escouade does at its first launch: seven stops, one question each, from the harness to your first agent answering, in about two minutes if your harness is already signed in. You can skip anything and come back.

## The journey

At the first start of a new install, Escouade opens the “First launch” in place of the projects rail and the workspace. On the left, the line of stops, drawn like the git graph: full when the stop is done, teal for the current one (“now”), hollow when it was skipped (“skipped · to resume”), dotted for those to come. Each stop is a button that takes you there. On the right, the stop’s question, why it is asked and its choices, the recommended one already selected.

![The first stop of the first launch: the line of stops on the left, the question “Which harness will your agents run on?” and Claude Code signed in, recommended](/images/en/first-launch-harness.jpg)

The journey sets nothing its own way: each stop writes what the settings write, and a stop is done as soon as what it sets is set, wherever that happens (an account added in Settings ticks the account stop). It never opens by itself on a device already set up, even after an update.

| Stop | The question | What it sets |
|---|---|---|
| 1. Harness | “Which harness will your agents run on?” | The harness at the top of the harness order, its plugin (from the app, offline) and, if it is signed in on the machine, its account |
| 2. Account | “Which account for Claude Code?” | “This machine’s account” or “A separate account” (see [several accounts](accounts)); folded into the first stop when the harness is already signed in |
| 3. Starting settings | “When should your agents ask you?” | The mode of new agents: “Accept edits” (recommended), “Ask”, “Plan” or “Auto”; the model and the effort are your account’s |
| 4. What to plug in | “What would you like to plug in?” | Nothing ticked in advance: a ticket source, voice, a code host |
| 5. First project | “Where will your first agent work?” | A folder on your machine (recommended), a repository to clone, or a demo project |
| 6. First agent | “Your first message” | An agent made in that project, a suggested message you send yourself |
| 7. Statistics | “Help improve Escouade?” | “No thanks” or “Share statistics” |

## From the keyboard

The whole journey works from the keyboard: `↑` and `↓` move between choices (from the question, they take you to the choices), `Space` takes the focused choice, `Enter` does the main button (“Continue”, “Keep Accept edits”, “Open project”…), `Esc` does “Later”. The focus lands on the question at each stop, and a screen reader hears the current stop in the line. In the first agent’s conversation, the keys stay its own: `Esc` stops the agent there.

## The starting settings

The third stop chooses when your agents ask for your consent. “Accept edits” is recommended: file edits go through without asking (they stay in the agent’s worktree, visible in its diff) and every command asks you. Bypass is shown, off, and never offered as a starting mode: turn it on in Settings › Agents (see [permission modes](managing-agents#permission-modes)).

![The stop “When should your agents ask you?” in the light theme: the four modes, Accept edits recommended, and Bypass off with its reason](/images/en/first-launch-defaults.jpg)

## What to plug in

Three cards, none ticked: “A ticket source” (Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues, Azure Boards), “Voice” (Whisper on your machine, its size said before the download, or an online service with your key) and “A code host” (for PRs and CI). Ticking a card opens its choices; only what you choose installs its plugin, and it connects through the same form as in the settings (see [integrations](integrations) and [code hosts](code-hosts)). What you skip waits for you in Help › Discover and in Settings.

## The first project and the first agent

A project is a folder, with its agents, its board and its docs; each agent works in its own [worktree](worktrees), on its own branch. The stop offers a folder on your machine, a repository to clone by its URL (with your usual git access), or “A demo project”: a small throwaway repository the app writes in `~/.escouade/demo/bac-a-sable`, a to-do list in TypeScript with two tests that fail on purpose, offline. A band at the top of the demo project says so, and “Delete” erases it with its folder.

The next stop makes your first agent in that project and shows its real conversation on the right. Three messages are suggested (“Explain this repository in five lines.”, “Run the tests and tell me what breaks.” when the project has a test command that can run here, “Find a small bug and suggest a fix.”): the one you pick is put in the field, editable, and nothing goes out until you click “Send”. It goes out on your account and counts toward its quota. Its first permission request is explained beside the real card (`Ctrl+Enter` allows, `Ctrl+Shift+Enter` always allows; on macOS, `Ctrl` becomes `⌘`). Once the turn is over, “See the diff” shows what it changed in its worktree: your repository hasn’t changed.

![The first agent’s stop: “Your agent has answered”, its conversation on the demo project on the right, its diff of one file](/images/en/first-launch-agent.jpg)

A turn cut short says so and lets you resume: an account at its usage limit (“Add an account…”), offline (“Put the message back”), a harness signed out (“Back to the account stop”).

## Statistics

The last stop asks about usage statistics: “No thanks” and “Share statistics” weigh the same. Nothing is sent before you answer, and what happened during the journey (the stops done or skipped, its length by bracket) is only sent with your consent, without any name, path or content. Skipped, the question comes back once at the next start. The detail is on the [Privacy](/en/privacy/) page.

## After the journey

“Escouade is ready”: “Get started” takes you to your first project, “See what there is to discover” opens the Help › Discover map. The rest of Escouade is discovered there by doing it for real, one thing at a time: each discovery says what it needs first (a download, a token, quota), “Try it” takes you to the real control, and it ticks itself when the app sees it done. A dot on the rail’s “Help” icon says some are left; “Hide all” removes it.

![The “Discover” map in the light theme: the trunk of what is installed and the five lanes Plan, Code, Review, Ship and Run](/images/en/first-launch-discover.jpg)

## Resume or redo the journey

- **Closed half-way** (the app quit, a power cut), it opens again at the next start, at the first stop neither done nor skipped.
- **“Skip all”** skips every stop left and closes the journey; it never comes back by itself.
- **Redo**: Help › First launch, or Settings › General › “Redo the first launch”, opens it again on any device, from the first stop, with the value in place on the line. Nothing is reset: no account or project is removed. “Close” closes it.

## Without a harness, or on a managed device

- **No harness found**: the first stop gives each one’s install command, “Copy” puts it on the clipboard, and Escouade finds it as soon as you come back to the window (or with “Search again”). See [harnesses](harness).
- **A managed device**: what a policy imposes is shown locked, with who imposes it (“Imposed by this device’s policy”). A harness not allowed is greyed out, a single one allowed is said (“Claude Code is the only harness allowed on this device.”), a forbidden mode can’t be taken. See [managed policies](managed-policies).
- **A device set up before**: the journey doesn’t open; only the dot on “Help” invites you to discover what’s new.
