import { defineZone } from '../types';

// The settings of the Kanban, and the sentences its header and cards build from them (`lib/board.ts`).
export default defineZone('boardSettings', {
  places: {
    free: { one: '{count} free slot', other: '{count} free slots' },
    full: 'All slots are taken',
    quota: 'Quota reached — resumes at {time}',
  },

  pause: {
    limit: 'Autopilot paused: usage limit reached (resumes around {time})',
    window: 'Autopilot paused: {window} quota at {pct} (resumes {when})',
    credits: 'Autopilot paused: monthly credits at {pct} (resumes {when})',
    budget: 'Autopilot paused: daily budget reached ({pct}, resumes {when})',
  },

  window: {
    week: 'weekly',
    hours: '{n}-hour',
    days: '{n}-day',
  },

  summary: {
    merge: 'merge {strategy} → {target}',
    pr: 'PR → {target}',
    push: 'push {prefix}*',
    keep: 'leave as is',
  },

  approve: {
    merge: 'Approve and merge',
    pr: 'Approve + PR',
    push: 'Approve and push',
    keep: 'Approve',
  },

  wait: {
    targetBranch: 'Waiting for the target branch',
    after: 'After {keys}',
    pause: 'Waiting: autopilot paused',
    redTarget: 'Waiting: the target’s CI is red',
    forced: 'Start requested…',
    autopilotOff: 'Autopilot is off',
    next: 'Picked up as soon as a slot is free',
    place: 'Waiting for a slot ({busy}/{max})',
  },

  launchAnyway: {
    one: '{key} is waiting for {keys}, which is not done yet. Start it anyway?',
    other: '{key} is waiting for {keys}, which are not done yet. Start it anyway?',
  },

  cycle: '{dep} already waits for {ticket} (directly or indirectly).',

  commit: {
    conventional: 'feat: limit login attempts [{key}]',
    plain: '{key} Limit login attempts',
  },

  tag: {
    loop: '{key} · loop {iteration}/{max}',
    review: '{key} · to review',
  },

  loops: { one: '{count} loop · {cost}', other: '{count} loops · {cost}' },

  tab: {
    actionsTitle: 'When I approve a ticket in “To review”',
    actions: {
      merge: {
        label: 'Merge into a branch',
        desc: 'Merges the agent’s worktree into the target branch, then frees the agent.',
      },
      pr: {
        label: 'Open a pull request',
        desc: 'Pushes {prefix}<key>, opens its PR to the target branch and follows it “In review” until it’s merged.',
      },
      push: {
        label: 'Push the ticket’s branch',
        desc: 'Commits and pushes to {prefix}<key>, with no merge or PR.',
      },
      keep: {
        label: 'Leave as is',
        desc: 'The changes stay uncommitted in the worktree.',
      },
    },
    targetBranch: 'Target branch',
    targetBranchChoose: 'Target branch: {branch}. Choose another branch',
    strategy: 'Strategy',
    strategies: { merge: 'Merge commit', squash: 'Squash', rebase: 'Rebase' },
    draftPr: 'Draft PR',
    reviewers: 'Reviewers',
    reviewersPlaceholder: 'ada, grace',
    labels: 'Labels',
    labelsPlaceholder: 'escouade, needs-review',
    replyThreads: 'Reply under fixed threads',
    replyThreadsDesc:
      '“Fixed in abc1234” under each thread given to the agent, once its fixes are pushed. Escouade never resolves a thread.',
    prUpdate: 'Branch update',
    prUpdates: { merge: 'Merge', rebase: 'Rebase' },
    prUpdateDesc: 'In conflict with the target: merge the target into the branch, or rebase and push with --force-with-lease.',

    beforeAfter: 'Before and after',
    testCommand: 'Test command',
    testCommandHint: 'run in the ticket’s worktree',
    testCommandPlaceholder: 'e.g. npm test',
    testsFirst: 'Re-run the tests first',
    testsFirstDesc: 'Blocks the action if a test fails and sends the ticket back to the agent.',
    cleanup: 'Delete the worktree of a done or deleted ticket',
    cleanupDesc:
      'After its teardown commands, whatever the approval. A worktree with uncommitted changes or commits neither merged nor pushed is kept, and you’re told. Merged or empty, its branch goes too; if pushed or proposed as a PR, it stays.',
    commitMessage: 'Generated commit message',
    onConflict: 'On conflict',
    conflicts: { ask: 'Ask me', agent: 'Let the agent resolve', abort: 'Cancel' },

    autopilot: 'Autopilot',
    autoAssign: 'Assign tickets automatically',
    autoAssignDesc: 'A free agent takes the next “To do” ticket',

    redTargetPause: 'Pause while the target’s CI is red',

    redTargetPauseDesc: 'The autopilot starts no ticket while the CI of the target branch fails. A ticket started by hand starts anyway.',

    parallel: 'In parallel',
    parallelDesc: 'Beyond that, “To do” tickets wait for a free slot',
    defaultModel: 'Default ({model})',
    default: 'Default',
    effort: 'Effort',
    mode: 'Mode',
  },
});
