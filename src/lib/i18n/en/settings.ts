import { defineZone } from '../types';

// The settings window and the settings of a project.
export default defineZone('settings', {
  tabs: {
    app: { label: 'Application', desc: 'Language of the interface and of texts written by Claude' },
    claude: { label: 'Claude Code', desc: 'Executable, model, and default permissions' },
    notifications: { label: 'Notifications', desc: 'Visual and sound alerts' },
    projects: { label: 'Projects', desc: 'Settings specific to each project' },
    board: { label: 'Kanban', desc: 'Autopilot and approved tickets' },
    integrations: { label: 'Integrations', desc: 'Jira, Trello, and GitHub Issues' },
    terminals: { label: 'Terminals', desc: 'Shells available in the built-in terminals' },
    network: {
      label: 'Network',
      desc: 'HTTP(S) proxy and TLS certificates for Claude Code, integrations, and updates',
    },
    about: { label: 'About', desc: 'Version and local data' },
  },

  modal: {
    changed: 'Modified, not saved yet',
    saved: 'Settings saved',
    noProject: 'No project open.',
  },

  problem: {
    projectName: 'The project “{name}” needs a name.',
    command: 'Every launch command in “{name}” needs a name and a command line.',
  },

  suggest: {
    nothing: 'Claude found no command to run for this project.',
    proposed: {
      one: '1 command suggested: review it before saving.',
      other: '{count} commands suggested: review them before saving.',
    },
    proposedLeftOne: {
      one: '1 command suggested, 1 left out (invisible characters or too long): review it before saving.',
      other: '{count} commands suggested, 1 left out (invisible characters or too long): review them before saving.',
    },
    proposedLeftMany: {
      one: '1 command suggested, {refused} left out (invisible characters or too long): review it before saving.',
      other: '{count} commands suggested, {refused} left out (invisible characters or too long): review them before saving.',
    },
  },

  fields: {
    shell: 'Shell',
    shellNotFound: '{name} (not found)',
    subfolder: 'Subfolder',
    addCommand: '+ Add a command',
  },

  app: {
    language: 'Language',
    uiLanguage: 'Interface language',
    system: 'System ({lang})',
    claudeLanguage: 'Language of texts written by Claude',
    sameAsUi: 'Same as the interface (default)',
    claudeLanguageHelp:
      'Proposed commit messages and pull request descriptions, comments posted to Jira, Trello, and GitHub, instructions given to agents.',
    langName: { fr: 'French', en: 'English' },
  },

  claude: {
    executable: 'Executable',
    path: 'Executable path',
    pathHint: 'blank = detect automatically',
    pathFound: 'claude (found in PATH)',
    pathMissing: 'not found — enter the path to claude.exe',
    newAgents: 'New agents',
    defaultModel: 'Default model',
    defaultEffort: 'Default effort',
    defaultMode: 'Default permission mode',
    processes: 'Processes',
    autoResume: 'Auto-resume after the usage limit',
    autoResumeDesc: '“continue” is sent once the quota has reset',
    idleStop: 'Stop idle Claude processes after',
    idleStopHint: 'minutes, 0 = never',
    idleStopLabel: 'Stop idle Claude processes after (minutes)',
  },

  notifications: {
    notifyFor: 'Notify me about',
    noteMac: 'Off: no system notification or chime for this type; the tab, the card, and the Dock still flag the agent.',
    noteOther: 'Off: no system notification or chime for this type; the tab, the card, and the taskbar still flag the agent.',
    questions: 'Questions and permissions',
    done: 'Finished tasks',
    errors: 'Errors',
    tickets: 'Tickets (ready to review, blocked)',
    channels: 'Channels',
    system: '{os} notifications when the app is not in the foreground',
    systemLabel: 'System notifications',
    sound: 'Sound',
    soundOn: 'Sound on',
    soundOnDesc: 'A question from Claude, the end of a turn',
    soundTest: 'Test the sound',
    soundTestButton: '▶ Test',
  },

  terminals: {
    paths: 'Paths',
    detected: 'Detected: {list}',
    none: 'none',
    auto: 'blank = auto',
    wsl: 'WSL distribution',
    wslHint: 'blank = default distribution',
  },

  network: {
    proxy: 'Proxy',
    proxyNote:
      'The proxy is passed to Claude Code processes, quota reading, integrations, and updates. It applies to agents the next time their process (re)starts.',
    proxyUrl: 'HTTP(S) proxy',
    proxyUrlHint: 'e.g. http://user:password@proxy:3128',
    proxyUrlNone: 'none',
    exclusions: 'Exclusions',
    exclusionsHint: 'NO_PROXY, separated by commas',
    proxyTerminals: 'Also apply the proxy to the built-in terminals',
    certificates: 'Certificates',
    certificatesNote:
      'Integrations, quotas, and updates trust recognized certificates and those installed on this system (usually a corporate proxy’s). Claude Code has its own list.',
    insecureTls: 'Skip TLS certificate verification',
    insecureTlsDesc:
      'For a proxy that decrypts traffic with an unrecognized certificate (the “UnknownIssuer” error). Applies to integrations, quotas, updates, and Claude Code processes (the next time their process starts), along with the commands agents run (npm, node…). Use it only on a network you trust: an intercepted connection, tokens included, would no longer be detected.',
  },

  about: {
    claudeFound: 'Claude Code detected',
    claudeMissing: 'Claude Code not found',
    checking: 'Checking…',
    check: 'Check for updates',
    noUpdate: 'No update available.',
    autoUpdate: 'Install updates automatically',
    autoUpdateDesc: 'Escouade restarts itself when no agent is working and everything is saved.',
    localData: 'Local data',
  },

  project: {
    identity: 'Identity',
    nameLabel: 'Project name',
    colorN: 'Color {n}',
    worktreePerAgent: 'One worktree per agent',
    worktreePerAgentDesc: 'Keeps each new agent’s changes in its own branch.',
    copy: 'Files copied into worktrees',
    copyDesc: 'Only files git ignores, never committed. One pattern per line: .env* at the root, **/.env* everywhere.',
    commit: 'Commit',
    commitDesc: 'Direct: Escouade suggests a message, you review it, and you commit yourself.',
    commitModes: {
      agent: 'Written by the agent',
      direct: 'Direct, with a suggested message',
    },
    worktrees: 'Worktrees',
    worktreesNote:
      'Available variables: ESCOUADE_PROJECT_DIR (the project), ESCOUADE_WORKTREE_DIR, ESCOUADE_BRANCH, and the ports reserved for a ticket (ESCOUADE_PORT_BASE, ESCOUADE_PORT_END). A failure is reported in the agent’s conversation.',
    fill: 'Fill in automatically',
    fillButton: '✦ Fill in automatically',
    filling: 'Claude is reading the project…',
    fillWorktreesDesc:
      'Claude reads the project (manifests, lockfiles, README…) without changing anything and suggests the commands. Review them before saving.',
    fillLaunchDesc:
      'Claude reads the project (manifests, scripts, docker-compose, README…) without changing anything and suggests the commands to run. Review them before saving.',
    proposedWorktrees: 'Suggested worktree commands',
    proposedWorktreesNote:
      'Claude’s suggestion: read each command in full. They replace both lists and, once saved, run on their own: the setup when each new worktree opens, the teardown before it is removed.',
    proposedLaunch: 'Suggested launch commands',
    proposedLaunchNote: 'Claude’s suggestion: read each command in full. They replace the ones in the list.',
    ignore: 'Ignore',
    replace: 'Replace the commands',
    noCommands: 'No commands.',
    worktreeRoot: 'the worktree root',
    projectFolder: 'the project folder',
    setupTitle: 'When a worktree opens',
    setupDesc: 'In order, before its agent’s first message: dependencies, generated code… Messages wait until they finish.',
    teardownTitle: 'Before it is removed',
    teardownDesc: 'What the setup created outside the worktree (database, containers…); often nothing.',
    launch: 'Launch',
    launchNote: 'Each command runs in its own terminal, read-only. Start them from the “Launch” section of the sidebar.',
    commandN: 'Command {n}',
    namePlaceholder: 'e.g. Front',
    commandPlaceholder: 'e.g. npm run dev',
    subfolderHint: '(blank = project folder)',
    subfolderPlaceholder: 'e.g. apps/web',
    noCommandsYet: 'No commands yet.',
    danger: 'Danger zone',
    close: 'Close the project',
    closeDesc:
      'The project and its agents are removed from the app, conversations included. The files and worktrees on disk are not touched.',
    closeButton: 'Close the project…',
  },

  steps: {
    setup: {
      group: 'Setup command {n}',
      add: 'Add a setup command',
    },
    teardown: {
      group: 'Teardown command {n}',
      add: 'Add a teardown command',
    },
    commandPlaceholder: 'e.g. npm ci',
    subfolderTitle: 'Worktree subfolder (blank: its root)',
    subfolderPlaceholder: 'subfolder',
    up: 'Move up ({keys})',
    upN: 'Move command {n} up',
    down: 'Move down ({keys})',
    downN: 'Move command {n} down',
    deleteN: 'Delete command {n}',
  },
});
