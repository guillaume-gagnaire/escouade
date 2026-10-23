# One worktree per agent

A worktree gives each agent its own folder and its own branch, so that several agents can work on the same repository without getting in each other’s way. This page explains how to turn it on, set it up, merge its work and clean it up.

## What a worktree is for

A git worktree is a second working folder of the same repository, on its own branch. With the “One worktree per agent” option, each new agent gets its own: its changes stay apart, neither the other agents nor your own folder see them until you merge them, and the files panel lists them separately (see [Git](git)).

Without a worktree, all of a project’s agents work in the project folder and their changes get mixed together: Escouade then attributes them to each agent based on its edits.

The agents of [Kanban](board) tickets always work in a worktree of their own, on the ticket’s branch, whether the option is on or not. They follow the same folder, copied-files and command settings as the others.

## Turning it on

The option is set per project, and only affects agents created afterwards: the ones that already exist stay where they are. Three places offer it:

- the “New project” window, with the “One git worktree per agent” switch (off by default);
- a right-click on the project’s tab, then “Turn on a worktree per agent” or “Turn off a worktree per agent” (the entry says “new agents”);
- Settings › Projects, “Git” group, “One worktree per agent”.

The project folder must be on a branch that has at least one commit: a brand-new repository with no commit can’t have a worktree yet. When a worktree can’t be created, the agent works in the project folder and the conversation shows “Worktree not created, the agent works in the project’s folder: …” with the reason.

## Where it goes and what it’s called

Four project settings decide the location and the names (Settings › Projects, “Git” group). The full list of settings is in [the project settings](settings-projects).

