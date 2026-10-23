# Settings: Kanban

The “Kanban” tab of the settings decides, project by project, what happens to an approved ticket, the tests run before approval, the autopilot, and the agents that take the tickets.

## Open the tab

Open the settings with `Ctrl+,` (`⌘,` on macOS), then choose “Kanban”. From the [Kanban](board), the ⚙ “After approval: …” button in the header opens this tab directly, on the project shown.

The “Project” bar at the top of the window lists your projects: what you change applies only to the selected one. Only “Pause above quota” is shared by all projects. Your changes stay in a draft until you click “Save”; “Cancel” drops them, and a dot to the right of the tab’s name marks what isn’t saved yet. With no project open, the tab shows “No project open.”.

## When I approve a ticket in “To review”

This first group says what approving a ticket in “To review” does (see [the Kanban](board)). The approve button takes the name of the choice, and the Kanban header sums it up, for example “merge squash → main”.

| Choice | What approving does | Button |
|---|---|---|
| “Merge into a branch” (default) | Merges the agent’s worktree into the target branch, then frees the agent. | “Approve and merge” |
| “Open a pull request” | Pushes `ticket/<key>` and opens a PR to the target branch for review. | “Approve + PR” |
| “Push the ticket’s branch” | Commits and pushes to `ticket/<key>`, with no merge or PR. | “Approve and push” |
| “Leave as is” | The changes stay uncommitted in the worktree. | “Approve” |

`ticket/` is the project’s “Tickets’ branch prefix”, set in the [“Projects”](settings-projects) tab (`ticket/` by default); `<key>` is the ticket’s key, for example `ATL-42`.

With the first three choices, Escouade first commits what the agent left in the worktree, then does the action. The ticket then moves to “Done” and its agent is archived. If the branch brings no change, the ticket is finished with nothing merged or pushed. With “Leave as is”, the ticket also moves to “Done”; the agent and its worktree stay as they are, unless the worktree holds nothing unsaved (see “Delete the worktree of a done or deleted ticket”, below).

### Target branch

Shown with “Merge into a branch” and “Open a pull request”. The ⎇ button opens the branch picker, with its search: it offers the project’s local branches, without those of the agents and the tickets (`escouade/…`, `ticket/…`). It’s the branch tickets are merged into, and also the base of each ticket’s branch and of its PR. By default it’s the branch checked out in the project’s folder when the first ticket starts; Escouade keeps it from then on. If the branch doesn’t exist, approval stops with an error.

The merge happens in the project’s folder when the target branch is checked out there (it’s refused if tracked files have uncommitted changes), otherwise in a temporary worktree. It’s refused if the target branch is checked out in an agent’s worktree. See [worktrees](worktrees).

