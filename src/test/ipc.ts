// Test utilities: a fake Tauri backend recording every command.

import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { buffers } from '../lib/editor/buffers.svelte';
import { navHistory } from '../lib/editor/history';
import { recentFiles } from '../lib/editor/quick-open';
import { fileSearches } from '../lib/editor/search.svelte';
import { gitSync } from '../lib/git-sync.svelte';
import { mcp } from '../lib/mcp.svelte';
import { dictation } from '../lib/dictation.svelte';
import { conversation } from '../lib/conversation.svelte';
import { clearDrafts } from '../lib/question-answers.svelte';
import { trees } from '../lib/editor/trees.svelte';
import { app } from '../lib/state.svelte';
import { workflows } from '../lib/workflows.svelte';
import type {
  AccountUsage,
  Agent,
  BoardSettings,
  BranchInfo,
  EngineDecl,
  GitInfo,
  Project,
  PullRequest,
  QuotaMeter,
  RateWindow,
  Settings,
  Ticket,
  Usage,
} from '../lib/types';
import claudeEngine from '../../tests/fixtures/engines/claude.json';

/** Claude Code as the backend declares it (the Rust tests keep the file the same as its declaration). */
export const CLAUDE_ENGINE = claudeEngine as EngineDecl;

export interface Call {
  cmd: string;
  args: Record<string, any>;
}

/** What a command answers when a test gives it no handler: nothing (null), except the lists the window reads on its own. */
const ANSWERS: Record<string, unknown> = { closed_projects: [], memory_list: [], claude_memory_offer: [] };

export function fakeBackend(handlers: Record<string, (args: any) => unknown> = {}) {
  const calls: Call[] = [];
  mockWindows('main');
  mockIPC(
    (cmd, args) => {
      calls.push({ cmd, args: (args ?? {}) as Record<string, any> });
      const h = handlers[cmd];
      return h ? h(args) : (ANSWERS[cmd] ?? null);
    },
    { shouldMockEvents: true },
  );
  return {
    calls,
    called: (cmd: string) => calls.filter((c) => c.cmd === cmd),
  };
}

export const SETTINGS: Settings = {
  claudePath: '',
  sound: true,
  osNotifications: true,
  notifyFor: { questions: true, done: true, errors: true, tickets: true },
  idleStopMinutes: 30,
  attachMaxMb: 0,
  attachWarnMb: 200,
  attachOrigin: true,
  attachBigText: 'path',
  attachKeepDays: 0,
  // The tests opt in to the delay: a message sent during a turn goes at once, as before.
  midTurnDelaySecs: 0,
  pwshPath: '',
  bashPath: '',
  wslDistro: '',
  proxyUrl: '',
  noProxy: 'localhost',
  proxyTerminals: false,
  insecureTls: false,
  autoResume: true,
  switchOnLimit: false,
  handoverOnLimit: false,
  handoverModels: {},
  quotaPause: 100,
  switchAt: 100,
  autoUpdate: true,
  telemetry: false,
  // The first start's question of the name for the shared tickets (ESC-190): asked already.
  nameAsked: true,
  integrations: {
    syncStates: true,
    loopComments: false,
    extractCriteria: true,
    autoImport: false,
    importLabel: 'claude-ready',
    importEvery: 15,
  },
  language: 'system',
  claudeLanguage: 'ui',
  writerEngine: null,
  accounts: [{ id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true }],
  mcpEnabled: false,
  mcpPort: 0,
  todoTools: true,
  claudeSelfUpdate: true,
  terminalView: false,
  remoteControlAll: false,
  privateProjects: false,
  theme: 'system',
  uiZoom: 100,
  quotaProjection: 'alert',
  terminalFontSize: 12.5,
  editorFontSize: 12.5,
  railWidth: 0,
  sideWidth: 0,
  voice: {
    device: '',
    model: 'base',
    language: 'ui',
    shortcut: 'Mod+Shift+Space',
    speech: {
      voices: {},
      speakers: {},
      rate: 1,
      volume: 1,
      silenceMs: 1200,
      engine: 'local',
      onlineService: 'openai',
      onlineModels: {},
      onlineVoices: {},
    },
    transcription: 'local',
    onlineService: 'openai',
    onlineModels: {},
  },
};

