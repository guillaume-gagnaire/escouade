# Branches

The status bar’s branch button opens a picker to switch branches, create them, delete them and sync with the remote repository. This page also describes how an agent takes up an existing branch and how to bring the base into an agent’s branch.

## The branch button

In the status bar, at the bottom of the window, the branch button shows the branch of the displayed project’s folder, for example `⎇ main ↑2 ↓5`. It only exists if the folder is a git repository. It is always the branch of the project folder, not that of an agent’s worktree. On a detached HEAD, it says “Detached HEAD”.

| What it shows | What it means |
|---|---|
| `↑2` | 2 commits to push: the branch has 2 commits that its remote branch doesn’t. |
| `↓5` | 5 commits to pull: the remote branch has 5 that the branch doesn’t. The number is amber when it isn’t zero. |
| “not published” | The branch has no remote branch: you never published it. |
| “remote deleted” | The remote branch it was tracking no longer exists. |
| “Pulling…”, “Pushing…” or “Fetching…” | A sync is under way. The button is greyed out until it ends. |

The sync indications only appear if the repository has a remote. The tooltip writes them out in full (“Tracking origin/main: 5 to pull, 2 to push”) and adds the time of the last fetch (“Last fetch: 3 min ago”).

Escouade itself fetches every project that has a remote every 5 minutes, without ever asking for credentials, so that the numbers stay up to date. This automatic fetch can be switched off, and its interval and time limit set, for every project or for one: see [a project’s settings](settings-projects#fetch-automatically-git-fetch).

## The branch picker

Clicking the button opens the picker above it. It lists the repository’s branches in two groups, “Local” then “Remote”, the folder’s branch first, marked with a ✓. Each row shows the name, a tag if any, and the branch’s ahead and behind counts (`↑2 ↓5`).

- The “Search for a branch…” field has the focus when it opens. It ranks branches the way `Ctrl+P` ranks files: what you type can be the start of a name, a piece of it, or just letters in order. A group shows 300 branches at most: beyond that, “N more branches: type to search for them.”
- `↑` and `↓` move, `Enter` picks the branch, `Esc` closes. The Menu key or `Shift+F10` opens the row’s menu.
- A line below the list describes the branch under the cursor: what it tracks (“Tracking origin/feat: 2 to push, 5 to pull”) or why it can’t be picked.

The tags say where a branch comes from:

| Tag | Meaning |
|---|---|
| “used by agent refacto-auth” | That agent’s worktree has this branch: it can neither be picked nor deleted. |
| “in another worktree” | A worktree that belongs to no agent has it. |
| “tracked by feat/login” | A remote branch that a local branch tracks: picking the remote one checks out the local one. |
| “remote deleted” | The remote repository no longer has the branch this one was tracking. |

At the foot of the picker: “New branch…” and “Merged branches…”, then the sync.

### Pull, push, fetch

These buttons only appear if the repository has a remote. They apply to the project folder’s branch. Escouade runs them under your name, so git may ask you for credentials. A message then says what was done (“3 commits pulled”, “2 commits pushed”, “Already up to date”), or why it couldn’t be.

| Button | What it does |
|---|---|
| “Pull” | Fetches, then brings in the new commits of the remote branch, fast-forward only. It is greyed out when the branch tracks nothing or isn’t behind. If the two branches have diverged, it refuses (“… have diverged: a fast-forward pull isn’t possible”): do the rebase or the merge yourself, or ask an agent. It also refuses if uncommitted changes would be overwritten. |
| “Push” | Pushes the branch. It is greyed out when the branch isn’t ahead. If the remote has commits you don’t have, it refuses and tells you to get them first. |
| “Publish” | Replaces “Push” when the branch has no remote branch, or when its own was deleted. It publishes to the remote the branch was tracking, otherwise to `origin`, otherwise to the only remote, and makes the branch track the published one. With several remotes and none called `origin`, it refuses: publish by hand. |
| “Fetch” (“now”) | Fetches the remote without bringing anything in, and says what there is to pull (“Fetch done: 3 commits to pull”). |

Under these buttons, a line says when the repository was last fetched (“Fetched 3 min ago”, the exact time on hover). If the last automatic fetch failed, it says so too, quietly, with its reason and what to do:

| What it says | What to do |
|---|---|
| “it asks for credentials” | Click “Fetch” once: git asks you for them. The automatic fetch does not try again in a loop: it waits for that fetch to succeed, then goes on. |
| “not done within 5 min” | The repository is heavy or the network slow: give it a longer time limit (“Settings” opens the right one). |
| “the network or the remote cannot be reached” | Check your connection, your VPN or your proxy. It tries again at the next interval. |
| “git refused” | git’s reason is on hover. |

On Windows, Git for Windows 2.47.0(1) has an ssh that stalls on big fetches. With it, and only if you did not choose an ssh of your own (`core.sshCommand`, `GIT_SSH` or `GIT_SSH_COMMAND`), the automatic fetch goes through Windows’ own ssh. The line then tells you to update Git (2.47.0(2) or later). Otherwise it goes through your configuration of git as it is (proxy, certificates, `insteadOf`): Escouade adds nothing to it.

## Switching branches

Clicking a row, or `Enter`, changes the project folder’s branch. For a remote branch, Escouade switches to the local branch that tracks it, and creates it, tracking the remote one, if there is none. If you are already on it, the picker just closes.

Escouade checks before it moves the folder, and refuses with a message in these cases:

- an agent without a worktree is working in the project folder (“Agent refacto-auth is working in the project’s folder: wait for the end of its turn.”);
- a merge, cherry-pick, revert or rebase is under way in the folder;
- the folder is on a detached HEAD holding commits that no branch has: leaving would only keep them in the reflog;
- the branch belongs to a worktree: the row is greyed out and says why;
- the switch would overwrite untracked files of the folder: the message names them, move or delete them.

When the folder has uncommitted changes to tracked files, Escouade doesn’t lose them, and asks you: “Switch to “feat/login”?”, “There are uncommitted changes in the project’s folder.” The “Stash and switch” button puts them in a git stash named “escouade: before switching to feat/login” and shows “Changes stashed: “escouade: before switching to feat/login” (git stash).” Escouade doesn’t put them back: take them back later with `git stash pop`. If the switch fails, they are put back where they were.

## Creating a branch

“New branch…”, at the foot of the picker, or “Create a branch here…” in a commit’s menu in the [history](git), opens the “New branch” window.

- “Name”: the branch’s name (`feat/my-branch`). Escouade checks it as you type, by git’s rules, and says in red under the field why a name is refused, for example because it is already taken.
- “Start from”: the starting point. The current branch by default, or another branch of your choice or, when the window comes from the history, the chosen commit (“Commit 4f2a9c1”). On a detached HEAD, it is “The current commit”.
- “and switch to it”: checked by default, it switches the folder to the new branch.
- “Create” (or `Enter` in the field) creates the branch and says “Branch “feat/my-branch” created: you’re on it.” or, without the switch, “Branch “feat/my-branch” created.”

The switch follows the same rules and the same refusals as a branch change, the stash question included. When the branch starts from the commit the folder is on, no file changes: the uncommitted changes come along with you and the question isn’t asked.

## Deleting branches

The menu of a picker row (right-click, Menu key or `Shift+F10`) offers “Delete branch…” for a local branch and “Delete remote branch…” for a remote one. It offers nothing for the folder’s local branch or a worktree’s: delete the agent first, with its worktree, see [one worktree per agent](worktrees).

The project’s **base** is the reference: it is the Kanban’s target branch as long as it exists, otherwise the project folder’s branch.

### A local branch

The confirmation “Delete the branch “feat/login”?” says the branch will be deleted locally. A box “Also delete origin/feat/login”, unchecked by default, appears when the branch tracks a remote branch of the same name. If you check it while that remote branch is the remote’s default branch, the remote branch of the project’s base, or tracked by other local branches too, only the local branch is deleted and a message says why.

A branch whose work is already in the base (merged, or squash-merged) is deleted at once. Otherwise, Escouade asks “Delete “feat/login” anyway?” and says what would be lost: “3 commits are in no other branch.”, or “It isn’t merged into the project’s base, but its commits are in another branch.” “Delete anyway” forces the deletion.

### A remote branch

The confirmation “Delete the remote branch “origin/feat/login”?” warns that it will be deleted from the remote repository, for everyone. Escouade refuses the remote’s default branch and the base’s remote branch. It also refuses a branch you haven’t fetched (fetch first), or one that moved on the remote since your last fetch.

## Cleaning up merged branches

“Merged branches…”, at the foot of the picker, opens the “Merged branches” window: “These local branches are already in the project’s base.” Usually, these are the branches of approved tickets.

- All the branches are checked at first; uncheck those you want to keep. If there are none, the window says “No merged branches to clean up.”
- The base itself, the project folder’s branch and those a worktree holds are never in it.
- A squash-merged branch is in it too: Escouade compares what merging it would change to the base.
- “Delete N branches” deletes them one by one, never forcing. One that can’t be deleted stays in the list with the reason (“feat/login” couldn’t be deleted: …).

## Taking up an existing branch

An agent or a ticket can work on a branch that already exists instead of a new one:

- “New agent on a branch…”, in the menu of the arrow to the right of “New agent”, opens the picker in “Choose a branch” mode: it has neither the foot’s actions nor the row menus, and the branches that can’t be picked are greyed out with the reason. A name typed that no branch has also offers “Create the branch “…” and start an agent on it”, from where a new agent starts (see [worktrees](worktrees));
- in the status bar’s picker, “Start an agent on this branch” is in a row’s menu, except for the project folder’s branch and those a worktree holds;
- the “Branch” field of a ticket’s form offers “Take up an existing branch…”.

In that picker, the project folder’s branch and the Kanban’s target branch are greyed out, with the reason: an agent doesn’t work on them directly. The rest happens in a worktree: see [one worktree per agent](worktrees). Escouade never deletes a branch that was taken up, even once the ticket is approved.

## Integrating the base into an agent’s branch

While an agent works on its branch, the base may move on. “Integrate main” (the name of the agent’s base branch is written in the entry) brings its new commits into the agent’s branch, in its worktree. It is in the menu of a right-click on the agent’s card, and under “Merge …” in the files panel of [Git](git). The entry only exists for an agent that has a worktree and isn’t archived.

Escouade merges the base into the branch, or rebases the branch onto the base when the Kanban’s strategy is “Rebase” (see [the Kanban settings](settings-board)). For the “Merge commit” and “Squash” strategies, it is a merge: there is nothing to squash into a branch.

It refuses, with a message, in these cases:

- the agent is working (“Wait for the end of refacto-auth’s turn to integrate main.”);
- its worktree has uncommitted changes (“Commit or stash the agent’s changes first.”);
- a merge, a rebase or another operation is already under way in its worktree;
- the agent is archived (“refacto-auth is archived: restore it to integrate main.”);
- the base branch no longer exists.

The result appears in a message:

| Message | What happened |
|---|---|
| “main integrated into escouade/refacto-auth.” | The integration succeeded. |
| “escouade/refacto-auth already has everything that is in main.” | There was nothing to integrate: nothing was touched. |
| “main conflicts with escouade/refacto-auth in 1 file: refacto-auth has to resolve it.” | The merge stopped on conflicts. It is left as it stands in the worktree, and the agent gets a message naming the files: it has to resolve them, then commit the merge. |
| “The rebase onto main has conflicts and was undone: refacto-auth has to do it again.” | The rebase is undone, nothing is left half done, and the agent is told to rebase its branch and resolve the conflicts itself. |

> [!TIP]
> Integrate the base before merging: the conflicts get resolved in the agent’s worktree, by the agent that knows its code, and the merge into the base then goes through cleanly. To merge, see [one worktree per agent](worktrees).
