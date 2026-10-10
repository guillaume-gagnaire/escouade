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
    intro: 'Add a project to run Claude Code agents in it, follow their questions and open terminals.',
    /** `{command}`: the command that installs it, in code style. */
    claudeMissing: 'Claude Code couldn’t be found on this machine. Install it ({command}) or set its path in the settings.',
    addProject: 'Add a project',
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
    /** `{error}`: what the updater answered, as received. */
    downloadFailed: 'Couldn’t download the update: {error}',
    checkFailed: 'Couldn’t check for updates: {error}',
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
    estimateHint: 'Estimate (public prices) while Claude works; exact cost at the end of the turn',
    /** `{pct}`: written as a percentage; `{used}` and `{size}`: counts of tokens. */
    context: 'Context: {pct} of the model’s window ({used} tokens of {size})',
  },
  /** The bar at the bottom of the window. */
  status: {
    active: { one: '{count} active', other: '{count} active' },
    waiting: '{n} waiting',
    done: { one: '{count} done', other: '{count} done' },
    /** `{key}`: the shortcut. */
    nextWaiting: 'Go to the next agent that is waiting or needs a look ({key})',
    procsTitle: 'Running Claude processes (with the tools and MCP servers they start)',
    /** `{memory}` and `{cpu}`: the amounts, in code style. */
    procs: '{instances} Claude · {memory} · {cpu} CPU',
    session: '5-hour session',
    week: 'Weekly',
    resetsAt: 'Resets: {date}',
    sessionUnavailable: 'Session quota unavailable',
    weekUnavailable: 'Weekly quota unavailable',
    reset: 'resets in {countdown}',
    today: 'Today',
    /** The branch against its remote. */
    sync: {
      tracked: 'Tracking {upstream}: {behind} to pull, {ahead} to push',
      gone: 'The tracked branch {upstream} no longer exists on the remote repository',
      unpublished: 'Branch not published to the remote repository yet',
      lastFetch: 'Last fetch: {when}',
      never: 'never',
      publish: 'Publish branch',
      now: 'now',
      goneShort: 'remote deleted',
      unpublishedShort: 'not published',
    },
    restartIn: 'Restarting in {seconds} s',
    updateReady: 'Update {version} ready · Restart',
    updating: 'Update {version}…',
    sound: 'Notification sound',
    soundOn: 'On',
    soundOff: 'Off',
    /** `{key}`: the shortcut. */
    settings: 'Settings ({key})',
  },
});
