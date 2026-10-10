import { defineZone } from '../types';

// The launch commands, their logs, and the test launches.
export default defineZone('runs', {
  /** What a launch command is doing, in the list, above its log and on the lines of a test. */
  status: {
    ready: 'ready',
    stopping: 'stopping…',
    running: 'running',
    stopped: 'stopped',
    done: 'done',
    crashed: 'crashed',
    crashedCode: 'crashed (code {code})',
  },
  /** The buttons of a command (“Stop” and “Stop all” are `common.stop` and `common.stopAll`). */
  action: { run: 'Run', restart: 'Restart', runAll: 'Run all' },
  /** What the log of a command and the toasts about it say. */
  launch: {
    wontStop: '“{name}” won’t stop',
    removed: '“{name}” was removed',
    restartedAt: '— restarted at {time} —',
    /** `{why}`: the reason the backend gave, as received. */
    couldNotStart: '“{name}” couldn’t start: {why}',
    crashed: '“{name}” stopped with an error (code {code})',
  },
  /** A shell is needed to open a terminal; `{expected}` lists the ones the system is expected to have. */
  term: { noShell: 'No shell detected ({expected}). Check the settings.' },
  /** The launch section of the sidebar. */
  section: {
    title: 'Launch',
    configure: 'Launch commands…',
    none: 'No commands.',
    set: 'Configure',
    suggest: 'Suggest commands',
  },
  /** The head and the empty state of the log of a command. */
  log: {
    searchKey: 'Search ({key})',
    since: 'since {time}',
    notRun: 'Not run yet.',
  },
  /** The terminal view. */
  terminal: {
    close: 'Close terminal',
    exited: 'Process exited.',
    exitedCode: 'Process exited (code {code}).',
  },
  /** The steps of an agent's recipe, as launch commands, and its hidden characters spelled out. */
  recipe: {
    prepStep: 'Setup {n}',
    process: 'process {n}',
    /** The counts are 24 and over (spaces) or 2 and over (lines): always a plural. */
    spaces: '⟨{n} spaces⟩',
    spacesInvisible: '⟨{n} spaces or invisible characters⟩',
    emptyLines: '⟨{n} blank lines⟩',
    emptyLinesInvisible: '⟨{n} blank lines or invisible characters⟩',
  },
  /** The lines and the error of a test under way, written as it goes. */
  flow: {
    prepare: 'Setup: {command}',
    running: 'running…',
    starting: 'starting…',
    started: 'started',
    waitingFor: 'waiting for {host}…',
    readyIn: 'ready · {seconds} s',
    noAnswer: 'No response from {url} after 3 min',
    notWaited: 'skipped',
    code: 'code {code}',
    prepStopped: 'stopped',
    prepDone: 'done',
    prepFailed: 'Setup failed (code {code})',
    isolaFailed: 'isola up failed (code {code})',
    stopped: 'Stopped',
    couldNotStart: '“{name}” couldn’t start',
    exited: '{name} stopped',
    notRunning: '{name} isn’t running',
  },
  /** The window of “▶ Test”. */
  testLaunch: {
    title: 'Test {name}',
    steps: 'Launch steps',
    viewLog: 'View log',
    viewLogs: 'View logs',
    reopen: 'Reopen',
    /** `{address}`: the address opened, in code style. */
    opened: 'Opened in the browser: {address}',
    reading: 'Reading .isola.toml…',
    changed: 'The recipe just changed: read it again before running.',
    /** `{name}`: the agent. */
    isolaNotice:
      'isola runs the commands in this file in your shell, outside Claude Code’s permission mode. {name} may have written or edited it.',
    isolaConfig: 'isola configuration (.isola.toml)',
    recipeNotice: 'These commands were written by {name}. They run in your shell, outside Claude Code’s permission mode.',
    prepare: 'Setup',
    launch: 'Launch',
    root: 'the worktree root',
    variable: 'Variable',
    address: 'Address',
    opening: 'Opens in the browser',
    recipeChanged: 'The recipe changed: press ▶ Test again.',
  },
});
