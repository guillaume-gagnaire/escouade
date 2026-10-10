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
    week: 'Autopilot paused: weekly quota at {pct} (resumes {when})',
    fiveHour: 'Autopilot paused: 5-hour quota at {pct} (resumes {when})',
  },

  summary: {
    merge: 'merge {strategy} → {target}',
    pr: 'PR → {target}',
    push: 'push ticket/*',
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
    after: '⏸ after {keys}',
    pause: 'Waiting: autopilot paused',
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
        desc: 'Pushes ticket/<key> and opens a PR to the target branch for review.',
      },
      push: {
        label: 'Push the ticket’s branch',
        desc: 'Commits and pushes to ticket/<key>, with no merge or PR.',
      },
      keep: {
        label: 'Leave as is',
        desc: 'The changes stay uncommitted in the worktree.',
      },
    },
    targetBranch: 'Target branch',
    strategy: 'Strategy',
    strategies: { merge: 'Merge commit', squash: 'Squash', rebase: 'Rebase' },
    draftPr: 'Draft PR',

    beforeAfter: 'Before and after',
    testCommand: 'Test command',
    testCommandHint: 'run in the ticket’s worktree',
    testCommandPlaceholder: 'e.g. npm test',
    testsFirst: 'Re-run the tests first',
    testsFirstDesc: 'Blocks the action if a test fails and sends the ticket back to the agent.',
    cleanup: 'Delete the worktree once approved',
    cleanupDesc:
      'Frees disk space, after its teardown commands. After a merge, its branch goes too; if pushed or proposed as a PR, it stays.',
    commitMessage: 'Generated commit message',
    onConflict: 'On conflict',
    conflicts: { ask: 'Ask me', agent: 'Let the agent resolve', abort: 'Cancel' },

    autopilot: 'Autopilot',
    autoAssign: 'Assign tickets automatically',
    autoAssignDesc: 'A free agent takes the next “To do” ticket',
    quotaPause: 'Pause above quota',
    quotaPauseHint: 'for all projects',
    quotaPauseDesc: 'No ticket starts while the 5-hour window or the weekly window is above this threshold.',

    parallel: 'In parallel',
    parallelDesc: 'Beyond that, “To do” tickets wait for a free slot',
    defaultModel: 'Default ({model})',
    default: 'Default',
    effort: 'Effort',
    mode: 'Mode',
  },
});
