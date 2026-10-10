import { defineZone } from '../types';

// The Claude accounts.
export default defineZone('accounts', {
  principal: 'Main',

  tab: {
    list: 'Claude accounts',
    order: 'New agents start on the first active account under the pause threshold, in this order.',
    name: 'Name',
    folder: 'Folder',
    executable: 'Executable',
    executableDefault: 'The one in the settings',
    active: 'Active',
    lastActive: 'At least one account must be active.',
    up: 'Move {name} up',
    down: 'Move {name} down',
    signIn: 'Sign in…',
    signInAgain: 'Sign in again…',
    remove: 'Remove',
    add: 'Add an account…',
    stillUsed: { one: 'The account is still used by {count} agent.', other: 'The account is still used by {count} agents.' },
    state: {
      checking: 'Checking…',
      connected: 'Signed in · {email}',
      connectedNoEmail: 'Signed in',
      notConnected: 'Not signed in',
      principalNotConnected: 'Not signed in with a claude.ai account (API key?)',
      inactive: 'Inactive',
    },
    removeConfirm: {
      title: 'Remove the account “{name}”?',
      body: 'New agents will no longer start on it. Its folder stays on the disk, with its sign-in and its conversations: {path}',
      confirm: 'Remove',
    },
  },

  modal: {
    addTitle: 'Add a Claude account',
    signInTitle: 'Sign in to the account “{name}”',
    name: 'Name',
    namePlaceholder: 'Pro, Team…',
    share: 'Share with {name}',
    files: 'Files',
    folders: 'Folders',
    filesCopied: 'copied (linking a file needs administrator rights)',
    mode: 'Sharing',
    link: 'Link (a change applies to both accounts)',
    copy: 'Copy',
    nothingToShare: '{name} has no settings, CLAUDE.md, skills, agents, commands, plugins, hooks or output styles to share.',
    neverShared: 'The sign-in (.credentials.json) and .claude.json are never shared: each account has its own.',
    create: 'Create and sign in',
    signInHint: 'Sign in in the terminal below (use the /login command if Claude Code doesn’t offer it).',
    connected: 'Signed in as {email}',
    connectedNoEmail: 'Signed in',
    ended: 'Claude Code stopped before signing in.',
    restart: 'Restart',
    done: 'Done',
  },

  composer: {
    caption: 'Account',
    auto: 'Automatic ({name})',
    autoDetail: 'The project’s, else the first account under the pause threshold.',
    inactive: '{name} (inactive)',
    locked: 'An agent’s account can’t change after its first message (its conversation is filed in that account).',
  },

  project: {
    label: 'Preferred account',
    desc: 'The account this project’s new agents and tickets go to.',
    auto: 'Automatic',
    autoTitle: 'The first active account under the pause threshold, in the order of the accounts',
    inactive: '{name} (inactive)',
  },

  pause: {
    over: { one: 'The {accounts} account is past the threshold.', other: 'The {accounts} accounts are past the threshold.' },
  },

  quota: {
    fiveHour: { label: '5h', name: '5-hour quota' },
    sevenDay: { label: 'W', name: '7-day quota' },
    reset: 'reset {countdown}',
    tip: '{pct} · resets on {date} at {time}',
    unavailable: 'Quota unavailable',
  },

  panel: {
    open: 'Quota by account (current account: {name})',
    title: 'Claude accounts quota',
    current: 'current',
    over: 'past the pause threshold',
  },
});
