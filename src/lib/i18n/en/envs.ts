import { defineZone } from '../types';

// The deployments (ESC-67): the environments a done ticket is in, on its card; the preview of a pull request, on a card
// « In review »; the view « Environments » of a project's board.
export default defineZone('envs', {
  card: {
    label: 'Environments',
    deployed: '{name}: deployed',
    waiting: '{name}: not deployed yet',
    preview: 'Open preview',
    previewTitle: 'Opens the {word}’s preview: {url}',
  },

  tabs: { label: 'Board view', tickets: 'Tickets', environments: 'Environments' },

  view: {
    title: 'Environments',
    sub: '{project} · {host}',
    read: 'read {since}',
    refresh: 'Refresh',
    refreshTitle: 'Reads the host’s deployments now',
    waiting: 'Merged, not deployed yet',
    notInProd: {
      one: '{count} ticket not in production yet',
      other: '{count} tickets not in production yet',
    },
    runs: '{sha} · {when}',
    by: 'by {author}',
    nothingRuns: 'No successful deployment',
    failed: 'Last deployment failed · {sha}',
    failedNoSha: 'Last deployment failed',
    job: 'See the job',
    underWay: 'Deployment under way · {sha}',
    more: { one: '+ {count} more', other: '+ {count} more' },
    merged: 'merged {since}',
    empty: 'No ticket has reached it.',
    waitingEmpty: 'Nothing waits for a deployment.',
    production: 'production',
    /** The environments left out because nothing ever ran there (ESC-390). */
    neverRan: {
      one: '{names} isn’t shown: no successful deployment.',
      other: '{names} aren’t shown: no successful deployment.',
    },
    noneShown: 'No environment is shown: pick them in the environments menu.',
    loading: 'Reading the deployments…',
    none: 'No deployment on {host} for this repository. Those of your CI (GitHub Actions, GitLab CI, Bitbucket Pipelines) show here by themselves.',
    unsupported: '{host} has no deployments API: Escouade only follows the CI of the target branch.',
    error: 'Deployments unreadable: {error}',
  },

  /** The menu of the environments shown (ESC-390), in the board's header. */
  menu: {
    button: { one: '{count} environment of {total}', other: '{count} environments of {total}' },
    short: '{count} of {total}',
    title: 'Environments shown',
    tip: 'Choose the environments shown and their order',
    tickets: { one: '{count} ticket', other: '{count} tickets' },
    byYou: 'hidden by you',
    neverRan: 'no successful deployment',
    move: 'Drag to change the order',
    foot: 'Drag to change the order, or Alt+↑ and Alt+↓. Kept for this project, on this machine.',
  },
});
