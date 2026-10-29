// Everything the English page says. fr.ts says the same, key for key.
import type { Catalog } from './catalog';
import { DOWNLOAD, SUPPORT } from './site';
import { docPath } from './docs';

export const en: Catalog = {
  meta: {
    title: 'Escouade — the development environment built for agents',
    description:
      'Escouade is a free, open source agentic development environment (ADE) for Windows, macOS and Linux: plan in a Kanban, have agents code in parallel, each in its own worktree, then review, test and ship without leaving the window.',
    locale: 'en_US',
  },

  header: {
    sections: 'Sections',
    video: 'Video',
    features: 'Features',
    install: 'Install',
    roadmap: 'Roadmap',
    faq: 'FAQ',
    github: 'GitHub',
    discord: 'Discord',
    docs: 'Docs',
    download: 'Download',
    language: 'Language',
  },

  anchors: { features: 'features', install: 'install', roadmap: 'roadmap' },

  hero: {
    line1: 'The development environment',
    line2: 'built for agents.',
    lede: 'Plan, code, review, test, ship: the whole cycle runs with agents working in parallel, in a free, open source ADE that runs on your machine.',
    download: 'Download for Windows, macOS and Linux',
    github: 'View on GitHub',
    meta: 'Version {version} · Windows 10 and 11, macOS 11 or later, Linux · Free and open source (GPL-3.0)',
    video: 'Watch the video',
  },

  spotlight: {
    title: 'The one that needs you comes to the front.',
    text: 'A question, a permission request, a finished task: the agent says so, and you answer without hunting for the terminal it is waiting in. Try it: answer e2e-tests.',
    points: [
      'Ctrl+J jumps to the next agent that is waiting',
      'The overview shows every agent, the waiting ones first',
      'A chime, a system notification, a blinking tab: you choose what notifies you',
    ],
  },

  video: { title: 'Escouade at work.' },

  features: {
    title: 'From ticket to merge.',
    lead: 'The same five steps as ever. The difference: a squad of agents works through them with you, in parallel.',
    stages: {
      plan: {
        title: 'Plan',
        text: 'Write down what needs doing, or import it: each ticket carries its acceptance criteria, and autopilot hands it to an agent.',
        points: [
          {
            title: 'Screenshots and files in the ticket',
            text: 'Paste a screenshot or a mockup, attach a log or a spreadsheet: the ticket’s agent gets them with its instructions.',
          },
          {
            title: 'Tickets that wait for each other',
            text: 'A ticket can wait for others before it starts, or pick up a branch that already exists.',
          },
          {
            title: 'An autopilot that spares your quota',
            text: 'From 1 to 6 agents in parallel, a number of loops per ticket, and a pause near the limit of your quotas.',
          },
        ],
      },
      code: {
        title: 'Code',
        text: 'Agents in parallel, each in its own worktree: you follow their work live, and take over in the editor or a terminal.',
        points: [
          {
            title: 'Subagents and background tasks',
            text: 'What an agent starts shows in the sidebar: each subagent with its conversation, each background command with its live output.',
          },
          {
            title: 'A plan’s progress',
            text: 'A banner says where an agent following a plan stands: the tasks done, the subagents at work, the task that is blocked.',
          },
          {
            title: 'Attach any file',
            text: 'A screenshot of the bug, a PDF, an archive, a 150 MB log: drop them into the message, the agent reads them.',
          },
        ],
      },
      review: {
        title: 'Review and test',
        text: 'Every change gets reviewed before it goes in: diffs, history and branches, then the app started on the ticket’s feature.',
        points: [
          {
            title: 'Branches without commands',
            text: 'Switch branches, create them, clean up the merged ones, start an agent on a branch that already exists.',
          },
          {
            title: 'Real terminals',
            text: 'PowerShell, Git Bash and WSL on Windows, zsh, bash and fish elsewhere, with your shell’s autocompletion, to run your tests by hand, even in an agent’s worktree.',
          },
          {
            title: 'Search everything',
            text: 'Ctrl+K searches the conversations, commands and tool outputs of all your agents.',
          },
        ],
      },
      ship: {
        title: 'Ship',
        text: 'A ticket you approve goes out the way you set it up: commit, merge into the target branch, push or pull request. Escouade makes the moves, the decision stays yours.',
        points: [
          {
            title: 'A commit you review',
            text: 'Written by the agent, or a proposed message you read before you click, in your team’s language.',
          },
          {
            title: 'Merge, squash or rebase',
            text: 'An agent’s or a ticket’s worktree goes into its target branch, then gets cleaned up. A conflict goes back to the agent, which resolves it.',
          },
          {
            title: 'Push or pull request',
            text: 'A ticket’s pull request opens with its description, its progress and its acceptance criteria.',
          },
          {
            title: 'Your .env files stay with you',
            text: 'The files git ignores are copied into each worktree so the app runs there, and are never committed, pushed or merged.',
          },
        ],
      },
      steer: {
        title: 'Steer',
        text: 'You lead the squad without watching over it: a signal when you are needed, cost and quotas live, and an app your agents can drive too.',
        points: [
          {
            title: 'Several accounts',
            text: 'Personal, work: each with its own quotas and statistics, and an agent stopped by the usage limit resumes on another account.',
          },
          {
            title: 'An MCP server',
            text: 'An agent, or your assistant outside Escouade, reads your projects, creates and drives tickets, writes to agents and answers their questions, within autopilot’s safeguards.',
          },
          {
            title: 'The overview',
            text: 'Every agent of every project on one page, the ones waiting for your answer first.',
          },
        ],
      },
    },
    items: {
      agents: {
        title: 'Projects in tabs, agents in parallel',
        text: 'One tab per project, and in each as many agents as you like: each with its own conversation, model, effort and, if you want, its own git worktree. Status, tokens, cost and files touched, live.',
        alt: 'Escouade’s window: the projects’ tiles on the left, agents on Claude Code, Codex and Cursor, and one agent’s conversation with its plan and the menu of its account’s models',
      },
      chat: {
        title: 'A real chat, or Claude Code’s own interface: your choice',
        text: 'Markdown, highlighted code, tool calls that unfold into a diff, questions and permission requests as cards you answer with a click. Prefer Claude Code’s interface? One button moves the agent there, in a terminal, without losing its session, and Escouade still follows it: notifications, cost, plan.',
        alt: 'A conversation: a tool call unfolded to its diff, the tests passed and two screenshots sent by the agent',
      },
      notifications: {
        title: 'You know when you’re needed',
        text: 'A question, a finished task: the tab and the card blink, a chime sounds, a system notification says what is asked. Ctrl+J jumps to the next agent that is waiting, and closing the window stops nothing.',
        alt: 'An agent waiting for your answer: its card at the top of the list, the badge on the project’s tile and its question with its choices',
      },
      git: {
        title: 'Git in plain sight',
        text: 'Uncommitted files per agent, side-by-side diffs, the repository’s git graph. A commit written by the agent or a proposed message you review, then a merge or squash of a worktree into its branch.',
        alt: 'The conversation and the files side by side: the uncommitted changes of a project with two repositories, and the diff of one of them',
      },
      editeur: {
        title: 'An editor, without leaving the app',
        text: 'File tree, tabs, syntax highlighting and changed lines in the gutter. Ctrl+click goes to a definition without a language server, Ctrl+P opens a file by name, Ctrl+Shift+F searches every file.',
        alt: 'The built-in editor: the project’s file tree, tabs and the code with its changed lines in the gutter',
      },
      tableau: {
        title: 'Tickets that agents pick up on their own',
        text: 'Each project’s Kanban: write a ticket and its acceptance criteria. Autopilot on, agents pick them up, each in its own worktree, and loop until every criterion is met.',
        alt: 'A project’s Kanban: tickets to do, two in progress on Codex and Cursor, one to review with its criteria met, and the merged tickets',
      },
      test: {
        title: 'Test each ticket in one click',
        text: 'A ticket moves to “To review”: “Test” shows you the agent’s test plan, starts its servers on their reserved ports and opens your browser on the feature.',
        alt: 'The “Test DEM-4” window: preparation done, two servers ready and the address opened in the browser',
      },
      integrations: {
        title: 'Your Jira, Trello, GitHub, Linear, ClickUp, GitLab and Azure Boards tickets, in the Kanban',
        text: 'Link a source to your project, “Import”, tick: the tickets arrive with their acceptance criteria. Escouade keeps their status up to date and comments on the original ticket when it is ready for review, then done.',
        alt: 'The “Import tickets” window: three Jira tickets ticked, with their acceptance criteria detected',
      },
      lancement: {
        title: 'Launch your project in one click',
        text: 'The commands that run your project (front end, API, worker…), written by you or suggested from the project, each in its own terminal with its live status. A crash shows right away, with its exit code.',
        alt: 'A project’s launch commands, one of them crashed, and the dev server’s log',
      },
      stats: {
        title: 'Tokens, cost, quotas: live',
        text: 'The status bar tracks your 5-hour quota, your weekly quota and the day’s cost, which climbs while your agents work. The breakdown by project, model, agent and ticket is in the statistics.',
        alt: 'The statistics page: tokens per day over two weeks, cost by project and by model',
      },
      remote: {
        title: 'And from your phone',
        text: 'Turn on remote control for an agent: its session opens on claude.ai and in the Claude mobile app, and what you send from there also shows up in Escouade.',
        alt: 'An agent in remote control: a message sent from claude.ai in its conversation, and its menu with the claude.ai link',
      },
    },
  },

  cards: [
    {
      title: 'English and French',
      text: 'The interface in your language, and a language of its own for commits, pull requests and ticket comments.',
    },
    { title: 'Usage limit? It resumes', text: 'An agent stopped by its usage limit resumes on its own as soon as your quota is back.' },
    { title: 'Always there', text: 'Window closed, Escouade stays in the notification area and resumes each session on restart.' },
    { title: 'Behind a proxy', text: 'HTTP(S) proxy for Claude, quotas, updates, integrations and, if you want, terminals.' },
    { title: 'Automatic updates', text: 'Signed, downloaded in the background, installed without a window when no agent is working.' },
    {
      title: 'From the keyboard',
      text: 'Ctrl+1…9 projects, Ctrl+N an agent, Ctrl+J the one that is waiting, Ctrl+K search, Esc interrupt.',
    },
  ],

  install: {
    title: 'Ready in three steps.',
    steps: [
      {
        title: 'Install a harness',
        text: 'Escouade’s agents run on the harness on your machine: Claude Code, Codex, OpenCode or GitHub Copilot CLI. Install it and sign in once, with your own account.',
        link: { label: 'The four harnesses', href: docPath('en', 'harness') },
      },
      {
        title: 'Install Escouade',
        text: 'The .exe for Windows 10 or 11 (with Git for Windows), the .dmg for macOS 11 or later, on an Apple Silicon or Intel Mac, the AppImage or the .deb for Linux (x86-64).',
        link: { label: 'Latest release', href: DOWNLOAD },
      },
      {
        title: 'Open a project',
        text: 'Pick a folder, create an agent and write your first request. The next ones come with Ctrl+N (⌘N on Mac).',
      },
    ],
  },

  roadmap: {
    title: 'What comes next.',
    lead: 'Your code host in the loop, plugins and their marketplace, and dictation through Wispr Flow. No date promised: the work is planned in detail on GitHub.',
    next: 'Version {version} · in the works',
    items: [
      {
        title: 'Wispr Flow',
        text: 'Wispr Flow’s dictation in the message field and the voice conversation, as an online service and as a desktop app.',
      },
      {
        title: 'GitHub, GitLab, Bitbucket and Gitea',
        text: 'Tickets’ pull requests followed through to the merge, with their CI and their deployments, without leaving Escouade.',
      },
      {
        title: 'Plugins and their marketplace',
        text: 'Escouade grows through plugins, and every external integration becomes one: code hosts, ticket sources, harnesses. A marketplace gathers them, reviewed before they get in.',
      },
    ],
    plan: 'The detailed plan (in French)',
    changelog: 'What is already out',
  },

  faq: {
    title: 'Frequently asked questions.',
    items: [
      {
        q: 'What is an ADE?',
        a: 'An agentic development environment: the place where you build software with agents from start to finish, not just where you talk to them. In Escouade, you write the tickets, agents code in parallel, each in its own worktree, and you review, test and ship in the same place: Kanban, conversation, editor, terminals, Git and tests.',
      },
      {
        q: 'Does it replace my editor?',
        a: 'Not necessarily. Escouade’s editor is enough to review and touch up what the agents do (file tree, tabs, search across files, go to a definition), but it has no language server and no extensions. Keep your IDE for the rest: each worktree is a real folder, which you open wherever you like.',
      },
      {
        q: 'Which harness runs the agents?',
        a: 'The one on your machine, with your own account: Claude Code (Claude subscription or API key), Codex (ChatGPT account), OpenCode (the provider you choose, local models included) or GitHub Copilot CLI (GitHub account with Copilot). Each agent has its own, and the documentation compares them. Cursor CLI comes later.',
      },
      {
        q: 'Is it free?',
        a: 'Yes, and open source, under the GPL-3.0 license (or later). Escouade uses your own harness and your own account, with no middleman. If it helps you, you can support the project:',
        link: { label: 'buy a coffee', href: SUPPORT.en },
      },
      {
        q: 'Where does my data go?',
        a: 'Nowhere: projects, conversations and statistics stay on your machine, in ~/.escouade/. The network is only used by the harnesses themselves, to read your quotas, for updates and, if you connect them, for Jira, Trello, GitHub, Linear, ClickUp, GitLab or Azure Boards, where an imported ticket gets its status and comments.',
      },
      {
        q: 'What about my Jira, Trello, GitHub, Linear, ClickUp, GitLab or Azure Boards tokens?',
        a: 'They stay on your machine: in the system keychain (Windows Credential Manager, macOS Keychain), or on Linux in the integrations file of ~/.escouade/, and are only used for calls to those services. For GitHub, Escouade can also reuse the token from gh.',
      },
      {
        q: 'I prefer Claude Code’s interface. Can I keep it?',
        a: 'Yes. The “Claude Code view” button, in an agent’s header, runs it in Claude Code’s own interface, in a terminal: your / commands, your shortcuts, your dialogs. “Escouade view” brings it back to the chat without losing the session. A setting opens new agents in Claude Code’s interface straight away. Ticket agents, which the autopilot drives, stay in the chat.',
      },
      {
        q: 'Does it work on Mac or Linux?',
        a: 'Yes. On Mac: macOS 11 or later, Apple Silicon and Intel alike. On Linux (x86-64): the AppImage, which updates itself, or the .deb for Debian and Ubuntu.',
      },
      {
        q: 'Can autopilot use up my quota?',
        a: 'You choose how many agents work in parallel (from 1 to 6), how many loops each ticket gets at most, and at what percentage of your quotas autopilot pauses. Nothing starts while an agent is waiting for its usage limit to end, and a ticket that reaches its last loop moves to “To review” with “Partial goal”.',
      },
      {
        q: 'Do my .env files end up in commits?',
        a: 'No. The project files that git ignores (.env* by default) are copied into each agent’s worktree so the app can run there; when you approve a ticket, Escouade refuses to commit, push or merge them.',
      },
      {
        q: 'How do updates work?',
        a: 'The app checks for a new version every five minutes and downloads it in the background. It installs without an installer window, at the restart you choose, or on its own when no agent is working and everything is saved, after warning you 30 seconds ahead.',
      },
      {
        q: 'Is it an Anthropic product?',
        a: 'No. Escouade is an independent project, not affiliated with Anthropic, OpenAI or GitHub. Claude and Claude Code are trademarks of Anthropic, Codex of OpenAI, GitHub Copilot of GitHub.',
      },
    ],
  },

  close: { title: 'Your next development environment.' },

  footer: {
    github: 'GitHub',
    discord: 'Discord',
    support: 'Buy a coffee',
    license: 'GPL-3.0 license',
    privacy: 'Privacy',
    legal: 'Legal notice',
    note: 'Independent project, not affiliated with Anthropic, OpenAI or GitHub. Claude and Claude Code are trademarks of Anthropic, Codex of OpenAI, GitHub Copilot of GitHub.',
  },

  notFound: { title: 'Page not found', text: 'There is nothing at this address.', home: 'Back to the home page' },
  failure: { title: 'Something went wrong', home: 'Back to the home page' },

  demo: {
    status: { ready: 'Ready', running: 'Running', question: 'Question', done: 'Done', totest: 'To review' },
    agents: 'Agents',
    loop: 'loop {loop}/{max}',
    criteria: { one: '{count} of {total} criteria met', other: '{count} of {total} criteria met' },
    kanban: 'Kanban',
    projects: 'Projects',
    running: '{count} running',
    waiting: '{count} waiting',
    done: { one: '{count} done', other: '{count} done' },
    today: 'Today ≈ {amount}',
    pause: 'Pause the demonstration',
    caption: 'Animated demonstration · made-up data',
    description:
      ': five agents of the demo-api project work in parallel; one asks a question you can answer, and a ticket agent loops on its criteria until “To review”.',
  },

  squad: {
    names: { auth: 'refactor-auth', e2e: 'e2e-tests', docs: 'docs-api', login: 'fix-login' },
    ticketSlugs: ['paginate-users', 'filter-by-role', 'export-to-csv'],
    question: { text: 'Run the whole e2e suite (38 tests)?', options: ['Yes', 'Auth only'] },
    results: {
      files: { one: '{count} file changed', other: '{count} files changed' },
      tests: { one: '{count} test passed', other: '{count} tests passed' },
      met: 'Criteria met · ready for review',
    },
  },

  docs: {
    title: 'Documentation',
    lead: 'Everything Escouade does, and how to use it: from installing it to autopilot, every feature and every setting.',
    meta: {
      title: 'Escouade documentation',
      description:
        'The documentation of Escouade, the agentic development environment: installation, agents and chat, Git and worktrees, the Kanban and autopilot, integrations, accounts, settings and shortcuts.',
    },
    pageTitle: '{title} · Escouade documentation',
    groups: {
      start: { title: 'Get started', text: 'What Escouade is, installing it, choosing your harnesses and your first steps.' },
      agents: {
        title: 'Agents and chat',
        text: 'Start agents, talk to them, follow their work and hear from them when they need you.',
      },
      code: {
        title: 'Code and Git',
        text: 'Review and commit their changes, give each agent its own worktree, edit files, run the project.',
      },
      board: {
        title: 'Kanban and automation',
        text: 'Hand tickets to autopilot, review and approve them, import them, drive Escouade over MCP.',
      },
      usage: { title: 'Accounts and usage', text: 'Several accounts per harness, quotas, cost and statistics.' },
      settings: { title: 'Settings', text: 'Every tab of the settings window, option by option.' },
      plugins: {
        title: 'Plugins',
        text: 'Add features to Escouade: a plugin’s manifest, how it is installed, its protocol, its extension points.',
      },
      reference: { title: 'Reference', text: 'Shortcuts, local data, updates and troubleshooting.' },
    },
    pages: {
      introduction: 'Introduction',
      installation: 'Installation',
      harness: 'The harnesses',
      'quick-start': 'Quick start',
      interface: 'The interface',
      'managing-agents': 'Agents',
      chat: 'The conversation',
      plan: 'Plan progress',
      overview: 'Keeping track of agents',
      notifications: 'Notifications',
      'claude-code-view': 'The terminal view',
      'remote-control': 'Remote control',
      git: 'Git and commits',
      worktrees: 'Worktrees',
      branches: 'Branches',
      editor: 'The editor',
      terminals: 'Terminals',
      launch: 'Running the project',
      board: 'The Kanban',
      autopilot: 'Autopilot',
      testing: 'Review and approve',
      integrations: 'External tickets',
      'shared-board': 'Sharing the Kanban',
      'docs-space': 'The Docs space',
      workflows: 'Workflows',
      'shared-memory': 'Shared memory',
      'code-index': 'Code index',
      mcp: 'The MCP server',
      accounts: 'Several accounts',
      usage: 'Quotas and cost',
      stats: 'Statistics',
      'settings-application': 'Application',
      'settings-claude-code': 'Harness',
      'settings-accounts': 'Accounts',
      'settings-notifications': 'Notifications',
      'settings-projects': 'Projects',
      'settings-board': 'Kanban',
      'settings-integrations': 'Integrations',
      'settings-terminals': 'Terminals',
      'settings-network': 'Network',
      'settings-voice': 'Voice',
      'settings-about': 'About',
      'plugins/installer': 'Installing a plugin',
      'plugins/manifest': 'The manifest',
      'plugins/api': 'The protocol',
      'plugins/sdk': 'The SDK',
      'plugins/testing': 'Testing a plugin',
      'plugins/themes': 'Themes',
      'plugins/publishing': 'Publishing on the store',
      shortcuts: 'Keyboard shortcuts',
      'escouade-yml': '.escouade.yml',
      data: 'Local data',
      updates: 'Updates',
      troubleshooting: 'Troubleshooting',
    },
    nav: 'Documentation pages',
    menu: 'Contents',
    toc: 'On this page',
    previous: 'Previous',
    next: 'Next',
    edit: 'Edit this page on GitHub',
    callouts: { note: 'Note', tip: 'Tip', warning: 'Warning' },
    search: {
      button: 'Search',
      placeholder: 'Search the documentation',
      empty: 'Nothing found for “{query}”.',
      keys: '↑ ↓ to choose, Enter to open, Esc to close',
      failed: 'The search could not load the pages. Check your connection and open it again.',
      close: 'Close',
    },
  },
};
