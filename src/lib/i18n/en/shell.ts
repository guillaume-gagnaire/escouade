import { defineZone } from '../types';

// The frame of the app: status bar, welcome screen, modals, toasts, updates.
export default defineZone('shell', {
  /** The window when it has nothing else to show. */
  app: {
    /** `{error}`: what the backend answered, as received. */
    cantStart: 'Can’t start: {error}',
    noAgent: 'No agents in this project.',
    newAgent: 'New agent',
  },
  welcome: {
    intro: 'Add a project to run agents, follow their questions, and open terminals.',
    /** `{command}`: the command that installs it, in code style. */
    noEngine: 'No harness found',
    noAccount: 'No account',
    noAccountDesc: 'Add an account: the one already signed in on this machine, or a new one. Only the accounts added start agents.',
    noEngineDesc: 'Escouade runs coding agents. Install one of them, then set its path in {where} if it isn’t on the PATH.',
    harnessSettings: 'Settings › Harness',
    harnessSettingsButton: 'Settings › Harness…',
    copy: 'Copy the command of {harness}',
    copied: 'Command copied',
    machine: 'Signed in on this machine',
    machineAs: 'Signed in on this machine · {email}',
    notConnected: 'Installed, not signed in',
    searchAgain: 'Search again',
    searched: 'Searched in the PATH',
    searchedWsl: 'Searched in the PATH and in WSL ({distro})',
    title: 'A first project',
    addProject: 'Add a project…',
  },
  /** The window that adds a project. */
  newProject: {
    title: 'New project',
    folderTitle: 'Project folder',
    pathPlaceholder: 'C:\\path\\to\\the\\project',
    browse: 'Browse…',
    tabPreview: 'Tab preview',
    untitled: 'new-project',
    colorN: 'Color {n}',
    firstAgent: 'Create a first agent',
    firstAgentDesc: 'Opens a conversation in this project right away',
    worktree: 'One git worktree per agent',
    worktreeDesc: 'Keeps each agent’s work apart and shows its changed files separately',
    create: 'Create project',
    creating: 'Creating…',
    history: {
      found: 'History found: {agents}, {tickets}, last activity on {date}',
      fresh: 'Start from a blank project',
      freshDesc: 'The history stays in Settings › Projects, where you can erase it.',
      reopen: 'Reopen project',
      reopening: 'Reopening…',
    },
    /** What is known of the folder chosen. */
    git: {
      choose: 'Choose the project folder',
      missing: 'Folder not found',
      repoClean: 'Git repository detected · branch {branch} · clean',
      repoDirty: {
        one: 'Git repository detected · branch {branch} · {count} file changed',
        other: 'Git repository detected · branch {branch} · {count} files changed',
      },
      none: 'No git repository · one will be initialized',
    },
  },
  /** The page's process ended while it showed a project (ESC-290): loaded again without it. */
  pageCrash: {
    project: 'The interface stopped while showing “{project}”: it was reloaded without that project.',
    other: 'The interface stopped: it was reloaded.',
    reopen: 'Reopen',
  },
  /** The update downloaded, its notes once installed, and the toasts about it. */
  update: {
    ready: 'Escouade {version} is ready',
    notes: 'What’s new in Escouade {version}',
    saveFirst: {
      one: 'Save your files first: {count} file isn’t saved.',
      other: 'Save your files first: {count} files aren’t saved.',
    },
    working: {
      one: '{count} agent is working or waiting for your answer: its current turn will be interrupted. If it’s working on a ticket, it picks up again by itself; otherwise it waits for your next message.',
      other:
        '{count} agents are working or waiting for your answer: their current turn will be interrupted. Ticket agents pick up again by themselves; the others wait for your next message.',
    },
    restartNow: 'Restart now',
    restarting: 'Restarting…',
    failed: 'The update to {version} couldn’t be installed.',
    notReady: 'The update to {version} isn’t ready anymore: Escouade will offer it again once it’s downloaded.',
    installed: 'Escouade {version} is installed.',
    seeNotes: 'See what’s new',
    /** `{error}`: the cause the backend gives, already in the language of the interface (or what the call answered, as received). */
    downloadFailed: 'Couldn’t download the update: {error}',
    checkFailed: 'Couldn’t check for updates: {error}',
    /** The button of a failed download: the page of the release, where the installer can be taken from. */
    openInBrowser: 'Download in the browser',
  },
  /** The question of the first start: the usage statistics (`telemetry`). */
  telemetry: {
    title: 'Help improve Escouade?',
    body: 'Escouade can send usage statistics, with nothing that identifies you: which features are used, with which version of Escouade and which system. Never your projects, your code, your conversations or your name.',
    detail:
      'They go to a server of Escouade’s hosted in France (Aptabase, open source), under an id drawn at random, and are kept 25 months. You can change your mind at any time in Settings › About.',
    more: 'What is sent, in detail',
    yes: 'Share statistics',
    no: 'No thanks',
  },
  /** The question asked when the window is closed with files unsaved. */
  quit: {
    title: 'Quit Escouade?',
    unsaved: {
      one: '{count} file in the editor isn’t saved: its changes will be lost.',
      other: '{count} files in the editor aren’t saved: their changes will be lost.',
    },
    confirm: 'Quit anyway',
  },
  /** A file the editor was asked to open, outside the folder of its source. */
  openFile: {
    outsideProject: '{name} is outside the project folder.',
    outsideAgent: '{name} is outside this agent’s folder.',
  },
  /** What an agent has spent. */
  spend: {
    estimateHint: 'Estimate (public prices) during the turn; exact cost at its end',
    /** `{pct}`: written as a percentage; `{used}` and `{size}`: counts of tokens. */
    context: 'Context: {pct} of the model’s window ({used} tokens of {size})',
  },
  /** The bar at the bottom of the window. */
  status: {
    activeWord: { one: 'active', other: 'active' },
    waitingWord: 'waiting',
    doneWord: { one: 'done', other: 'done' },
    harnessAccount: '{harness}, {account}',
    overage: 'extra usage',
    /** `{key}`: the shortcut. */
    nextWaiting: 'Go to the card of the next agent that is waiting or needs a look ({key})',
    procsTitle: 'Running agent processes (with the tools and MCP servers they start)',
    /** `{memory}` and `{cpu}`: the amounts, in code style. */
    procs: { one: '{count} agent · {memory} · {cpu} CPU', other: '{count} agents · {memory} · {cpu} CPU' },
    procRow: '{name}: {memory} · {cpu}',
    today: 'Today',
    dayCostTip: '{harness} has no quota: its account cost {cost} today, with no daily budget.',
    dayBudgetTip:
      '{harness} has no quota: its account cost {cost} today, out of a daily budget of {budget} (once reached, the autopilot waits for midnight).',
    dayProjectedTip: 'Projected at midnight: {amount}.',
    /** The first line of the branch button's tooltip while an agent open works in a folder of its own. */
    agentBranch: 'Branch of agent {agent}, in its worktree',
    /** After the pull request of the branch shown: how many more it has. */
    morePrs: { one: '+{count} more', other: '+{count} more' },
    /** The branch against its remote. */
    sync: {
      tracked: 'Tracking {upstream}: {behind} to pull, {ahead} to push',
      gone: 'The tracked branch {upstream} no longer exists on the remote repository',
      unpublished: 'Branch not published to the remote repository yet',
      lastFetch: 'Last fetch: {when}',
      never: 'never',
      goneShort: 'remote deleted',
      unpublishedShort: 'not published',
      behindShort: { one: '{count} commit to pull', other: '{count} commits to pull' },
      aheadShort: { one: '{count} commit to push', other: '{count} commits to push' },
    },
    restartIn: 'Restarting in {seconds} s',
    updateReady: 'Update {version} ready · {restart}',
    restart: 'Restart',
    updating: 'Update {version}…',
    sound: 'Notification sound',
    soundTip: 'Notification sound: {state}',
    soundOn: 'On',
    soundOff: 'Off',
  },
});
