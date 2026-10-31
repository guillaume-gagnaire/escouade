import { defineZone } from '../types';

// The accounts of every harness.
export default defineZone('accounts', {
  principal: 'Main',

  tab: {
    today: 'Today',
    list: 'Accounts',
    group: '{harness} accounts',
    order:
      'New agents start on the first active account of their harness whose 5 hours is under the “Pause above quota” and “Switch accounts at” thresholds and whose week keeps its pace, in this order.',
    name: 'Name',
    folder: 'Folder',
    executable: 'Executable',
    executableDefault: 'The one in the settings',
    active: 'Active',
    lastActive: 'Each harness needs at least one active account.',
    up: 'Move {name} up',
    down: 'Move {name} down',
    signIn: 'Sign in…',
    signInAgain: 'Sign in again…',
    changeKey: 'Change the key…',
    usage: 'Usage on {site}',
    usageHint:
      '{harness} gives Escouade no usage: its dashboard shows it (the month’s included usage, the reset). A limit reached stops the turn on a card that says so.',
    usageHintInterface:
      '{harness} only gives Escouade the tokens of its turns: its quota (by group of models, weekly) is read by /usage in its interface, and {site} says what each plan includes. A limit reached stops the turn on a card that says so; the agent tries again by itself an hour later, or moves to another account as your settings say.',
    usageHintGemini:
      '{harness} only gives Escouade the tokens of its turns: its usage is read by /stats in its interface, and {site} shows it for an API key. A limit reached stops the turn on a card that says so; the agent tries again by itself when Gemini says (an hour later for its daily quota), or moves to another account as your settings say.',
    usageHintCost:
      '{harness} gives Escouade what each turn costs, no quota: {site} shows the account’s usage and bill. A rate limit stops the turn on a card that says so, and the agent goes on by itself a minute later.',
    remove: 'Remove',
    add: 'Add an account…',
    none: 'No account',
    noneHint: 'No agent or ticket starts on {harness} until an account is added.',
    addTo: 'Add a {harness} account…',
    machine: 'This machine’s account',
    defaults: 'New agents',
    defaultModel: 'Model',
    defaultEffort: 'Effort',
    defaultMode: 'Mode',
    harnessDefault: 'The harness’s',
    harnessDefaultNamed: 'The harness’s ({name})',
    reserved: 'Reserved to projects',
    reservedAll: 'Every project',
    budget: 'Daily budget ($)',
    budgetNone: 'None',
    budgetHint:
      'Once reached, the autopilot pauses until midnight, as when a quota is used up. Empty: no budget. A local model costs nothing.',
    budgetInvalid: 'The daily budget is an amount in dollars, positive.',
    closedProject: '{name} (closed)',
    /** `{names}`: the projects, each quoted (`quoted`) and listed as a sentence does. */
    noAccount: {
      one: 'No active account of its default harness serves {names}: its agents and tickets won’t start.',
      other: 'No active account of their default harness serves {names}: their agents and tickets won’t start.',
    },
    quoted: '“{name}”',
    reservedHint: 'Tick projects to keep this account to them alone (a client’s projects, say): the others never start on it.',
    stillUsed: { one: 'The account is still used by {count} agent.', other: 'The account is still used by {count} agents.' },
    state: {
      checking: 'Checking…',
      connected: 'Signed in · {email}',
      connectedNoEmail: 'Signed in',
      notConnected: 'Not signed in',
      principalNotConnected: 'Not signed in with a claude.ai account (API key?)',
      inactive: 'Inactive',
    },
    removeMachineConfirm: {
      title: 'Remove this machine’s account “{name}”?',
      body: 'New agents will no longer start on it. {harness} stays signed in on this machine: removing it signs nothing out ({path}).',
      confirm: 'Remove',
    },
    removeConfirm: {
      title: 'Remove the account “{name}”?',
      body: 'New agents will no longer start on it. Its folder stays on the disk, with its sign-in and its conversations: {path}',
      confirm: 'Remove',
    },
  },

  modal: {
    addTitle: 'Add a {harness} account',
    harness: 'Harness',
    kind: 'Account',
    machine: 'This machine’s account',
    machineConnected: '{harness} is signed in on this machine · {email}',
    machineConnectedNoEmail: '{harness} is signed in on this machine',
    machineFound: '{harness} is installed on this machine',
    machineNotConnected: '{harness} isn’t signed in on this machine',
    machineSignInLater: '{harness} isn’t signed in on this machine yet: you sign in just after, in a terminal',
    machineAdded: 'Added already',
    machineChecking: 'Checking…',
    machineHint: 'Its usual folder, as it is: its settings, its sign-in and its conversations ({path}).',
    newAccount: 'A new account',
    newHint: 'A folder of its own under ~/.escouade, and its own sign-in.',
    /** Cursor CLI: a new account is an API key (ESC-43). */
    newHintKey: 'An API key of another Cursor account, kept in the system keychain.',
    reservedTo: 'Reserved to the project “{name}”: its sessions, memory and history will be shared with no other project.',
    addMachine: 'Add this account',
    signInTitle: 'Sign in to the account “{name}”',
    name: 'Name',
    namePlaceholder: 'Pro, Team…',
    share: 'Share with {name}',
    shareMachine: 'Share with this machine’s account',
    machineName: 'This machine’s account',
    files: 'Files',
    folders: 'Folders',
    filesCopied: 'copied (linking a file needs administrator rights)',
    mode: 'Sharing',
    link: 'Link (a change applies to both accounts)',
    copy: 'Copy',
    nothingToShare: '{name} has no settings, CLAUDE.md, skills, agents, commands, plugins, hooks or output styles to share.',
    neverShared: 'The sign-in (.credentials.json) and .claude.json are never shared: each account has its own.',
    create: 'Create and sign in',
    /** Above the sign-in's terminal. */
    signIn: 'Sign-in',
    signInHint: 'Sign in in the terminal below (use the /login command if Claude Code doesn’t offer it).',
    connected: 'Signed in as {email}',
    connectedNoEmail: 'Signed in',
    ended: '{harness} stopped before signing in.',
    // What a Codex account keeps apart (its folder is a CODEX_HOME).
    codex: {
      nothingToShare: '{name} has no config.toml, AGENTS.md, skills, prompts or rules to share.',
      neverShared: 'The sign-in (auth.json) is never copied: each account has its own (its token changes each time it is renewed).',
      signInHint: 'Sign in in the terminal below: codex login opens your browser on ChatGPT’s sign-in.',
    },
    // What a Copilot CLI account keeps apart (its folder is a COPILOT_HOME).
    copilot: {
      nothingToShare: '{name} has no mcp-config.json, copilot-instructions.md, agents or skills to share.',
      neverShared:
        'The sign-in is never copied: each account signs in with copilot login (its config.json and the system keychain), without the GitHub CLI’s token.',
      signInHint: 'Sign in in the terminal below: copilot login gives a code to enter on github.com/login/device.',
    },
    cursor: {
      nothingToShare: 'Nothing to share: each Cursor agent has a configuration folder of its own, where “For this agent” keeps its rules.',
      neverShared: 'Cursor CLI’s sign-in (agent login) is the machine’s: it isn’t copied, and a configuration folder doesn’t move it.',
      signInHint: 'Sign in in the terminal below: agent login opens your browser on cursor.com.',
      /** A new Cursor account is its API key (ESC-43): the machine's is the sign-in of agent login. */
      keyTitle: 'API key of the account “{name}”',
      key: 'API key',
      keyPlaceholder: 'From cursor.com, Settings › API Keys',
      keyHint:
        'Kept in the system keychain, given to this account’s agents by CURSOR_API_KEY, never written in a file nor on a command line. Their turns go to that key’s Cursor account; the machine’s account stays the sign-in of agent login.',
      create: 'Add the account',
      saveKey: 'Save the key',
    },
    agy: {
      nothingToShare:
        'Nothing to share: each Antigravity agent has a folder of its own, where Escouade writes its rules and it keeps its conversations.',
      neverShared: 'Antigravity CLI’s Google sign-in is the machine’s: it stays in the system keychain, and a folder doesn’t move it.',
      signInHint: 'Sign in in the terminal below: agy opens your browser on your Google account. Escouade never sees your credentials.',
      newHintKey: 'A Gemini API key of another Google account, kept in the system keychain.',
      keyPlaceholder: 'From aistudio.google.com, Get API key',
      keyHint:
        'Kept in the system keychain, given to this account’s agents by GEMINI_API_KEY, never written in a file nor on a command line. Their turns go to that key’s Gemini API (its models only); the machine’s account stays agy’s Google sign-in.',
    },
    gemini: {
      nothingToShare:
        'Nothing to share: each Gemini agent has a GEMINI_CLI_HOME of its own, where Escouade writes its settings and rules and it keeps its sessions.',
      neverShared:
        'This account’s Google sign-in stays in its folder, where gemini writes it: Escouade gives it to its agents by GOOGLE_APPLICATION_CREDENTIALS without ever reading it. ~/.gemini is never touched.',
      signInHint: 'Sign in in the terminal below: gemini opens your browser on your Google account. Escouade never sees your credentials.',
      newHintKey: 'A Google sign-in made by gemini in a terminal, or a Gemini API key kept in the system keychain.',
      keyPlaceholder: 'From aistudio.google.com, Get API key',
      keyHint:
        'Kept in the system keychain, given to this account’s agents by GEMINI_API_KEY, never written in a file nor on a command line. Their turns go to that key’s Gemini API.',
      machineApart:
        'Gemini CLI keeps its Google sign-in in ~/.gemini, which Escouade doesn’t touch: add a new account, signed in by gemini or by an API key. A GEMINI_API_KEY of the machine would make it the machine’s account.',
      way: 'Sign-in',
      google: 'Google account',
      googleHint: 'gemini signs in to it in a terminal, just after',
      apiKey: 'API key',
      apiKeyHint: 'A Gemini API key, from aistudio.google.com',
    },
    vibe: {
      nothingToShare: 'Nothing to share: each Vibe agent has a VIBE_HOME of its own, where “For this agent” keeps its rules and sessions.',
      neverShared:
        'Mistral Vibe’s key (vibe --setup) is the machine’s: it stays in the system keychain, and a configuration folder doesn’t move it.',
      signInHint: 'Give your API key in the terminal below: vibe --setup keeps it in the system keychain.',
      newHintKey: 'An API key of another Mistral account, kept in the system keychain.',
      keyPlaceholder: 'From console.mistral.ai, API Keys',
      keyHint:
        'Kept in the system keychain, given to this account’s agents by MISTRAL_API_KEY, never written in a file nor on a command line. Their turns are billed to that key’s Mistral account; the machine’s account stays the key of vibe --setup.',
    },
    restart: 'Restart',
    done: 'Done',
  },

  composer: {
    caption: 'Account',
    auto: 'Automatic ({name})',
    autoNone: 'Automatic (no account)',
    autoDetail:
      'The project’s, else the first account whose 5 hours is under the pause and switch thresholds and whose week keeps its pace.',
    inactive: '{name} (inactive)',
    locked: 'An agent’s account can’t change after its first message (its conversation is filed in that account).',
  },

  project: {
    label: 'Preferred account',
    desc: 'The account this project’s new agents and tickets go to.',
    auto: 'Automatic',
    autoTitle:
      'The first active account whose 5 hours is under the pause and switch thresholds and whose week keeps its pace, in the order of the accounts',
    inactive: '{name} (inactive)',
  },

  private: {
    own: {
      one: 'Account reserved to this project: {names}. {harness}’s sessions, memory and history there are shared with no other project.',
      other: 'Accounts reserved to this project: {names}. {harness}’s sessions, memory and history there are shared with no other project.',
    },
    machineToo: 'The machine’s account is still {harness}’s outside Escouade.',
    shared: {
      one: 'Its agents may go to {names}, an account shared with other projects: {harness}’s sessions, memory, history and global settings are common there.',
      other:
        'Its agents may go to {names}, accounts shared with other projects: {harness}’s sessions, memory, history and global settings are common there.',
    },
    add: 'Add a reserved account…',
    reserve: 'Reserve {name} to this project',
    reserveTitle: 'The other projects will no longer go to this account.',
    sharedOnLimit: 'Shared accounts at the limit',
    sharedOnLimitDesc:
      'When its reserved accounts reach the usage limit, its agents may go on on a shared account (a switch of account, a handover to another harness). Off, they wait for the reset.',
    sharing: {
      one: 'The private project {names} still goes to this account, shared with other projects.',
      other: 'The private projects {names} still go to this account, shared with other projects.',
    },
    reserveTo: 'Reserve to “{name}”',
  },

  pause: {
    over: { one: 'The {accounts} account is past the threshold.', other: 'The {accounts} accounts are past the threshold.' },
  },

  quota: {
    hours: { label: '{n}h', name: { one: '{count}-hour quota', other: '{count}-hour quota' } },
    days: { label: '{n}d', name: { one: '{count}-day quota', other: '{count}-day quota' } },
    credits: { label: 'credits', name: 'Credits of the month' },
    week: { label: 'W' },
    reset: 'reset {countdown}',
    overageCurrent: '{names}: extra usage in progress, billed separately. Agents already running stay on it, and new ones go to it too.',
    overage: '{names}: extra usage in progress, billed separately. Agents already running stay on it, new ones go to another account.',
    overageMoving:
      '{names}: extra usage in progress, billed separately. Its agents go on on another account at the end of their turn, and new ones go there.',
    projected: '→ {pct}',
    projectedSr: 'projected at the reset: ',
    projectedCost: '→ {amount}',
    projectedTip: 'projected at the reset: {pct}',
    tip: '{pct} · resets on {date} at {time}',
    unavailable: 'Quota unavailable',
    line: '{name}: {tip}',
  },

  panel: {
    open: 'Quota by account (current account: {name})',
    title: 'Accounts quota',
    label: 'Quota by account',
    noQuota: 'No quota: only what its turns cost counts.',
    usageElsewhere: '{harness} gives Escouade neither its usage nor its cost.',
    usageLink: 'See the usage on {site}',
    budget: 'Daily budget',
    budgetReset: 'Reset at midnight.',
    projectionNote:
      'After each quota, where it will be at its reset at the current pace: green, orange past 80%, red past 100%. The status bar only shows those past 80%.',
    projectionNoteAlways: 'After each quota, where it will be at its reset at the current pace: green, orange past 80%, red past 100%.',
    current: 'current',
    over: 'past the pause or switch threshold',
    behind: 'pace too fast for its window',
    nearLimit: 'near the limit of its window',
    today: 'Today: {amount}',
  },

  resume: {
    on: 'Resume on {account}',
    another: 'Resume on another account',
    failed: 'Resuming on {account} failed: {error}.',
    back: 'Go back to {account}',
  },

  quotas: {
    title: 'Quotas',
    limitTitle: 'When a quota runs out',
    limitNote:
      'New agents start on the first active account of their harness whose 5 hours is under the thresholds “Pause above quota” and “Switch accounts at” and whose week keeps its pace. What follows says where an agent that reaches them goes.',
    switchAt: 'Switch accounts at',
    switchAtLimit: 'At the limit',
    switchAtDesc:
      'An agent goes on on another account of its harness at the end of its turn when a short window (5 hours) of its account reaches this threshold, or a long one (week, 30 days, month’s credits) reaches it without lasting to its reset at the current pace. New agents and tickets go to the first account in the order whose 5 hours is not full and whose week keeps its pace, even on “At the limit” (a running agent then does not move before 100%, and “Resume a ticket’s agent stopped by the usage limit on another account”, below, decides). “Pause above quota” (Kanban) also holds the tickets back once every account has passed it.',
    projection: 'Quota projection',
    projectionDesc:
      'After each quota, where it will stand at its reset at the current pace (“→ 64%”): green, amber past 80%, red past 100%; for a harness without a quota, the day’s cost projected to midnight, against its budget. The quota panel and the tooltips show them all; the status bar, those past 80% or all of them, except under 1240 px wide, where they stay in the tooltips.',
    projectionAlert: 'Past 80%',
    projectionAlways: 'Always',
    projectionOff: 'Never',
  },

  engineOrder: {
    label: 'Harness order',
    desc: 'After its default harness, a project goes on to the next ones in this order when its own quota is spent, if they have an active account that serves it.',
    up: 'Move {name} up',
    down: 'Move {name} down',
  },

  switchOnLimit: {
    label: 'Resume a ticket’s agent stopped by the usage limit on another account',
    desc: 'Otherwise it waits for its account to reset',
  },
});