export function project(over: Partial<Project> = {}): Project {
  return {
    id: 'p1',
    name: 'demo-api',
    path: 'C:\\code\\demo-api',
    color: 'oklch(0.72 0.12 48)',
    worktreePerAgent: false,
    createdAt: 1,
    runCommands: [],
    board: board(),
    worktreeCopy: ['.env*'],
    worktreeSetup: [],
    worktreeTeardown: [],
    integrations: { links: [], comments: ['review', 'done'] },
    commitMode: 'agent',
    account: '',
    worktreeDir: '',
    agentBranchPrefix: 'escouade/',
    ticketBranchPrefix: 'ticket/',
    remoteControl: false,
    engine: null,
    private: false,
    sharedOnLimit: false,
    ...over,
  };
}

export function agent(over: Partial<Agent> = {}): Agent {
  return {
    id: 'a1',
    projectId: 'p1',
    name: 'refacto-auth',
    named: true,
    engine: 'claude',
    model: 'opus',
    effort: 'high',
    mode: 'auto',
    sessionId: null,
    lastEntry: null,
    forkOf: null,
    forkAt: null,
    cwd: 'C:\\code\\demo-api',
    worktree: null,
    createdAt: 1,
    archived: false,
    status: 'done',
    tokens: 0,
    cost: 0,
    activeMs: 0,
    touchedFiles: [],
    lastActivity: 1,
    prompts: 0,
    activeSince: null,
    alive: true,
    escouadeNext: false,
    pending: [],
    requests: [],
    contextTokens: 0,
    contextWindow: 0,
    liveTokens: 0,
    liveCost: 0,
    remoteControl: false,
    remoteSource: null,
    remoteOff: false,
    interfaceRemote: null,
    remoteSession: null,
    remoteUrl: null,
    remoteState: null,
    resumeAt: null,
    ticketId: null,
    progressLine: null,
    prs: [],
    prWord: null,
    appendPrompt: null,
    portBase: null,
    recipe: null,
    approvedRecipe: null,
    approvedIsola: null,
    account: 'principal',
    movedFrom: null,
    handedFrom: null,
    signIn: null,
    activity: null,
    setup: null,
    terminal: false,
    attached: false,
    terminalId: null,
    tuiExit: null,
    isola: false,
    ...over,
  };
}

/** An agent's pull request (ESC-64), open, its CI green. */
export function pullRequest(over: Partial<PullRequest> = {}): PullRequest {
  return {
    number: 7,
    url: 'https://github.com/acme/demo/pull/7',
    title: 'Connexion par jeton',
    state: 'open',
    draft: false,
    head: 'escouade/login',
    base: 'main',
    mergeable: true,
    approvals: 0,
    changesRequested: false,
    openThreads: 0,
    autoMerge: false,
    checks: 'success',
    fork: false,
    ...over,
  };
}

export function board(over: Partial<BoardSettings> = {}): BoardSettings {
  return {
    action: 'merge',
    target: '',
    strategy: 'squash',
    draft: false,
    testsFirst: false,
    testCommand: '',
    cleanup: true,
    conventional: true,
    conflict: 'ask',
    maxParallel: 2,
    model: '',
    effort: '',
    mode: '',
    autopilot: true,
    prefix: '',
    nextNumber: 1,
    ...over,
  };
}

export function ticket(over: Partial<Ticket> = {}): Ticket {
  return {
    id: 't1',
    projectId: 'p1',
    key: 'DEM-1',
    title: 'Ajouter le fichier',
    description: '',
    criteria: [
      { text: 'Le fichier existe', ok: false, note: '' },
      { text: 'Tests verts', ok: false, note: '' },
    ],
    progress: [],
    maxLoops: 5,
    column: 'todo',
    rank: 1,
    after: [],
    agentId: null,
    iteration: 0,
    loops: 0,
    partial: false,
    blocked: null,
    conflict: false,
    step: null,
    outcome: null,
    outcomeClaude: null,
    outcomeUrl: null,
    outcomeCommit: null,
    forced: false,
    reminded: false,
    cost: 0,
    createdAt: 1,
    startedAt: null,
    reviewAt: null,
    doneAt: null,
    external: null,
    branch: '',
    model: '',
    effort: '',
    account: '',
    accountLost: null,
    engine: null,
    images: [],
    files: [],
    ...over,
  };
}

export function gitInfo(over: Partial<GitInfo> = {}): GitInfo {
  return {
    isRepo: true,
    branch: 'main',
    upstream: null,
    upstreamGone: false,
    ahead: 0,
    behind: 0,
    hasRemote: false,
    lastFetch: null,
    modified: 0,
    added: 0,
    deleted: 0,
    total: 0,
    agents: {},
    ...over,
  };
}

