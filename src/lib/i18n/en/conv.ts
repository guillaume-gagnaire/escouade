import { defineZone } from '../types';

// The conversation with an agent.
export default defineZone('conv', {
  status: { running: 'Running', waiting: 'Question', idle: 'Ready', done: 'Done', error: 'Error' },

  header: {
    renameTitle: 'Rename the agent',
    renameHint: 'Double-click to rename',
    editorHint: 'Browse and edit the files of this agent',
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
    layout: 'Layout ({key})',
    layoutClassic: 'Classic layout',
    layoutSplit: 'Conversation and files side by side',
  },

  loadError: 'Couldn’t load the conversation: {error}',
  empty: {
    title: 'Agent ready',
    hint: 'Describe the task to give to Claude. The agent works in {cwd}.',
    claudeMissing: 'Claude Code couldn’t be found on this machine: install it or set its path in the settings (⚙).',
  },
  older: {
    show: { one: 'Show the previous message', other: 'Show the previous {n} messages' },
    drawn: { one: '{n} previous message shown', other: '{n} previous messages shown' },
  },
  working: 'Claude is working…',
  newMessages: 'New messages',
  archived: { text: 'Agent archived.', restore: 'Restore' },
  stopped: 'The agent stopped. Send a message to restart Claude on the same session.',

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
    queuedHint: 'Claude takes it into account at its next step',
    remote: 'from claude.ai',
    remoteHint: 'Sent from claude.ai or the Claude app (remote control)',
  },

  event: {
    taskCompleted: 'Background task completed',
    taskFailed: 'Background task failed',
    taskStopped: 'Background task stopped',
    taskStatus: 'Background task {status}',
    notification: 'Notification',
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
  },

  markdown: { plainText: 'text', copied: 'Copied ✓' },
});
