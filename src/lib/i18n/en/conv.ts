import { defineZone } from '../types';

// The conversation with an agent.
export default defineZone('conv', {
  status: { running: 'Running', waiting: 'Question', idle: 'Ready', done: 'Done', error: 'Error' },

  header: {
    renameTitle: 'Rename the agent',
    renameHint: 'Double-click to rename',
    test: 'Test',
    testIsolaHint: 'Launch the isola services of this worktree and open the feature in the browser',
    testHint: 'Launch the worktree of this agent and open the feature in the browser',
    prepare: 'Prepare launch',
    prepareHint: 'Ask the agent how to launch its worktree, on its own ports',
    context: 'Context',
    contextNow: 'Current context',
    tokens: 'Tokens',
    duration: 'Duration',
    filesHint: 'Show uncommitted files',
    filesClose: 'Close the files',
    layout: 'Layout ({key})',
    layoutClassic: 'Classic layout',
    layoutSplit: 'Conversation and files side by side',
    toTerminal: '{engine} view',
    toTerminalHint: 'Run the agent in {engine}’s own interface, in a terminal',
    toRich: 'Escouade view',
    toRichHint: 'Back to Escouade’s conversation (the session goes on)',
    view: 'Agent view',
    rich: 'Escouade',
    more: 'Agent actions',
    sandbox: 'Sandbox',
    sandboxLabel: 'Sandbox. {meaning}',
    sandboxDefault: 'Its commands run in a sandbox.',
  },

  noAccount: {
    body: 'This agent won’t start until a {harness} account is added: the one already signed in on this machine, or a new one. Your message stays in the field.',
  },

  terminal: {
    starting: 'Starting {engine}…',
    exited: '{engine} stopped.',
    exitedAtOnce: '{engine} stopped as soon as it started (code {code}). Read its output above, then restart it.',
    exitedAtOnceNoCode: '{engine} stopped as soon as it started. Read its output above, then restart it.',
    sessionLost: '{speaker} session not found: the interface restarts in a new session.',
    restart: 'Restart',
    failed: '{engine} didn’t start: {error}',
    imageTooBig: '{name} is over {max}: {engine} won’t get it.',
    trust: {
      title: '{engine} asks whether it may trust the agent’s folder.',
      codex: 'Answer in the terminal: without trust, it stops. Escouade already approved its hooks for this process alone.',
      copilot: 'Answer in the terminal: Escouade doesn’t answer for you and types nothing while the question is asked.',
      cursor:
        'Answer in the terminal (“Trust this workspace”): Escouade doesn’t answer for you and types nothing while the question is asked.',
    },
    limited:
      '{title}: Escouade sees your messages, Cursor’s replies and the end of each turn, but not its permissions or questions waiting, nor the results of its tools, nor the tokens: look at the terminal. This conversation is Cursor’s own: Escouade links it to the agent’s.',
    limitedTitle: 'Limited following',
    keeps:
      '{title}: the conversation, the card, the notifications and the cost stay up to date while you work here. Going back to the Escouade view only stops the interface.',
    keepsTitle: 'Escouade keeps the thread',
  },

  escouadeNext: {
    title: 'Escouade’s tools at the next start',
    text: 'Its process started without Escouade’s tools, which “Agents can drive Escouade” now gives it: restart it to give it the tools. Its session is resumed and its conversation stays.',
    busy: 'It can be restarted once it is done with what it is doing.',
    busyBackground:
      'It still has background tasks (command, monitor, subagent, workflow): restarting it would stop them. A command or a dev server doesn’t end by itself: stop it first, from its view (“Stop”), or wait until it has finished.',
    restart: 'Restart',
  },

  loadError: 'Couldn’t load the conversation: {error}',
  empty: {
    title: 'Agent ready',
    hint: 'Describe the task to give to {engine}. The agent works in {cwd}.',
    engineMissing: '{engine} couldn’t be found on this machine: install it or set its path in Settings › Harness (⚙).',
  },
  older: {
    show: { one: 'Show the previous message', other: 'Show the previous {n} messages' },
    drawn: { one: '{n} previous message shown', other: '{n} previous messages shown' },
  },
  working: '{engine} is working…',
  newMessages: 'New messages',
  archived: { text: 'Agent archived.', restore: 'Restore' },
  stopped: 'The agent stopped. Send a message to restart {engine} on the same session.',

  search: {
    title: 'Search conversations',
    hint: 'Search the messages, commands and files of your agents’ conversations.',
    searching: 'Searching…',
    none: 'No message matches.',
    capped: 'The first {n} results: narrow your search.',
    timedOut: {
      one: '{n} result, search stopped after 5 s: narrow your search.',
      other: '{n} results, search stopped after 5 s: narrow your search.',
    },
    results: { one: '{n} result', other: '{n} results' },
    agentGone: 'This agent no longer exists.',
    scope: 'Where to search',
    thisProject: 'This project',
    allProjects: 'All projects',
    archivedAgents: 'Archived agents',
    resultsLabel: 'Results',
    archived: 'archived',
    keyChoose: 'choose',
    keyOpen: 'open',
    keyClose: 'close',
  },

  user: {
    images: { one: '{count} image', other: '{count} images' },
    waiting: 'Waiting for the setup…',
    queued: 'sent during the turn',
    queuedHint: '{engine} takes it into account at its next step',
    remote: 'from claude.ai',
    remoteHint: 'Sent from claude.ai or the Claude app (remote control)',
    saidAloud: 'Said aloud',
  },

  sent: {
    by: 'Sent by {engine}',
    images: { one: '{count} image', other: '{count} images' },
    files: { one: '{count} file', other: '{count} files' },
    reveal: 'Show in folder',
    revealName: 'Show {name} in its folder',
    openName: 'Open {name}',
    missing: 'File not found',
    missingHint: 'Deleted or moved since it was sent',
    fenced: 'File of a private project, not shown',
    big: 'Image too heavy for a preview',
    loading: 'Loading…',
    view: 'View {name} full size',
    viewer: 'Images sent',
    prev: 'Previous image',
    next: 'Next image',
    close: 'Close (Esc)',
    counter: '{n} of {count}',
    imageMissing: 'Image not found: {src}',
    remoteOf: 'image from {host}',
    remoteShow: 'Show image',
  },

  event: {
    taskCompleted: 'Background task completed',
    taskFailed: 'Background task failed',
    taskStopped: 'Background task stopped',
    taskStatus: 'Background task {status}',
    notification: 'Notification',
    wake: 'Wake-up',
    fromClaudeCode: 'Message from Claude Code ({source})',
    handbackNamed: 'Report from the subagent “{label}”',
    handback: 'Report from a subagent',
    messageNamed: 'Message from “{label}”',
    message: 'Message from another Claude session',
  },

  thinking: { title: 'Thinking' },

  tool: {
    openFile: 'Open {path} in the editor',
    running: 'running',
    subTools: { one: '{count} tool', other: '{count} tools' },
    more: { one: 'Show the rest ({count} character)', other: 'Show the rest ({count} characters)' },
  },
  tools: {
    tasks: { one: '{count} task', other: '{count} tasks' },
    interrupted: 'interrupted',
    error: 'error',
    lines: { one: '{n} line', other: '{n} lines' },
    results: { one: '{n} result', other: '{n} results' },
    noResults: 'no results',
    done: 'done',
    taskCreate: 'Task: {subject}',
    taskUpdate: 'Task #{id}: {status}',
    taskStatus: { pending: 'to do', inProgress: 'in progress', completed: 'done', deleted: 'deleted', changed: 'updated' },
  },
  patch: { more: { one: '… {n} more line', other: '… {n} more lines' } },

  turn: {
    failed: 'The turn ended with an error',
    resumeAt: 'Auto-resume {when}',
    cancelResume: 'Cancel auto-resume',
    signInTitle: 'Sign in again',
    signIn:
      'The {account} account must sign in again: Claude Code couldn’t refresh its session. The agent will go on by itself once you’re signed in.',
    signInAction: 'Sign in again',
    done: 'Task done',
    tokens: '{tokens} tokens',
    filesEdited: { one: '{count} file edited', other: '{count} files edited' },
    editedFiles: 'Edited files',
    interrupted: 'Interrupted',
  },

  setup: {
    running: 'Setting up the worktree · {step} — your messages will be sent once it’s done.',
    showOutput: 'Show output',
    outputLabel: 'Output of {step}',
    noOutput: 'No output yet.',
    later: 'The worktree will be created at the first message, under the agent’s name.',
  },

  markdown: { plainText: 'text', copied: 'Copied ✓' },
});