For the new agents to start from the target branch, set “Starting branch of new worktrees” to “Board’s target branch” in [the project settings](settings-projects): that setting replaced this tab’s “Agents start from the target branch” (see [the starting branch](worktrees#the-starting-branch)).

### Strategy

Shown with “Merge into a branch” only. Default: “Squash”.

| Strategy | What happens to the target branch |
|---|---|
| “Merge commit” | A merge commit, with git’s message; the ticket’s commits stay in the history. |
| “Squash” | A single commit, with the generated message (see “Generated commit message” below). |
| “Rebase” | The ticket’s branch is rebased onto the target branch, then the target branch fast-forwards. |

A conflict cancels the merge: see “On conflict” below.

### Draft PR

Shown with “Open a pull request” only. Off by default. When on, the PR opens as a draft.

Escouade pushes the branch, then creates the PR with the GitHub CLI `gh` if it’s installed. Without `gh`, or if `gh` fails, it opens GitHub’s page for finishing the PR in your browser; this setting doesn’t apply then. If the repository isn’t on GitHub, the branch is only pushed.

## Before and after

This group sets what happens before the action chosen above, and after it.

| Setting | Effect | Default |
|---|---|---|
| “Test command” | The command run in the ticket’s worktree, for example `npm test`. It’s stopped after 20 minutes. | Empty |
| “Re-run the tests first” | Runs the test command before the action. If a test fails, the action is blocked and the ticket goes back to the agent with the end of the test output. The setting is grayed out while the test command is empty. | Off |
| “Delete the worktree of a done or deleted ticket” | Frees disk space: the worktree of a done ticket, whatever the approval, and that of a deleted ticket, in any column, are deleted after their teardown commands, along with those of the agents the ticket had before. A worktree with uncommitted changes, or commits neither merged nor pushed, is kept, and you’re told (see below). Merged or empty, the ticket’s branch goes too; if pushed or proposed as a PR, it stays, as does any branch the ticket took up. Off, worktrees stay. | On |
| “Generated commit message” | On, Haiku writes the message in the Conventional Commits form; if its answer isn’t usable, the message is `feat: <title> [<key>]`. Off, the message is `<key> <title>`. An example appears under the setting, with your project’s key. | On |
| “On conflict” | What a merge that hits a conflict does. See below. | “Ask me” |

Escouade never loses work when cleaning up: a worktree is deleted only if all its files are committed and every commit of its branch is in the target branch or in another branch, local or remote. Otherwise it stays:

- with “Leave as is”, the ticket’s outcome says so, for example “◇ Left in the worktree · worktree kept: 2 uncommitted file(s)”, and the agent stays;
- for a deleted ticket, its archived agent’s card says so, under its name, in the “Archived” list.

Restoring that agent puts it back to work in the same worktree. To tidy up what is left later, use “Clean up” in the [“Projects”](settings-projects) tab.

The commit message is used to commit what the agent left in the worktree, and as the squash’s message; a merge commit keeps git’s message. Its language follows “Language of texts written by the agents” (in the [“Application”](settings-application) tab).

### On conflict

A conflict can only happen during a merge. In every case the merge is canceled first.

| Choice | What happens |
|---|---|
| “Ask me” (default) | The ticket is blocked with the list of conflicting files, and its card offers “Let the agent resolve” and “Cancel”. |
| “Let the agent resolve” | Escouade hands over to the agent right away: the target branch is merged into the ticket’s branch (or, with the “Rebase” strategy, the agent is asked to rebase), and the ticket goes back to “In progress” at loop 1. |
| “Cancel” | The ticket is blocked with the list of conflicting files, with no button to resolve. |

## Autopilot

See [the autopilot](autopilot).

| Setting | Effect | Default |
|---|---|---|
| “Assign tickets automatically” | A free agent takes the next “To do” ticket. It’s the same switch as “Autopilot” in the Kanban header, which saves there right away. Off, a ticket only starts when you click “Start”. | On |
| “Pause above quota” | Choose 80%, 90%, 95%, or 100%. No ticket starts while the 5-hour window or the weekly window has reached this threshold. Shared by all projects (“for all projects”). | 100% |

When the pause is in effect, a strip under the Kanban header says why nothing starts and offers “Resume now”, which lifts the pause until the end of the window concerned.

With several Claude accounts, the pause holds only if every account the project can use is held back, and the same threshold is used to choose the account new agents start on. See [accounts](accounts) and [quota tracking](usage).

> [!NOTE]
> The setting “Resume a ticket’s agent stopped by the usage limit on another account” isn’t in this tab. It’s in the [“Harness”](settings-claude-code) tab, and only appears with several accounts. It’s off by default.

## Agents

The agents that take this project’s tickets. See [the Kanban](board).

| Setting | Effect | Default |
|---|---|---|
| “In parallel” | The number of tickets “In progress” at once: 1, 2, 3, 4, 5, or 6. Beyond that, “To do” tickets wait for a free slot. | 2 |
| “Model” | The model of the ticket agents: “Default (…)”, which follows the default model of the “Harness” tab, or Fable, Opus, Sonnet, Haiku. | “Default” |
| “Effort” | “Default”, or Low, Medium, High, Very high, Max. | “Default” |
| “Mode” | “Default”, or the permission mode: Auto, Ask, Plan, Accept edits, Bypass. | “Default” |

“Default” means the matching choice of the account that takes the ticket, on its card in [Settings › Accounts](settings-accounts) (“New agents”): the menu names it after the account a new agent of the project would go to. Until you change them, those are its harness’s: Sonnet, Medium, and Auto for Claude Code.

Permission modes are explained in [agents](managing-agents). In short:

- “Auto”: a classifier approves safe actions and asks about the rest.
- “Ask”: Claude asks for your approval before each sensitive action.
- “Plan”: Claude analyzes and proposes a plan without changing anything.
- “Accept edits”: file edits are accepted without asking.
- “Bypass”: no permission requests, only for safe environments.