/** A local branch of the project's repository, as `branchList` gives it. */
export function branchInfo(over: Partial<BranchInfo> = {}): BranchInfo {
  return {
    name: 'main',
    remote: false,
    current: false,
    upstream: null,
    upstreamGone: false,
    ahead: 0,
    behind: 0,
    worktree: null,
    trackedBy: null,
    agent: null,
    lastCommitAt: 1,
    ...over,
  };
}

/** Puts the app singleton back into a known state. */
export function resetApp(over: { projects?: Project[]; agents?: Agent[]; tickets?: Ticket[] } = {}) {
  const projects = over.projects ?? [project()];
  app.projects = projects;
  app.agents = Object.fromEntries((over.agents ?? []).map((a) => [a.id, a]));
  app.attention = {};
  app.tickets = Object.fromEntries((over.tickets ?? []).map((t) => [t.id, t]));
  app.board = {};
  app.docs = {};
  app.docsSpace = {};
  app.docsPage = {};
  app.docsEdit = null;
  app.docsShare = {};
  app.boardIssues = {};
  app.setupOutput = {};
  app.relaunched = {};
  app.autopilotPause = null;
  app.claudePathFound = true;
  app.engines = [CLAUDE_ENGINE];
  app.enginesFound = ['claude'];
  app.accountModels = {};
  app.engineModels = {};
  app.projectPauses = {};
  app.editor = {};
  app.ui = { activeProject: projects[0]?.id ?? null, view: 'project', selectedAgent: {} };
  app.settings = { ...SETTINGS };
  app.usage = { todayCost: 0, accounts: [], current: {} };
  app.resources = { instances: 0, memory: 0, cpu: 0, agents: [] };
  app.git = {};
  gitSync.running = {};
  app.shells = [];
  app.terminals = [];
  app.exitedTerms = {};
  app.launches = {};
  app.selectedLaunch = {};
  app.selectedTerm = {};
  app.filesOpen = false;
  app.planOpen = {};
  app.subView = {};
  app.subDrafts = {};
  app.subFocus = { from: null, back: null };
  app.subsOpen = {};
  app.taskView = {};
  app.bgOpen = {};
  app.filesScope = 'agent';
  app.panelTab = 'files';
  app.composerPut = null;
  app.draftReturned = {};
  workflows.reset();
  app.diffSplit = false;
  app.modal = null;
  app.toasts = [];
  app.update = null;
  app.restartAt = null;
  app.failedUpdate = null;
  app.updateTrouble = null;
  app.models = [];
  app.accounts = [];
  // French all along, as the tests (the language itself is put back by the setup).
  app.lang = { ui: 'fr', system: 'fr', claude: 'fr' };
  app.ready = true;
  buffers.reset();
  mcp.reset();
  dictation.clear();
  conversation.clear();
  clearDrafts();
  trees.reset();
  navHistory.reset();
  recentFiles.reset();
  fileSearches.reset();
}

/**
 * A harness that does nothing Claude Code alone does: no effort, attachments, copy, `/` commands, quota, cost, terminal
 * view, remote control or background tasks, and two modes, one with a meaning of its own. What the window shows for
 * its agents is what any harness has.
 */
export const BARE_ENGINE: EngineDecl = {
  id: 'codex',
  name: 'Harness nu',
  speaker: 'Nu',
  tested: { oldest: '0.160.0', newest: '0.163.0' },
  caps: {
    modes: [
      { id: 'default', meaning: null },
      { id: 'plan', meaning: 'Lecture seule, dans un bac à sable' },
    ],
    effort: false,
    images: false,
    pdf: false,
    fork: 'none',
    resumeOtherAccount: false,
    midTurn: 'refused',
    slashCommands: false,
    quota: 'none',
    quotaWindows: [],
    cost: 'none',
    terminalView: false,
    remoteControl: false,
    backgroundTasks: false,
    todoTools: false,
    defaultRefusal: false,
    sandbox: false,
    usagePage: null,
    accountsByKey: false,
    readOnly: false,
  },
  install: { windows: 'npm install -g @openai/codex', unix: 'npm install -g @openai/codex' },
  models: [
    { id: 'gpt-5.5', label: 'GPT-5.5', effort: false, auto: false },
    { id: 'gpt-5.5-mini', label: 'GPT-5.5 mini', effort: false, auto: false },
  ],
  efforts: [],
  writer: 'Nu mini',
};