| Setting | Default | What it does |
|---|---|---|
| “Worktrees folder” | `.claude/worktrees` | Where new worktrees go: relative to the project folder (`../app-worktrees`, `.worktrees`), to your home folder (`~/worktrees/app`), or a full path. Under the field, “The next one: …” shows where the next one would go. It can’t be the project’s own folder, a folder that holds it, or a folder inside `.git`. Worktrees made already stay where they are. |
| “Agents’ branch prefix” | `escouade/` | A new agent’s branch is called `<prefix><name>`. Empty, it is its name alone. |
| “Tickets’ branch prefix” | `ticket/` | A ticket’s branch, the one its pull request comes from, is called `<prefix><key>`. Empty, it is its key alone. |
| “Files copied into worktrees” | `.env*` | The files that git ignores and that a new worktree takes from the project (see [below](#copied-files)). |

The worktree’s folder is `<worktrees folder>/<agent name>`, on a new branch `<prefix><agent name>`. If either already exists, Escouade adds `-2`, `-3`… By default, a new agent’s worktree waits for its first message, which gives it its name: the branch and the folder take that name (see [below](#at-the-first-message)). With that setting off, an agent is first called `agent-1`, `agent-2`… and gets its worktree at once; when Escouade names it from its first message (for example `refacto-auth`), its branch becomes `escouade/refacto-auth` if that name is free, while the folder keeps its first name. A name you give it yourself changes its branch too, on the same terms, unless the branch is pushed already (it has an upstream, or the remote has a branch of that name): the branch then keeps its name, and a notification says so. The folder never follows the name: for a folder named after the branch, create the agent with “New agent on a branch…” (below). The agent’s card shows the branch without its prefix (`⎇ refacto-auth`), and its full name on hover.

### At the first message

The setting “Create the worktree at the first message, under the agent’s name” (Settings › Projects, “Git” group, and the “Worktrees” step of the assistant) is on by default, for existing projects too. With it, “New agent” creates the agent at once, but with no worktree or branch: its card shows “⎇ at the first message” and its conversation “The worktree will be created at the first message, under the agent’s name.” At the first message:

1. The model of written texts names the agent from that message (for example `refacto-auth`). It has 15 seconds: past that, or if it fails, the agent keeps `agent-N` and nothing waits. A name that comes later renames the branch, as with the setting off.
2. The worktree is created: the branch `<prefix><name>` and the folder `<name>`, with `-2`, `-3`… if either is taken.
3. The files are copied and the setup commands run in that folder.
4. The message goes once the setup is done.

Meanwhile, the conversation shows the current step: “Naming the agent”, “Creating the worktree refacto-auth”, then the setup commands. Before the first message, the agent’s terminal and “Open in editor” wait: there is no folder to open yet.

These are not concerned, and get their worktree at once as before: tickets’ agents, an agent you give a name when you create it, an agent created with “New agent on a branch…”, a copy of an agent, and an agent in the Claude Code view (its interface runs from its creation). Turn the setting off for a project whose setup is long, if you’d rather have it run while you write the first message.

The worktree starts from the commit the project folder is on at that moment. That folder’s branch becomes the agent’s **base branch**: it is the one that the merge and “Integrate” aim at.

When the worktrees folder is inside the repository (the default), Escouade adds it to the repository’s `.git/info/exclude` file: the worktrees don’t show up in the project’s changes. A folder that already holds files git tracks is refused: “The worktrees folder … holds files git tracks: pick another one in the project’s settings.”

## Copied files

A file like `.env`, which git ignores, doesn’t exist in a fresh worktree. Right after the worktree is created, Escouade copies into it the project’s files that git ignores and that match a pattern of the “Files copied into worktrees” setting.

- One pattern per line, with `*` and `?` inside a name and `**` to cross folders: `.env*` takes the files at the root, `**/.env*` those of every folder. The default is `.env*`.
- Only files git ignores are copied, so none of them can end up in a commit. A file that git tracks is never copied.
- The copies show up in the file tree of the [editor](editor), greyed out (“Ignored by git”). A direct commit never takes them.
- If a copy fails, the conversation says so (“Files not copied into the worktree: …”) and the agent works without that file.

## Setting up the worktree

A fresh worktree has no dependencies installed and no generated code. The “Worktrees” group of the project settings lists the commands Escouade runs to set it up, in the “When a worktree opens” list: they run in order, in the new worktree, before the agent’s first message, and its messages wait for them to finish.

Each command has a shell (the first shell detected by default), a command line (for example `npm ci`) and a “Subfolder” of the worktree (blank, it is the root). “+ Add a command” adds one, the × deletes it. To change the order, use the command’s ↑ and ↓ buttons, or `Alt+↑` and `Alt+↓` (`⌥↑` and `⌥↓` on macOS) from any field of the command. An empty command is skipped.

The commands get these environment variables:

| Variable | Value |
|---|---|
| `ESCOUADE_PROJECT_DIR` | The project folder. |
| `ESCOUADE_WORKTREE_DIR` | The worktree’s folder. |
| `ESCOUADE_BRANCH` | The worktree’s branch. |
| `ESCOUADE_PORT_BASE`, `ESCOUADE_PORT_END` | The ports reserved for a ticket’s worktree, when it has some (see [testing](testing)). |

The proxy of the “HTTP(S) proxy” setting is passed to the commands when “Also apply the proxy to the built-in terminals” is on (see [the network settings](settings-network)). A command that runs past 20 minutes is stopped.

### Following the setup

- The agent’s card says “Setting up…”; its tooltip gives the command in progress.
- The conversation shows “Setting up the worktree · npm ci — your messages will be sent once it’s done.” The “Show output” button unfolds the last lines of what the command writes, live. The output follows the new lines while you are at the bottom, and stays put if you scroll up.
- A message you send during the setup waits: it goes out once the setup is finished.
- At the end, the conversation notes “Worktree set up (3 commands, 12 s).”
- The full output of the last setup, command after command, is written to `~/.escouade/logs/`, one file per agent named `<agent identifier>-setup.log`. It is replaced at each setup and deleted with the agent.

A setup command can use the name of the worktree’s folder, for example for a local address (`<folder>.localhost`) or a database name: with the setting [“Create the worktree at the first message, under the agent’s name”](#at-the-first-message), that name is the agent’s, not `agent-N`. The folder no longer changes afterwards, even if the agent is renamed.

Archiving or deleting the agent during its setup stops it.

### When a command fails

The first command that fails stops the setup: the following ones don’t run. The conversation shows a warning, for example “The worktree setup failed on `npm ci` (code 1).”, followed by the last 30 lines of the output and the path of the full output (“Full output: …”). The failure can also come from a subfolder that doesn’t exist in the worktree, from a command that doesn’t start, or from a command stopped after 20 minutes.

The agent is told in its first message, with the command that failed and its last lines, and it starts all the same: it can take it into account.

### Fill in automatically

“✦ Fill in automatically”, at the top of the “Worktrees” group, asks Claude to read the project (manifests, lockfiles, README…) and suggest the commands. It changes nothing: it only has reading tools, and it has 4 minutes at most. Its suggestion appears apart, under “Suggested worktree commands”, with each command in full, its shell and its folder:

- “Replace the commands” replaces both lists of the draft (setup and teardown). Nothing is kept until you click “Save” in the settings window;
- “Ignore” drops the suggestion.

Claude suggests eight commands at most per list. A command on several lines, with invisible characters, with a run of 24 spaces or more, or of over 300 characters is left out, and the message that follows the reading says so (“2 commands suggested, 1 left out (invisible characters or too long): review them before saving.”).

> [!WARNING]
> Once saved, these commands run on their own in every new worktree. Read each one in full before saving.

## Tearing down the worktree

The “Before a worktree is removed” list holds what must be undone outside the worktree before deleting it: a database or containers that the setup created for it. Most often, it is empty. It is edited like the setup list, and is also filled by “✦ Fill in automatically”.

These commands run when an agent is deleted with its worktree, when the Kanban cleans up the worktree of a done or deleted ticket, and with “Clean up”. Each one runs even if the previous one failed, with 5 minutes at most, and their output isn’t shown. If the project uses isola (an `.isola.toml` file), `isola destroy` runs afterwards. A problem is reported once the agent is deleted: “Agent deleted, but the worktree teardown failed on …”.

## Working on a branch you choose

An agent can also work in a worktree on a branch you choose, one that already exists or one you create for it:

- “New agent on a branch…” in the menu of the ▾ button, to the right of “New agent”, opens the branch picker (“Choose a branch”); a name typed that no branch has offers “Create the branch “feat/signup” and start an agent on it”;
- in the status bar’s branch picker, a right-click on a branch offers “Start an agent on this branch”;
- in a ticket’s form, the “Branch” field offers “Take up an existing branch…”.

A remote branch is taken up through the local branch that tracks it, created if there is none. The project folder isn’t touched. Three branches are refused, with the reason: the project folder’s branch, the Kanban’s target branch (tickets are merged into it), and one that another worktree already holds.

The worktree’s folder is named after the branch, its last part: `feat/login` gives `.claude/worktrees/login`, then `login-2` if that folder is taken. A setup script can use it for a local address (`login.localhost`) or a database name, and so tell which instance runs on which branch. The name keeps the branch as it is, capitals, `_` and dots included; `ESCOUADE_BRANCH` gives the whole branch.

### Creating the branch from the picker

Type the new branch’s name in the picker of “New agent on a branch…”. If no branch, local or remote, has that name, the last row offers “Create the branch “feat/signup” and start an agent on it”, followed by where it starts (“from main”). The branch starts where a new agent does: from the Kanban’s target branch if “Agents start from the target branch” is on (see [the Kanban settings](settings-board)), otherwise from the project folder’s branch. The line under the list says so when the cursor is on the entry.

The name is checked the way git does. A name git refuses greys the entry out, with the reason, and nothing is created. A name that starts like a branch of a remote (`origin/…`) is refused too.

A branch created this way belongs to the agent, like a new agent’s: deleting the agent with its worktree deletes it too. It always keeps the name you gave it, even when the agent is named or renamed.

### A branch that was already there

A branch that was there before the agent is never created or deleted by Escouade: when you delete the agent, the box offers “Also delete the worktree (the branch … is kept)”. Its base branch is the Kanban’s target branch if it exists, otherwise the project folder’s branch. For a ticket, the entry’s tooltip says “Escouade never deletes this branch, even once the ticket is approved.” Branches are covered in [the branches page](branches).

## Merging into the base branch

When an agent’s work suits you, you merge it into its base branch from the files panel:

1. The agent must have committed its work (see [Git](git)). The merge only takes commits.
2. In the panel, “Uncommitted” tab, “This agent” button, click “Merge escouade/refacto-auth → main…”.
3. The window asks “Merge escouade/refacto-auth into main?” and says the agent’s commits are merged into the project’s “main” branch. The “Squash (a single commit)” box is unchecked by default. Click “Merge”.

The merge happens in the project folder.

| Choice | Result |
|---|---|
| Without “Squash” | A merge commit (`Merge branch 'escouade/refacto-auth'`) that keeps all the agent’s commits. |
| With “Squash (a single commit)” | A single commit, named after the agent, that lists the subjects of the commits it groups. |

Escouade refuses the merge, with a message, in these cases:

- the agent has uncommitted files (“The agent has 2 uncommitted files: ask it to commit before merging.”);
- the project folder has uncommitted changes to tracked files: commit them or stash them;
- the agent’s branch has no new commit (“Nothing to merge”);
- the base branch no longer exists.

If the project folder isn’t on the base branch, a question appears: “Switch to “main”?” It says which branch the project is on. “Switch and merge” switches the folder to the base, then merges.

A merge with conflicts is aborted: the project folder stays as it was and an error message appears. In that case, first bring the base into the agent’s branch with “Integrate main”, which hands the conflicts to the agent (see [branches](branches)), then start again.

The merge removes neither the agent’s worktree nor its branch. To tidy them up, delete the agent with its worktree (see below); to clean up the branches already merged in one go, use “Merged branches” in [the branch picker](branches). For Kanban tickets, the approval takes care of the commit, the merge or the pull request according to the settings (see [the Kanban settings](settings-board)).

## Opening a pull request

Rather than merging yourself, you can offer an agent’s work as a pull request, on GitHub for now (GitLab, Bitbucket and Gitea will follow). The project needs its remote on the code host and an account connected in [Settings › Integrations](settings-integrations); otherwise nothing offers it.

“Open a PR” is in the conversation’s header, in the menu of a right click on the agent’s card, and in the menu of its branch in the [branch picker](branches). The window proposes a title and a description, written by the model for written texts from the branch’s commits and diff: read them, edit them, pick the target branch and a draft if you want. Nothing goes out before “Push and open”, which pushes the branch then opens the pull request. The worktree’s uncommitted files are not in it, and the window says so.

The agent’s card then shows its pull requests under its lines, three at most, the ones to merge first: their number opens the pull request in the browser, and its tooltip tells its state, its CI and its reviews. Escouade reads them again when you show the agent, then every minute while it is shown, and no more once merged. A pull request someone else opened on the same branch counts too.

An agent without an Escouade worktree that works in a worktree of its own, made by Claude Code or one it went into with a `cd`, has its pull requests too: Escouade follows the branch of the folder its session works in, and its header shows it. Never the one of the project’s folder, nor another agent’s worktree.

When the CI fails, “Agent fixes it” sends the agent the failed checks with the end of their logs. When a review asks for changes or leaves open threads, “Agent addresses the feedback” sends it the requests and the comments, each with its file and line. These messages only go when you click: the agent fixes, commits and pushes, and the pull request updates. It doesn’t reply under the threads for you.

Once a pull request is open, the agent’s branch keeps its name, even if the agent is renamed later.

## Archiving, restoring, deleting

The actions are in the menu of a right-click on the agent’s card.

- “Archive” stops the agent and keeps its conversation. Its worktree and its branch stay on disk (the Kanban cleans up that of a finished ticket, see below). The confirmation only appears for an agent whose ticket is in progress or to review.
- “Restore” brings an archived agent back. If its worktree folder has disappeared in the meantime, for example after a pull request or a push, Escouade recreates it from its branch, copies the project’s files again and runs the setup again. If another worktree has taken its folder, its own is recreated beside it (`-2`) and the conversation says so. If the branch no longer exists (merged then deleted), the agent stays as it is and its conversation says so.
- “Delete…” asks for confirmation. The box “Also delete the worktree and the branch escouade/refacto-auth” is checked by default. Checked, Escouade stops the agent, runs the teardown commands, removes the worktree’s folder, then the branch. Unchecked, the agent disappears from Escouade and the worktree and the branch stay on disk. If files of that worktree are open and unsaved in the editor, the window warns that they will be lost.

> [!WARNING]
> The agent’s branch is deleted even if it hasn’t been merged: its commits then exist nowhere else. Merge it first if you want to keep its work.

If the cleanup runs into a problem (a process still holding the folder, a teardown that fails), the agent is deleted all the same and a message reports it afterwards. Closing a project doesn’t touch its worktrees on disk.

## Leftover worktrees

The worktree of a done or deleted ticket goes on its own (the “Delete the worktree of a done or deleted ticket” setting of the [“Kanban”](settings-board) tab). For other archived agents, or what is left from before, Settings › Projects › “Leftover worktrees” › “Clean up” deletes the worktrees of the project’s archived agents, after their teardown commands, and forgets those whose folder is already gone (`git worktree prune`).

No work is lost there: a worktree with uncommitted changes, or commits that are neither in the target branch nor in another branch (pushed, proposed as a PR), is kept. The row says how many worktrees were deleted, then those it kept and why; the archived agent’s card says so too. The branch of a deleted worktree goes with it if its work is in the target branch; otherwise it stays, and “Restore” recreates the worktree from it.

## Duplicating an agent

“Duplicate the conversation” (the agent’s menu) copies an agent that has a worktree into a worktree of its own, created from the commit the original is on, on a new branch whose name builds on the copy’s. The original’s uncommitted changes aren’t in it, and the conversation says so. See [agents](managing-agents).
