import { defineZone } from '../types';

// The branches: the picker of the status bar, switching, creating, deleting and cleaning up.
export default defineZone('branches', {
  /** The picker the status bar's branch button opens. */
  picker: {
    title: 'Branches',
    searchLabel: 'Search for a branch',
    searchPlaceholder: 'Search for a branch…',
    local: 'Local',
    remote: 'Remote',
    /** The branch button when no branch is checked out. */
    detached: 'Detached HEAD',
    /** The tooltip of the branch button when the repository has no remote to sync with. */
    switchTitle: 'Switch branch',
    empty: 'No branches.',
    noMatch: 'No branch matches.',
    /** `{count}`: branches left out of the list to keep it short, which the search finds. */
    more: {
      one: '{count} more branch: type to search for it.',
      other: '{count} more branches: type to search for them.',
    },
    switching: 'Switching branch…',
    /** The branch the project's folder is on, read to a screen reader. */
    current: 'Branch of the project’s folder',
    /** `{agent}`: the agent whose worktree holds the branch. */
    usedBy: 'used by agent {agent}',
    /** `{branch}`, `{agent}`: also what the toast says when the backend refuses because of it. */
    usedByWhy: 'The branch “{branch}” is used by agent {agent}, in its worktree.',
    otherWorktree: 'in another worktree',
    /** `{path}`: the folder of a worktree that belongs to no agent. */
    otherWorktreeWhy: 'The branch “{branch}” is taken by the worktree {path}.',
    /** `{branch}`: the local branch that already tracks a remote branch. */
    trackedBy: 'tracked by {branch}',
    trackedByWhy: 'Tracked by the local branch “{branch}”: that’s the one that gets checked out.',
    gone: 'remote deleted',
    /** `{upstream}`: the remote branch a local one tracks. */
    goneWhy: 'The tracked branch {upstream} no longer exists on the remote repository.',
    tracks: 'Tracking {upstream}: {ahead} to push, {behind} to pull',
    ahead: { one: '{count} commit to push', other: '{count} commits to push' },
    behind: { one: '{count} commit to pull', other: '{count} commits to pull' },
    newBranch: 'New branch…',
    mergedBranches: 'Merged branches…',
    deleteLocal: 'Delete branch…',
    deleteRemote: 'Delete remote branch…',
  },
  /** The sync with the remote, at the foot of the picker. */
  sync: {
    pull: 'Pull',
    push: 'Push',
    publish: 'Publish',
    fetch: 'Fetch',
    now: 'now',
    /** What the branch button says while it runs. */
    pulling: 'Pulling…',
    pushing: 'Pushing…',
    fetching: 'Fetching…',
  },
  /** Switching the project's folder to another branch. */
  switch: {
    /** `{branch}`: the branch asked for. */
    title: 'Switch to “{branch}”?',
    dirty: 'There are uncommitted changes in the project’s folder.',
    stashAndSwitch: 'Stash and switch',
    /** `{name}`: the message of the stash made. */
    stashed: 'Changes stashed: “{name}” (git stash).',
    /** `{agent}`: the agent whose turn is running in the project's folder. */
    agentWorking: 'Agent {agent} is working in the project’s folder: wait for the end of its turn.',
  },
  create: {
    title: 'New branch',
    name: 'Name',
    namePlaceholder: 'feat/my-branch',
    from: 'Start from',
    /** `{hash}`: the commit chosen in the graph. */
    fromCommit: 'Commit {hash}',
    /** Detached HEAD: the commit the folder is on. */
    fromHead: 'The current commit',
    switchTo: 'and switch to it',
    /** `{branch}`: the branch to create. */
    created: 'Branch “{branch}” created.',
    createdHere: 'Branch “{branch}” created: you’re on it.',
    dirtyTitle: 'Create “{branch}” and switch to it?',
  },
  delete: {
    /** `{branch}`: the branch to delete. */
    title: 'Delete the branch “{branch}”?',
    body: 'The branch “{branch}” will be deleted locally.',
    /** `{remote}`: its remote copy, as git names it (`origin/feat`). */
    alsoRemote: 'Also delete {remote}',
    done: 'Branch “{branch}” deleted.',
    remoteTitle: 'Delete the remote branch “{branch}”?',
    remoteBody: 'It will be deleted from the remote repository, for everyone.',
    forceTitle: 'Delete “{branch}” anyway?',
    forceCommits: {
      one: '{count} commit is in no other branch.',
      other: '{count} commits are in no other branch.',
    },
    /** Not in the base, but every commit of it is in another branch. */
    forceNoCommits: 'It isn’t merged into the project’s base, but its commits are in another branch.',
    forceConfirm: 'Delete anyway',
  },
  /** The menu of a commit of the git graph. */
  graph: {
    createHere: 'Create a branch here…',
    /** `{branch}`: a local branch on the commit. */
    switchTo: 'Switch to {branch}',
    compare: 'Compare with the current branch',
    /** The folder is on no branch (detached HEAD): it is its commit that is compared. */
    compareHead: 'Compare with HEAD',
    /** The title of the diff window. `{from}`: the current branch (or HEAD); `{to}`: the branch, or the commit, compared with it. */
    compareTitle: '{from} ↔ {to}',
    delete: 'Delete branch…',
  },
  /** “Merged branches”: the clean-up. */
  merged: {
    title: 'Merged branches',
    intro: 'These local branches are already in the project’s base.',
    list: 'Branches to delete',
    empty: 'No merged branches to clean up.',
    remove: { one: 'Delete {count} branch', other: 'Delete {count} branches' },
    removing: 'Deleting…',
    done: { one: '{count} branch deleted.', other: '{count} branches deleted.' },
    /** `{branch}`: the branch that stayed; `{reason}`: why, as the backend says. */
    failed: '“{branch}” couldn’t be deleted: {reason}',
  },
});
