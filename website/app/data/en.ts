// Everything the English page says. fr.ts says the same, key for key.
import type { Catalog } from './catalog';
import { DOWNLOAD } from './site';

export const en: Catalog = {
  meta: {
    title: 'Escouade — the cockpit for your Claude Code agents',
    description:
      'Escouade, the cockpit for your Claude Code agents: a free, open source Windows and macOS app to run several agents in parallel, in a single window, and hand them tickets, even from Jira, Trello or GitHub, that they carry through to the end.',
    locale: 'en_US',
  },

  header: {
    sections: 'Sections',
    video: 'Video',
    features: 'Features',
    install: 'Install',
    faq: 'FAQ',
    github: 'GitHub',
    language: 'Language',
  },

  anchors: { features: 'features', install: 'install' },

  hero: {
    line1: 'A squad of Claudes.',
    line2: 'A single window.',
    lede: 'Escouade, the cockpit for your Claude Code agents: a Windows and macOS app to run several agents in parallel, with projects in tabs, a native chat, git, an editor, terminals, statistics, and a Kanban of tickets, written by you or imported from Jira, Trello or GitHub, that agents pick up and carry through to the end.',
    download: 'Download for Windows and macOS',
    github: 'View on GitHub',
    meta: 'Version {version} · Windows 10 and 11, macOS 11 and later · Free and open source (MIT)',
    video: 'Watch the video',
  },

  features: {
    eyebrow: 'Features',
    title: 'Everything you need to work with several Claudes',
    lead: 'Each agent moves forward on its own; you keep the big picture and step in when you need to.',
    items: {
      agents: {
        title: 'Projects in tabs, agents in parallel',
        text: 'Each project gets its own tab and its own color, which tints the whole interface. In each one, run as many Claude Code agents as you like, each with its own conversation, model and effort; Haiku names each of them as soon as you send your first request.',
        points: [
          'Live status: working, asking, done',
          'Tokens, cost and touched files per agent',
          'One git worktree per agent, if you want',
        ],
        alt: 'Escouade’s window: the projects in tabs, the list of agents and the conversation of one of them',
      },
      chat: {
        title: 'A real chat, not a terminal',
        text: 'Markdown, highlighted code, compact tool calls that you unfold to see a diff or a command’s output. Attach an image, a PDF or a file, mention a @file, run a /command, and keep typing while Claude works: it takes your message into account at the next step. Ctrl+K finds a message across the conversations of all your agents.',
        points: [
          'Claude’s questions and permission requests as cards, which you can also answer from the keyboard',
          'Model (with its version), effort and mode, changeable at any time',
          'A card sums up each task: duration, cost, modified files',
        ],
        alt: 'A conversation: Claude’s reply in markdown, a tool call unfolded to its diff',
      },
      notifications: {
        title: 'You know when you’re needed',
        text: 'When an agent asks a question or finishes, Escouade lets you know: a blinking tab and card, a chime, a system notification that says what is asked, a flashing taskbar button (or a bouncing Dock icon). Closing the window stops nothing: each agent resumes its session on restart.',
        points: [
          'Ctrl+J jumps to the next agent that is waiting',
          'The overview shows every agent, the waiting ones first',
          'You choose what notifies you: questions, completions, errors, tickets',
        ],
        alt: 'An agent waiting for an answer, its question and its card in the sidebar',
      },
      git: {
        title: 'Git in plain sight',
        text: 'Uncommitted files per agent or for the whole project, unified or side-by-side diff, the repository’s git graph with the agent’s branch up front. The half-and-half layout shows the conversation and the files together, updated while the agent writes.',
        points: [
          'Commit written by the agent, or direct with a proposed message that you review',
          'Merge or squash a worktree into its base branch, then clean up',
          'Pull, push and fetch from the status bar',
        ],
        alt: 'The half-and-half layout: the conversation, the uncommitted files and their diff side by side',
      },
      editeur: {
        title: 'An editor, without leaving the app',
        text: 'Touch up a file of the project or of an agent’s worktree without switching windows: file tree, tabs, syntax highlighting, search, and the lines changed since the last commit marked in the gutter. Ctrl+click goes to a definition without a language server, Ctrl+P opens a file by name, Ctrl+Shift+F searches every file.',
        points: [
          'See the changes in the text, and undo a block',
          'Compare with what the agent just wrote before you choose',
          'Rename, delete to the trash, open a terminal here',
        ],
        alt: 'The built-in editor: the worktree’s file tree, tabs and the code with its changed lines',
      },
      tableau: {
        title: 'Tickets that agents pick up on their own',
        text: 'Each project’s Kanban: to do, in progress, to test, done. Write a ticket with its acceptance criteria; in autopilot, agents pick it up, each in its own worktree with its own ports, and loop until every criterion is met.',
        points: [
          'Autopilot, from 1 to 6 agents in parallel, paused near your quota limits',
          'Loop, criteria, progress and cost live on each card',
          'Validate: tests, generated commit, merge, pull request or push; or send it back to the agent',
        ],
        alt: 'A project’s Kanban: two tickets in progress with their loop and criteria, and the finished tickets',
      },
      test: {
        title: 'Test each ticket in one click',
        text: 'When a ticket moves to “To test”, “▶ Test” shows you the agent’s test plan, then prepares its worktree, starts its servers on their reserved ports, waits for them to answer and opens your browser straight on the feature that was built.',
        points: [
          'The plan comes from the agent: you read it before it runs',
          'Logs stay in the Launch section, under its name',
          '“Prepare launch” for any agent with a worktree',
        ],
        alt: 'The “Test DEM-6” window: preparation, servers ready and address opened in the browser',
      },
      integrations: {
        title: 'Your Jira, Trello and GitHub tickets, in the Kanban',
        text: 'Connect Jira, Trello or GitHub Issues and link a source to your project. “Import” searches and filters its tickets: tick them and they arrive in the Kanban with their acceptance criteria. Escouade then keeps their status up to date and comments on the original ticket when it is ready to test, then when it is done.',
        points: [
          'Acceptance criteria taken from the description or the checklist',
          'Status and comments synced column by column, and retried on failure',
          'Automatic import, if you turn it on, of tickets labelled claude-ready',
        ],
        alt: 'The “Import tickets” window: three Jira tickets ticked, with their acceptance criteria detected',
      },
      lancement: {
        title: 'Launch your project in one click',
        text: 'Set up the commands that run your project (front end, API, worker…), each with its own shell and folder, or let Claude read the project and suggest them. Each command runs in its own terminal, with its live status.',
        points: [
          'Start all, stop all, restart',
          'A crash shows right away, with its exit code',
          'Stopping also kills what the command started',
        ],
        alt: 'Three launch commands, one of them crashed, and the dev server’s log',
      },
      stats: {
        title: 'Tokens, cost, quotas: live',
        text: 'The status bar tracks your 5-hour session quota, your weekly quota, when they reset, and the day’s cost, which climbs while Claude works. An agent stopped by its usage limit resumes on its own when the quota is back.',
        points: [
          'Input, cache and output tokens, by day, week or month',
          'Cost by project, by model, by agent and by ticket',
          'Memory and CPU used by Claude',
        ],
        alt: 'The statistics page: tokens per day, cost by project and by model',
      },
      remote: {
        title: 'And from your phone',
        text: 'Turn on remote control for an agent: its session opens on claude.ai and in the Claude mobile app. What you send from there also shows up in Escouade.',
        points: [
          'Case by case, from a right click on the agent',
          'Same session after a restart',
          'The agent stays reachable as long as Escouade is running',
        ],
        alt: 'A phone on claude.ai and Escouade showing the same conversation',
      },
    },
  },

  cards: [
    { title: 'Real terminals', text: 'PowerShell, Git Bash and WSL built in, with your shell’s autocompletion.' },
    {
      title: 'Usage limit? It resumes',
      text: 'An agent stopped by its usage limit resumes on its own as soon as your quota is back, and the Kanban waits before starting a new ticket.',
    },
    {
      title: 'Always there',
      text: 'Closing the window doesn’t stop the agents: Escouade stays in the notification area and resumes each session on restart.',
    },
    {
      title: 'Behind a proxy',
      text: 'HTTP(S) proxy for Claude, quotas, updates, integrations and, if you want, terminals.',
    },
    {
      title: 'Automatic updates',
      text: 'New versions, signed, download in the background and install without a window, when no agent is working.',
    },
    {
      title: 'From the keyboard',
      text: 'Ctrl+1…9 for projects, Ctrl+N for an agent, Ctrl+J for the one that is waiting, Ctrl+Enter to allow, Ctrl+K to search, Esc to interrupt.',
    },
  ],

  install: {
    eyebrow: 'Install',
    title: 'Ready in three steps',
    steps: [
      {
        title: 'Install Claude Code',
        text: 'Escouade drives the Claude Code installed on your machine: install it and sign in once, with your Claude subscription or an API key.',
        link: { label: 'Claude Code documentation', href: 'https://code.claude.com/docs/en/overview' },
      },
      {
        title: 'Install Escouade',
        text: 'Download the installer of the latest version: the .exe for Windows 10 or 11 (with Git for Windows), the .dmg for macOS 11 or later (Apple Silicon or Intel Mac).',
        link: { label: 'Latest release', href: DOWNLOAD },
      },
      {
        title: 'Open a project',
        text: 'Pick a folder, create an agent and write your first request. The next ones come with Ctrl+N (⌘N on Mac).',
      },
    ],
  },

  faq: {
    eyebrow: 'FAQ',
    title: 'Frequently asked questions',
    items: [
      {
        q: 'Is it free?',
        a: 'Yes, and open source, under the MIT license. Escouade uses your own Claude Code: your Claude subscription or your API key, with no middleman.',
      },
      {
        q: 'Where does my data go?',
        a: 'Nowhere: projects, conversations and statistics stay on your machine, in ~/.escouade/. The network is only used by Claude Code itself, to read your quotas, for the app’s updates and, if you connect them, for Jira, Trello or GitHub: an imported ticket gets its status and comments there (its criteria, what was done).',
      },
      {
        q: 'What about my Jira, Trello or GitHub tokens?',
        a: 'They stay on your machine, in the system keychain (Windows Credential Manager, macOS Keychain), and are only used for calls to those services. For GitHub, Escouade can also reuse the token from gh.',
      },
      {
        q: 'Does it work on Mac or Linux?',
        a: 'On Mac, yes: macOS 11 or later, Apple Silicon and Intel alike. Linux isn’t supported yet.',
      },
      {
        q: 'Can autopilot use up my quota?',
        a: 'You choose how many agents work in parallel (from 1 to 6), how many loops each ticket gets at most, and at what percentage of your quotas autopilot pauses. Nothing starts while an agent is waiting for its usage limit to end, and a ticket that reaches its last loop moves to “To test” with “Partial goal”.',
      },
      {
        q: 'Do my .env files end up in commits?',
        a: 'No. The project files that git ignores (.env* by default) are copied into each agent’s worktree so the app can run there; when you validate a ticket, Escouade refuses to commit, push or merge them.',
      },
      {
        q: 'How do updates work?',
        a: 'The app checks for a new version every five minutes and downloads it in the background. It installs without an installer window, at the restart you choose, or on its own when no agent is working and everything is saved, after warning you 30 seconds ahead.',
      },
      {
        q: 'Is it an Anthropic product?',
        a: 'No. Escouade is an independent project, not affiliated with Anthropic. Claude and Claude Code are trademarks of Anthropic.',
      },
    ],
  },

  footer: {
    github: 'GitHub',
    license: 'MIT license',
    note: 'Independent project, not affiliated with Anthropic. Claude and Claude Code are trademarks of Anthropic.',
  },

  notFound: { title: 'Page not found', text: 'There is nothing at this address.', home: 'Back to the home page' },

  demo: {
    status: { ready: 'Ready', running: 'Running', question: 'Question', done: 'Done', totest: 'To test' },
    agents: 'Agents',
    loop: 'loop {loop}/{max}',
    criteria: { one: '{count} of {total} criteria met', other: '{count} of {total} criteria met' },
    tokens: 'tok',
    running: '{count} running',
    waiting: '{count} waiting',
    done: { one: '{count} done', other: '{count} done' },
    today: 'Today ≈ {amount}',
    pause: 'Pause the demonstration',
    caption: 'Animated demonstration · made-up data',
    description:
      ': five Claude Code agents of the demo-api project work in parallel; one asks a question you can answer, and a ticket agent loops on its criteria until “To test”.',
  },

  squad: {
    names: { auth: 'refactor-auth', e2e: 'e2e-tests', docs: 'docs-api', login: 'fix-login' },
    ticketSlugs: ['paginate-users', 'filter-by-role', 'export-to-csv'],
    question: { text: 'Run the whole e2e suite (38 tests)?', options: ['Yes', 'Auth only'] },
    results: {
      files: { one: '{count} file changed', other: '{count} files changed' },
      tests: { one: '{count} test passed', other: '{count} tests passed' },
      met: 'Criteria met · ready to test',
    },
  },
};
