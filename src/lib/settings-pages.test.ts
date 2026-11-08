import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import { board, project, resetApp } from '../test/ipc';
import { setLang } from './i18n';
import {
  BOARD_ELSEWHERE,
  legacyTab,
  pageInfo,
  rowsOf,
  SETTINGS_ELSEWHERE,
  SETTINGS_OF,
  SETTINGS_PAGES,
  SETTINGS_ROWS,
  settingsForm,
  type ProjectDraft,
  type SettingsPage,
  type SettingsTab,
} from './settings.svelte';
import { app } from './state.svelte';
import type { BoardSettings, Settings } from './types';

vi.mock('./terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

// Every key of the types, listed so that the compiler refuses a key added to a type and forgotten here: a key of
// `Settings`, of a project or of its board that no page sets would be a setting lost by the new window.
const SETTINGS_KEYS: Record<keyof Settings, true> = {
  claudePath: true,
  sound: true,
  osNotifications: true,
  notifyFor: true,
  idleStopMinutes: true,
  attachMaxMb: true,
  attachWarnMb: true,
  attachOrigin: true,
  attachBigText: true,
  attachKeepDays: true,
  midTurnDelaySecs: true,
  pwshPath: true,
  bashPath: true,
  wslDistro: true,
  proxyUrl: true,
  noProxy: true,
  proxyTerminals: true,
  insecureTls: true,
  autoResume: true,
  switchOnLimit: true,
  handoverOnLimit: true,
  handoverModels: true,
  favoriteModels: true,
  quotaPause: true,
  switchAt: true,
  autoUpdate: true,
  betaUpdates: true,
  telemetry: true,
  integrations: true,
  language: true,
  claudeLanguage: true,
  writerEngine: true,
  writerModel: true,
  accounts: true,
  engineOrder: true,
  mcpEnabled: true,
  mcpPort: true,
  todoTools: true,
  claudeSelfUpdate: true,
  terminalView: true,
  remoteControlAll: true,
  privateProjects: true,
  theme: true,
  pluginTheme: true,
  pluginsVerifiedOnly: true,
  uiZoom: true,
  terminalFontSize: true,
  quotaProjection: true,
  editorFontSize: true,
  railWidth: true,
  sideWidth: true,
  userName: true,
  userId: true,
  nameAsked: true,
  userNameCleared: true,
  voice: true,
};

const PROJECT_KEYS: Record<Exclude<keyof ProjectDraft, 'board'>, true> = {
  docsBranch: true,
  name: true,
  color: true,
  tileLetters: true,
  logo: true,
  tileLabel: true,
  tileLabelText: true,
  worktreePerAgent: true,
  worktreeAtFirstMessage: true,
  worktreeBase: true,
  runCommands: true,
  copy: true,
  worktreeSetup: true,
  worktreeTeardown: true,
  commitMode: true,
  pagesOpen: true,
  account: true,
  worktreeDir: true,
  agentBranchPrefix: true,
  ticketBranchPrefix: true,
  remoteControl: true,
  engine: true,
  private: true,
  sharedOnLimit: true,
  workflowFixes: true,
  workflowsAutoRead: true,
  workflowsInRepo: true,
  workflowsEnabled: true,
  userName: true,
  shareNew: true,
  integrations: true,
};

const BOARD_KEYS: Record<keyof BoardSettings, true> = {
  action: true,
  target: true,
  strategy: true,
  draft: true,
  reviewers: true,
  labels: true,
  replyThreads: true,
  prUpdate: true,
  testsFirst: true,
  testCommand: true,
  cleanup: true,
  conventional: true,
  conflict: true,
  maxParallel: true,
  model: true,
  effort: true,
  mode: true,
  autopilot: true,
  pauseOnRedTarget: true,
  prefix: true,
  nextNumber: true,
};

/** The pages whose rows set `key`, of the app's settings, of a project or of its board. */
const pagesSetting = (where: 'settings' | 'project' | 'board', key: string) => [
  ...new Set(SETTINGS_ROWS.filter((r) => (r[where] as string[]).includes(key)).map((r) => r.page)),
];

const LEGACY: SettingsTab[] = [
  'app',
  'claude',
  'accounts',
  'notifications',
  'voice',
  'projects',
  'board',
  'integrations',
  'plugins',
  'mcp',
  'terminals',
  'network',
  'about',
];

describe('the register of the settings pages', () => {
  it('declares each page once, with its scope, its icon, its label and its description', () => {
    const ids = SETTINGS_PAGES.map((p) => p.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(SETTINGS_PAGES.filter((p) => p.scope === 'app').map((p) => p.id)).toEqual([
      'general',
      'agents',
      'accounts',
      'services',
      'voice',
      'notifications',
      'plugins',
      'advanced',
      'about',
      'allProjects',
    ]);
    expect(SETTINGS_PAGES.filter((p) => p.scope === 'project').map((p) => p.id)).toEqual([
      'project',
      'git',
      'launch',
      'board',
      'tickets',
      'team',
      'privacy',
    ]);
    for (const p of SETTINGS_PAGES) {
      expect(p.icon).toBeTruthy();
      expect(p.label).not.toContain('settings.pages');
      expect(p.desc).not.toContain('settings.pages');
    }
  });

  it('names its pages in the language of the interface, read when asked', () => {
    expect(pageInfo('accounts').label).toBe('Comptes et harness');
    expect(pageInfo('tickets').label).toBe('Tickets importés');
    setLang('en');
    flushSync();
    expect(SETTINGS_PAGES.map((p) => p.label)).toEqual([
      'General',
      'Agents',
      'Accounts and harnesses',
      'Connected services',
      'Voice',
      'Notifications',
      'Plugins',
      'Advanced',
      'About',
      'Project',
      'Git and worktrees',
      'Launch',
      'Kanban',
      'Imported tickets',
      'Team',
      'Privacy',
      'All projects',
    ]);
    setLang('fr');
    flushSync();
  });

  it('declares each row once, on a page of the register, with a label of the catalogs', () => {
    const ids = SETTINGS_ROWS.map((r) => r.id);
    expect(new Set(ids).size).toBe(ids.length);
    const pages = new Set<string>(SETTINGS_PAGES.map((p) => p.id));
    for (const r of SETTINGS_ROWS) {
      expect(pages.has(r.page), r.id).toBe(true);
      // A key missing from the catalogs would show as itself.
      expect(r.label, r.id).not.toMatch(/^[a-z]+(\.[a-zA-Z]+)+$/);
    }
    // Each page has rows, and some fold their rare ones into « Avancé ».
    for (const p of SETTINGS_PAGES) expect(rowsOf(p.id).length, p.id).toBeGreaterThan(0);
    expect(SETTINGS_ROWS.filter((r) => r.advanced).map((r) => r.id)).toEqual(
      expect.arrayContaining(['editorFontSize', 'userId', 'idleStop', 'engineOrder', 'claudePath', 'quotaProjection', 'closeProject']),
    );
  });

  it('has each key of Settings set by one page and one alone, or declared set out of the window', () => {
    for (const key of Object.keys(SETTINGS_KEYS)) {
      const pages = pagesSetting('settings', key);
      const elsewhere = SETTINGS_ELSEWHERE.includes(key as keyof Settings);
      expect({ key, pages: elsewhere ? ['(hors fenêtre)', ...pages] : pages }).toEqual({
        key,
        pages: [expect.any(String)],
      });
    }
    // No row sets a key that is not one.
    for (const r of SETTINGS_ROWS) for (const k of r.settings) expect(SETTINGS_KEYS, r.id).toHaveProperty(k);
    // `SETTINGS_OF` is read from the rows.
    for (const key of Object.keys(SETTINGS_KEYS) as (keyof Settings)[]) {
      const owners = (Object.keys(SETTINGS_OF) as SettingsPage[]).filter((p) => SETTINGS_OF[p].includes(key));
      expect(owners.length, key).toBe(SETTINGS_ELSEWHERE.includes(key) ? 0 : 1);
    }
  });

  it('has each key of a project and of its board set by one page of a project and one alone', () => {
    const projectPages = new Set<string>(SETTINGS_PAGES.filter((p) => p.scope === 'project').map((p) => p.id));
    for (const key of Object.keys(PROJECT_KEYS)) {
      const pages = pagesSetting('project', key);
      expect({ key, pages }).toEqual({ key, pages: [expect.any(String)] });
      expect(projectPages.has(pages[0]), key).toBe(true);
    }
    for (const key of Object.keys(BOARD_KEYS)) {
      const pages = pagesSetting('board', key);
      if (BOARD_ELSEWHERE.includes(key as keyof BoardSettings)) expect(pages, key).toEqual([]);
      else expect({ key, pages }).toEqual({ key, pages: ['board'] });
    }
    for (const r of SETTINGS_ROWS) {
      for (const k of r.project) expect(PROJECT_KEYS, r.id).toHaveProperty(k);
      for (const k of r.board) expect(BOARD_KEYS, r.id).toHaveProperty(k);
      // A page of the app sets no key of a project, a page of a project no key of the app.
      if (pageInfo(r.page).scope === 'app') expect([...r.project, ...r.board], r.id).toEqual([]);
      else expect(r.settings, r.id).toEqual([]);
    }
  });

  it('leads each id of a 2.0 tab to a page, and the sections asked for to theirs', () => {
    const pages = new Set<string>(SETTINGS_PAGES.map((p) => p.id));
    for (const tab of LEGACY) expect(pages.has(legacyTab(tab)), tab).toBe(true);
    expect(Object.fromEntries(LEGACY.map((tab) => [tab, legacyTab(tab)]))).toEqual({
      app: 'general',
      claude: 'accounts',
      accounts: 'accounts',
      notifications: 'notifications',
      voice: 'voice',
      projects: 'project',
      board: 'board',
      integrations: 'services',
      plugins: 'plugins',
      mcp: 'agents',
      terminals: 'advanced',
      network: 'advanced',
      about: 'about',
    });
    expect(legacyTab('projects', 'launch')).toBe('launch');
    expect(legacyTab('integrations', 'sources')).toBe('tickets');
  });
});

describe('settingsForm by page', () => {
  beforeEach(() => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site', board: board({ action: 'pr' }) })] });
  });

  it('opens on a page, or on the page a 2.0 tab leads to', () => {
    settingsForm.open();
    expect(settingsForm.page).toBe('accounts');
    settingsForm.open({ page: 'tickets', projectId: 'p2' });
    expect(settingsForm.page).toBe('tickets');
    expect(settingsForm.projectId).toBe('p2');
    settingsForm.open({ tab: 'projects', section: 'launch', projectId: 'p1' });
    expect(settingsForm.page).toBe('launch');
    settingsForm.resume({ tab: 'mcp' });
    expect(settingsForm.page).toBe('agents');
  });

  it('marks as changed the page of each key changed, and no other', () => {
    const changed = () => SETTINGS_PAGES.map((p) => p.id).filter((p) => settingsForm.changed(p));
    settingsForm.open();
    expect(changed()).toEqual([]);
    settingsForm.settings.terminalView = true;
    expect(changed()).toEqual(['agents']);
    settingsForm.settings.quotaPause = 80;
    expect(changed()).toEqual(['agents', 'accounts']);
    settingsForm.settings.wslDistro = 'Debian';
    settingsForm.settings.privateProjects = true;
    expect(changed()).toEqual(['agents', 'accounts', 'advanced', 'allProjects']);

    settingsForm.open();
    settingsForm.settings.integrations.syncStates = !settingsForm.settings.integrations.syncStates;
    settingsForm.settings.voice = { ...settingsForm.settings.voice, ...({ speed: 1.5 } as object) };
    expect(changed()).toEqual(['services', 'voice']);

    settingsForm.open();
    settingsForm.projects.p2.tileLetters = 'ST';
    settingsForm.projects.p2.worktreeDir = '../wt';
    settingsForm.projects.p2.pagesOpen = 'window';
    settingsForm.projects.p2.board.maxParallel = 7;
    settingsForm.projects.p2.workflowsAutoRead = false;
    settingsForm.projects.p2.shareNew = true;
    settingsForm.projects.p2.sharedOnLimit = true;
    expect(changed()).toEqual(['project', 'git', 'launch', 'board', 'team', 'privacy']);
    // A project's page, for that project alone.
    expect(settingsForm.changed('git', 'p2')).toBe(true);
    expect(settingsForm.changed('git', 'p1')).toBe(false);
    settingsForm.projects.p1.integrations.comments = ['done'];
    expect(settingsForm.changed('tickets', 'p1')).toBe(true);
    expect(settingsForm.changed('tickets', 'p2')).toBe(false);
  });

  it('sees no change of a page from a setting saved at once, out of the draft', () => {
    settingsForm.open();
    app.settings.theme = 'dark';
    app.settings.switchAt = 70;
    expect(SETTINGS_PAGES.filter((p) => settingsForm.changed(p.id))).toEqual([]);
  });
});

describe('the names of the rows', () => {
  it('are whole words, never a text with a blank to fill (« Avancé » names them, the search finds them)', () => {
    for (const lang of ['fr', 'en'] as const) {
      setLang(lang);
      for (const r of SETTINGS_ROWS) expect(r.label, `${r.id} in ${lang}`).not.toMatch(/[{}]/);
    }
    setLang('fr');
  });
});
