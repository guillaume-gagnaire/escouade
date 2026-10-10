import { defineZone } from '../types';

// Commits, diffs, the files panel and the git graph.
export default defineZone('git', {
  files: {
    scopeAgent: 'This agent',
    scopeProject: 'Whole project',
    hintWorktree: 'worktree {path} · isolated from the other agents',
    hintAgent: 'Files changed by this agent (from its edits)',
    hintProject: 'All the project’s agents · attributed by worktree',
    emptyAgent: 'No files changed by this agent.',
    emptyProject: 'No uncommitted changes.',
    openNamed: 'Open {name} in editor',
    showDiff: 'Show diff',
    commitAgent: 'Commit…',
    commitAll: 'Commit all…',
    merge: 'Merge {branch} → {base}…',
    changesOf: 'Changes in {name}',
    more: { one: '… and {n} more file', other: '… and {n} more files' },
    restore: 'Restore the file',
    deleteFile: 'Delete the file…',
    discardChanges: 'Discard changes…',
    deleteTitle: 'Delete “{name}”?',
    discardTitle: 'Discard the changes to “{name}”?',
    deleteBody: '{path} was never committed: it is deleted from your disk and can’t be recovered.',
    discardBody: '{path} goes back to its state at the last commit: its uncommitted changes are lost.',
    discardConfirm: 'Discard changes',
  },

  diff: {
    unified: 'Unified',
    split: 'Side by side',
    closeTitle: 'Close ({key})',
    none: 'No differences.',
    binary: 'Binary file.',
    tooLarge: 'This diff is too large to display.',
    large: 'Large diff ({n} lines)',
    showMore: { one: 'Show {count} more line', other: 'Show {count} more lines' },
    ofFile: 'Diff of {path}',
  },

  commit: {
    title: 'Commit',
    scope: 'Changes in {owner}',
    filesLabel: 'Files in the commit',
    noneForAgent: 'No files to commit for this agent.',
    noneForProject: 'No changes in the project folder: commit an agent’s worktree from “This agent”.',
    leftOut: {
      one: 'Never committed: {files} (copied into the worktrees).',
      other: 'Never committed: {files} (copied into the worktrees).',
    },
    message: 'Message',
    proposing: 'Haiku is writing the message…',
    noProposal: 'No suggestion: {reason}.',
    created: 'Commit {hash} created',
    regenerate: 'Regenerate',
    commit: 'Commit',
    committing: 'Committing…',
    discardTitle: 'Discard the message?',
    discardBody: 'The message you wrote for this commit will be lost.',
    discardConfirm: 'Discard',
  },

  graph: {
    empty: 'No commits.',
    tag: 'tag {name}',
    agentBranch: 'branch {branch} of agent {agent}',
  },

  agent: {
    commitPrompt:
      'Commit the changes you made in this repository, with a clear message in Conventional Commits format. Only include the files you changed; if there are several distinct topics, make several commits.',
    commitAllPrompt:
      'Commit all the current changes in the repository, grouped into coherent commits, with clear messages in Conventional Commits format.',
    commitRequested: 'Commit request sent to {name}',
    mergeTitle: 'Merge {branch} into {base}?',
    mergeBody: 'The commits of agent “{name}” are merged into the project’s “{base}” branch.',
    mergeConfirm: 'Merge',
    squash: 'Squash (a single commit)',
    merged: 'Merge completed',
    switchTitle: 'Switch to “{base}”?',
    switchBodyOnBranch: 'The project is on the branch “{current}”. Escouade switches to “{base}” and then merges “{branch}”.',
    switchBodyDetached: 'The project is on no branch (detached HEAD). Escouade switches to “{base}” and then merges “{branch}”.',
    switchConfirm: 'Switch and merge',
    remoteOn: '{name} is reachable from claude.ai and the Claude app',
    linkCopied: 'claude.ai link copied',
  },
});
