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
});