/** Cursor CLI as the tests see it (ESC-43): neither quota nor cost, its usage on its dashboard, its accounts by API key. */
export const CURSOR_ENGINE: EngineDecl = {
  ...BARE_ENGINE,
  id: 'cursor',
  name: 'Cursor CLI',
  caps: { ...BARE_ENGINE.caps, midTurn: 'turnEnd', usagePage: 'https://cursor.com/dashboard?tab=usage', accountsByKey: true },
};

/** Mistral Vibe as the tests see it (ESC-267): no quota, its cost told, its usage on Mistral's console, its accounts by API key. */
export const VIBE_ENGINE: EngineDecl = {
  ...BARE_ENGINE,
  id: 'vibe',
  name: 'Mistral Vibe',
  caps: {
    ...BARE_ENGINE.caps,
    cost: 'exact',
    usagePage: 'https://admin.mistral.ai/organization/usage',
    accountsByKey: true,
  },
};

/** Antigravity CLI as it declares itself (ESC-271): its tokens, neither quota nor cost, its plans' page; another account by its Gemini API key. */
export const AGY_ENGINE: EngineDecl = {
  ...BARE_ENGINE,
  id: 'agy',
  name: 'Antigravity CLI',
  caps: {
    ...BARE_ENGINE.caps,
    usagePage: 'https://antigravity.google/docs/plans',
    accountsByKey: true,
  },
};

/** Gemini CLI as it declares itself (ESC-275): its tokens, neither quota nor cost, the usage page of its keys; an account by its Google sign-in or its key, as it was added. */
export const GEMINI_ENGINE: EngineDecl = {
  ...BARE_ENGINE,
  id: 'gemini',
  name: 'Gemini CLI',
  caps: {
    ...BARE_ENGINE.caps,
    usagePage: 'https://aistudio.google.com/usage',
    accountsByKey: false,
  },
};

/** Claude Code's two windows of a reading, the shorthand of the tests: `fiveHour` and `sevenDay`, as before 1.8. */
export interface ClaudeWindows {
  fiveHour?: RateWindow | null;
  sevenDay?: RateWindow | null;
}

/** Claude Code's windows as meters (`QuotaMeter`), those given. */
export function claudeMeters({ fiveHour, sevenDay }: ClaudeWindows): QuotaMeter[] {
  const meters: QuotaMeter[] = [];
  if (fiveHour) meters.push({ minutes: 300, ...fiveHour });
  if (sevenDay) meters.push({ minutes: 10080, ...sevenDay });
  return meters;
}

/** An account's reading as the backend sends it, Claude Code's by default; its windows by the tests' shorthand. */
export function usageOf(id: string, over: Partial<AccountUsage> & ClaudeWindows = {}): AccountUsage {
  const { fiveHour, sevenDay, ...rest } = over;
  return {
    id,
    engine: 'claude',
    meters: claudeMeters({ fiveHour, sevenDay }),
    connected: true,
    reason: null,
    todayCost: 0,
    updatedAt: 1,
    ...rest,
  };
}

/**
 * The quotas as the backend sends them, from the tests' shorthand of before 1.8: `fiveHour` and `sevenDay` are the windows
 * of the current account of Claude Code (`current`, Principal by default), which the status bar shows; a window left out
 * stays as that account had it.
 */
export function usageWith(
  over: ClaudeWindows & { todayCost?: number; updatedAt?: number; accounts?: AccountUsage[]; current?: string | Usage['current'] },
): Usage {
  const { fiveHour, sevenDay, todayCost = 0, accounts = [], current } = over;
  const id = typeof current === 'string' ? current : (current?.claude ?? 'principal');
  const list = accounts.map((a) => ({ ...a }));
  if ('fiveHour' in over || 'sevenDay' in over) {
    let entry = list.find((a) => a.id === id);
    if (!entry) list.push((entry = usageOf(id)));
    const had = (m: number) => entry!.meters.find((x) => x.minutes === m) ?? null;
    entry.meters = claudeMeters({
      fiveHour: 'fiveHour' in over ? fiveHour : had(300),
      sevenDay: 'sevenDay' in over ? sevenDay : had(10080),
    });
  }
  return { todayCost, accounts: list, current: { ...(typeof current === 'object' ? current : {}), claude: id } };
}
