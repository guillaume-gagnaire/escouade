import { beforeEach, describe, expect, it } from 'vitest';
import { board, project, resetApp } from '../test/ipc';
import { setLang } from './i18n';
import type { Account, Project, SetupInfo } from './types';
import {
  absoluteDir,
  applyCopy,
  blankConfig,
  configOf,
  controls,
  copyDirs,
  copyItems,
  recommendedConfig,
  stepsFor,
  validationText,
  withConfig,
} from './wizard';

const info = (over: Partial<SetupInfo> = {}): SetupInfo => ({
  hasCommit: true,
  branches: ['main', 'feature'],
  missingDirs: [],
  worktreesTracked: false,
  ...over,
});

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const WORK: Account = { id: 'work', name: 'Travail', configDir: 'C:\\w', claudePath: '', active: true };

/** A project set up as one that works: what the assistant copies. */
const source = (over: Partial<Project> = {}): Project =>
  project({
    id: 'src',
    name: 'web-shop',
    path: 'C:\\code\\web-shop',
    worktreePerAgent: true,
    worktreeDir: '../wt',
    agentBranchPrefix: 'agent/',
    ticketBranchPrefix: 'tk/',
    worktreeCopy: ['.env*', 'config/local.json'],
    worktreeSetup: [
      { id: 's1', command: 'npm ci', shell: 'pwsh', cwd: '' },
      { id: 's2', command: 'npm ci', shell: 'pwsh', cwd: 'web' },
    ],
    worktreeTeardown: [{ id: 'd1', command: 'docker compose down', shell: 'pwsh', cwd: '' }],
    runCommands: [
      { id: 'r1', name: 'api', command: 'cargo run', shell: 'pwsh', cwd: 'api' },
      { id: 'r2', name: 'web', command: 'npm run dev', shell: 'pwsh', cwd: 'web/' },
    ],
    board: board({
      action: 'merge',
      target: 'develop',
      strategy: 'rebase',
      testsFirst: true,
      testCommand: 'npm test',
      cleanup: false,
      conventional: false,
      conflict: 'agent',
      maxParallel: 4,
      autopilot: false,
      model: 'opus',
      effort: 'high',
      mode: 'auto',
      prefix: 'WEB',
      nextNumber: 42,
    }),
    commitMode: 'direct',
    account: 'work',
    remoteControl: true,
    integrations: { links: [{ service: 'jira', container: 'WEB', name: 'Web', states: {} }], comments: ['done'] },
    ...over,
  });

describe('the assistant’s steps and starting points', () => {
  beforeEach(() => setLang('fr'));

  it('goes through the copied settings only when copying', () => {
    expect(stepsFor('copy')).toEqual(['start', 'copied', 'worktrees', 'launch', 'kanban', 'account', 'integrations', 'summary']);
    for (const s of ['recommended', 'blank', 'current'] as const) expect(stepsFor(s)).not.toContain('copied');
  });

  it('starts from scratch on Escouade’s defaults, those of a project made without it', () => {
    const c = blankConfig();
    expect(c.worktreePerAgent).toBe(false);
    expect(c.copy).toBe('.env*');
    expect(c.board).toMatchObject({ action: 'merge', target: '', strategy: 'squash', cleanup: true, conventional: true, conflict: 'ask' });
    expect(c.commitMode).toBe('agent');
  });

  it('makes the worktree at the first message by default, keeps a project’s choice, and copies it with a worktree per agent (ESC-156)', () => {
    expect(blankConfig().worktreeAtFirstMessage).toBe(true);
    expect(recommendedConfig().worktreeAtFirstMessage).toBe(true);
    const off = project({ worktreeAtFirstMessage: false });
    expect(configOf(off).worktreeAtFirstMessage).toBe(false);
    // A project of before ESC-156 says nothing: on, as the backend reads it.
    expect(configOf(project()).worktreeAtFirstMessage).toBe(true);
    expect(withConfig(project(), { ...blankConfig(), worktreeAtFirstMessage: false }).worktreeAtFirstMessage).toBe(false);
    expect(applyCopy(blankConfig(), off, new Set(['worktreePerAgent']), null).worktreeAtFirstMessage).toBe(false);
    expect(applyCopy(blankConfig(), off, new Set(), null).worktreeAtFirstMessage).toBe(true);
  });

  it('starts the worktrees from the folder’s branch by default, keeps a project’s start, and copies it with them (ESC-160)', () => {
    expect(blankConfig().worktreeBase).toEqual({ kind: 'head' });
    expect(recommendedConfig().worktreeBase).toEqual({ kind: 'head' });
    expect(blankConfig().board).not.toHaveProperty('agentsFromTarget');
    const named = project({ worktreeBase: { kind: 'branch', branch: 'release' } });
    expect(configOf(named).worktreeBase).toEqual({ kind: 'branch', branch: 'release' });
    // A project of before ESC-160 says nothing: the folder's branch.
    expect(configOf(project()).worktreeBase).toEqual({ kind: 'head' });
    expect(withConfig(project(), { ...blankConfig(), worktreeBase: { kind: 'target' } }).worktreeBase).toEqual({ kind: 'target' });
    // « Une branche… » with none chosen yet saves the folder's.
    expect(withConfig(project(), { ...blankConfig(), worktreeBase: { kind: 'branch', branch: ' ' } }).worktreeBase).toEqual({
      kind: 'head',
    });
    expect(applyCopy(blankConfig(), named, new Set(['worktreePerAgent']), null).worktreeBase).toEqual({
      kind: 'branch',
      branch: 'release',
    });
    expect(applyCopy(blankConfig(), named, new Set(), null).worktreeBase).toEqual({ kind: 'head' });
  });

  it('recommends a worktree per agent, a squash merge into the current branch, cleanup, conventional commits, the agent on conflicts', () => {
    const c = recommendedConfig();
    expect(c.worktreePerAgent).toBe(true);
    expect(c.board).toMatchObject({
      action: 'merge',
      target: '',
      strategy: 'squash',
      cleanup: true,
      conventional: true,
      conflict: 'agent',
    });
    expect(c.commitMode).toBe('agent');
  });

  it('keeps an existing project’s settings, and its key of tickets, when saved again', () => {
    const p = source();
    const c = configOf(p);
    expect(c.board).not.toHaveProperty('prefix');
    const saved = withConfig(p, c);
    expect(saved.board.prefix).toBe('WEB');
    expect(saved.board.nextNumber).toBe(42);
    expect(saved.runCommands).toEqual(p.runCommands.map((r) => ({ ...r, cwd: r.cwd.trim() })));
    expect(saved.integrations).toBe(p.integrations);
  });

  it('keeps a project’s harness when saved again, and a new one has none of its own (Claude Code)', () => {
    expect(blankConfig().engine).toBeNull();
    expect(recommendedConfig().engine).toBeNull();
    expect(withConfig(source(), configOf(source())).engine).toBeNull();
    const codex = source({ engine: 'codex' });
    expect(withConfig(codex, configOf(codex)).engine).toBe('codex');
  });

  it('saves trimmed settings, without the steps and commands that run nothing, and no tests first without a command', () => {
    const c = blankConfig();
    c.copy = ' .env* \n\n dist/ ';
    c.worktreeSetup = [
      { id: 'a', command: ' npm ci ', shell: 'pwsh', cwd: ' web ' },
      { id: 'b', command: '  ', shell: 'pwsh', cwd: '' },
    ];
    c.runCommands = [{ id: 'r', name: ' ', command: '', shell: 'pwsh', cwd: '' }];
    c.board.testsFirst = true;
    c.board.testCommand = '  ';
    const p = withConfig(project(), c);
    expect(p.worktreeCopy).toEqual(['.env*', 'dist/']);
    expect(p.worktreeSetup).toEqual([{ id: 'a', command: 'npm ci', shell: 'pwsh', cwd: 'web' }]);
    expect(p.runCommands).toEqual([]);
    expect(p.board).toMatchObject({ testsFirst: false, testCommand: '' });
  });
});

describe('copying another project’s settings', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
  });

  it('asks for the folders of its commands and steps, each once', () => {
    expect(copyDirs(source())).toEqual(['api', 'web']);
  });

  it('tells a whole path, or one from the home folder, from one relative to the project', () => {
    expect(['C:\\wt', '/srv/wt', '~/wt', '"D:/x"', '\\\\srv\\share'].map(absoluteDir)).toEqual([true, true, true, true, true]);
    expect(['../wt', '.claude/worktrees', '', 'wt'].map(absoluteDir)).toEqual([false, false, false, false]);
  });

  it('lists its settings by group, neither its integrations nor its tickets among them', () => {
    const items = copyItems(source(), info(), [PRINCIPAL, WORK]);
    const groups = [...new Set(items.map((i) => i.group))];
    expect(groups).toEqual(['worktrees', 'launch', 'kanban', 'account']);
    expect(items.map((i) => i.id)).toEqual([
      'worktreePerAgent',
      'worktreeDir',
      'prefixes',
      'copy',
      'setup:s1',
      'setup:s2',
      'teardown:d1',
      'run:r1',
      'run:r2',
      'validation',
      'tests',
      'cleanup',
      'conventional',
      'conflict',
      'parallel',
      'agents',
      'commitMode',
      'account',
      'remoteControl',
    ]);
    expect(items.some((i) => /jira|integration|ticket/i.test(i.id))).toBe(false);
  });

  it('checks every setting by default when all of it works in the new folder', () => {
    const items = copyItems(source({ board: board({ target: 'main' }) }), info(), [PRINCIPAL, WORK]);
    expect(items.every((i) => i.checked)).toBe(true);
    const byId = Object.fromEntries(items.map((i) => [i.id, i]));
    expect(byId.worktreeDir).toMatchObject({ status: 'ok', note: 'relatif au projet' });
    expect(byId['run:r1']).toMatchObject({ status: 'ok', note: 'dossier « api » présent' });
    expect(byId.validation).toMatchObject({ status: 'ok', value: 'merge squash → main', note: 'branche présente' });
    expect(byId.account).toMatchObject({ status: 'ok', value: 'Travail' });
  });

  it('leaves out a command whose folder is not in the new folder, and says so', () => {
    const items = copyItems(source(), info({ missingDirs: ['web'] }), [PRINCIPAL, WORK]);
    const byId = Object.fromEntries(items.map((i) => [i.id, i]));
    expect(byId['run:r2']).toMatchObject({ checked: false, status: 'warn', note: 'dossier « web » introuvable ici' });
    expect(byId['setup:s2']).toMatchObject({ checked: false, status: 'warn' });
    expect(byId['run:r1'].checked).toBe(true);
    // A step in the worktree's own folder has no folder to look for.
    expect(byId['setup:s1']).toMatchObject({ checked: true, status: 'plain', note: '' });
  });

  it('says a target branch the new folder does not have, and takes its current branch instead', () => {
    const items = copyItems(source(), info({ branches: ['trunk', 'x'] }), [PRINCIPAL]);
    const v = items.find((i) => i.id === 'validation')!;
    expect(v).toMatchObject({ checked: true, status: 'bad', note: 'branche introuvable : trunk à la place' });
    const c = applyCopy(blankConfig(), source(), new Set(items.filter((i) => i.checked).map((i) => i.id)), info({ branches: ['trunk'] }));
    expect(c.board.target).toBe('');
    expect(c.board.strategy).toBe('rebase');
  });

  it('keeps a target branch the new folder has', () => {
    const c = applyCopy(blankConfig(), source(), new Set(['validation']), info({ branches: ['main', 'develop'] }));
    expect(c.board.target).toBe('develop');
  });

  it('does not take a worktrees folder written whole: it is the other project’s', () => {
    const items = copyItems(source({ worktreeDir: 'C:\\code\\web-shop-wt' }), info(), [PRINCIPAL]);
    expect(items.find((i) => i.id === 'worktreeDir')).toMatchObject({
      checked: false,
      status: 'warn',
      note: 'chemin complet d’un autre projet : non repris',
    });
  });

  it('does not take an account switched off, reserved to other projects or gone, and says it falls back to Automatique', () => {
    for (const accounts of [[PRINCIPAL], [PRINCIPAL, { ...WORK, active: false }], [PRINCIPAL, { ...WORK, projects: ['other'] }]]) {
      const a = copyItems(source(), info(), accounts).find((i) => i.id === 'account')!;
      expect(a).toMatchObject({ checked: false, status: 'warn', note: 'compte inactif, réservé ou supprimé : Automatique' });
    }
    const auto = copyItems(source({ account: '' }), info(), [PRINCIPAL]).find((i) => i.id === 'account')!;
    expect(auto).toMatchObject({ checked: true, status: 'plain', value: 'Automatique' });
  });

  it('takes what is checked over the defaults, and only that', () => {
    const src = source({ board: board({ ...source().board, target: 'main' }) });
    const all = new Set(copyItems(src, info(), [PRINCIPAL, WORK]).map((i) => i.id));
    const c = applyCopy(blankConfig(), src, all, info());
    expect(c).toMatchObject({
      worktreePerAgent: true,
      worktreeDir: '../wt',
      agentBranchPrefix: 'agent/',
      ticketBranchPrefix: 'tk/',
      copy: '.env*\nconfig/local.json',
      commitMode: 'direct',
      account: 'work',
      remoteControl: true,
    });
    expect(c.board).toMatchObject({
      target: 'main',
      strategy: 'rebase',
      testsFirst: true,
      testCommand: 'npm test',
      cleanup: false,
      conventional: false,
      conflict: 'agent',
      maxParallel: 4,
      autopilot: false,
      model: 'opus',
      effort: 'high',
      mode: 'auto',
    });
    expect(c.board).not.toHaveProperty('prefix');
    expect(c.runCommands.map((r) => r.name)).toEqual(['api', 'web']);
    expect(c.worktreeSetup.map((s) => s.cwd)).toEqual(['', 'web']);
    expect(c.worktreeTeardown.map((s) => s.command)).toEqual(['docker compose down']);

    const none = applyCopy(blankConfig(), src, new Set(['run:r1', 'conflict']), info());
    expect(none).toMatchObject({
      ...blankConfig(),
      runCommands: [expect.objectContaining({ name: 'api' })],
      board: { ...blankConfig().board, conflict: 'agent' },
    });
  });

  it('copies the harness with the agents’ settings, and lists nothing more for it', () => {
    const src = source({ engine: 'copilot' });
    const items = copyItems(src, info(), [PRINCIPAL, WORK]);
    expect(items.map((i) => i.id)).toEqual(copyItems(source(), info(), [PRINCIPAL, WORK]).map((i) => i.id));
    expect(applyCopy(blankConfig(), src, new Set(['agents']), info()).engine).toBe('copilot');
    expect(applyCopy(blankConfig(), src, new Set(['conflict']), info()).engine).toBeNull();
  });

  it('gives the commands and steps copied ids of their own: the other project’s launches stay its own', () => {
    const c = applyCopy(blankConfig(), source(), new Set(['run:r1', 'setup:s1']), info());
    expect(c.runCommands[0].id).not.toBe('r1');
    expect(c.worktreeSetup[0].id).not.toBe('s1');
  });

  it('writes what validating does as the copy lists it', () => {
    expect(validationText({ action: 'merge', strategy: 'squash', target: '', draft: false })).toBe('merge squash → branche courante');
    expect(validationText({ action: 'pr', strategy: 'squash', target: 'main', draft: true })).toBe('PR brouillon → main');
    expect(validationText({ action: 'keep', strategy: 'squash', target: 'main', draft: false })).toBe('branche gardée');
    setLang('en');
    expect(validationText({ action: 'push', strategy: 'squash', target: '', draft: false })).toBe('branch pushed');
  });
});

describe('the checks of the way of working', () => {
  const ctx = { accounts: [PRINCIPAL], claudeFound: true };

  it('finds nothing wrong in a ready repository', () => {
    expect(controls(info(), recommendedConfig(), ctx)).toEqual([]);
  });

  it('says a repository without commit, or no repository at all: no ticket starts', () => {
    expect(controls(info({ hasCommit: false, branches: [] }), recommendedConfig(), ctx)).toEqual(['noCommit']);
    expect(controls(null, recommendedConfig(), ctx)).toEqual(['noCommit']);
  });

  it('says a target branch not found, for a merge or a pull request only', () => {
    const c = recommendedConfig();
    c.board.target = 'develop';
    expect(controls(info(), c, ctx)).toEqual(['noTarget']);
    c.board.action = 'pr';
    expect(controls(info(), c, ctx)).toEqual(['noTarget']);
    c.board.action = 'keep';
    expect(controls(info(), c, ctx)).toEqual([]);
  });

  it('says worktrees in a folder git tracks', () => {
    expect(controls(info({ worktreesTracked: true }), recommendedConfig(), ctx)).toEqual(['tracked']);
  });

  it('says when no account may serve the project: all off, or reserved to other projects', () => {
    expect(controls(info(), recommendedConfig(), { ...ctx, accounts: [{ ...PRINCIPAL, active: false }] })).toEqual(['noAccount']);
    const reserved = [{ ...WORK, projects: ['other'] }];
    expect(controls(info(), recommendedConfig(), { ...ctx, accounts: reserved })).toEqual(['noAccount']);
    // An existing project the account is reserved to is served.
    expect(controls(info(), recommendedConfig(), { ...ctx, accounts: [{ ...WORK, projects: ['p9'] }], projectId: 'p9' })).toEqual([]);
  });

  it('says Claude Code not found', () => {
    expect(controls(info(), recommendedConfig(), { ...ctx, claudeFound: false })).toEqual(['noClaude']);
  });
});

describe('every setting a project has at its creation (ESC-163)', () => {
  beforeEach(() => setLang('fr'));

  it('starts a project not private, but when « Nouveaux projets privés » says so, without accounts reserved', () => {
    for (const make of [blankConfig, recommendedConfig]) {
      expect(make()).toMatchObject({ private: false, sharedOnLimit: false, reserved: [] });
      expect(make({ private: true })).toMatchObject({ private: true, sharedOnLimit: false, reserved: [] });
    }
  });

  it('keeps a private project’s settings and the accounts reserved to it alone, as the assistant edits them again', () => {
    const p = source({ private: true, sharedOnLimit: true });
    const accounts: Account[] = [
      PRINCIPAL,
      { ...WORK, projects: ['src'] },
      { ...WORK, id: 'both', projects: ['src', 'other'] },
      { ...WORK, id: 'theirs', projects: ['other'] },
    ];
    const c = configOf(p, accounts);
    expect(c).toMatchObject({ private: true, sharedOnLimit: true, reserved: ['work'] });
    // Without the accounts, none: the assistant of a new project has none to show.
    expect(configOf(p).reserved).toEqual([]);
    // A project of before ESC-140 says nothing: not private.
    expect(configOf(project())).toMatchObject({ private: false, sharedOnLimit: false });
  });

  it('saves « Projet privé », and « Comptes partagés à la limite » only for a private project', () => {
    const c = { ...blankConfig(), private: true, sharedOnLimit: true, reserved: ['work'] };
    expect(withConfig(project(), c)).toMatchObject({ private: true, sharedOnLimit: true });
    expect(withConfig(project(), { ...c, private: false })).toMatchObject({ private: false, sharedOnLimit: false });
    // The accounts are no setting of the project: the assistant reserves them apart.
    expect(withConfig(project(), c)).not.toHaveProperty('reserved');
  });

  it('keeps the whole Kanban of its tickets’ agents: in parallel, model, effort, mode, autopilot', () => {
    const c = recommendedConfig();
    expect(c.board).toMatchObject({ maxParallel: 2, model: '', effort: '', mode: '', autopilot: true });
    c.board = { ...c.board, maxParallel: 5, model: 'opus', effort: 'xhigh', mode: 'plan', autopilot: false };
    const saved = withConfig(project(), c);
    expect(saved.board).toMatchObject({ maxParallel: 5, model: 'opus', effort: 'xhigh', mode: 'plan', autopilot: false });
    expect(configOf(saved).board).toMatchObject({ maxParallel: 5, model: 'opus', effort: 'xhigh', mode: 'plan', autopilot: false });
  });

  it('copies neither « Projet privé » nor its accounts: the projects copied are never private, the new one keeps its own', () => {
    // As if the other project were private (the assistant never offers one): nothing of it comes.
    const src = source({ private: true, sharedOnLimit: true });
    const all = new Set(copyItems(src, info(), [PRINCIPAL, WORK]).map((i) => i.id));
    expect([...all].some((id) => /private|shared|reserved/i.test(id))).toBe(false);
    expect(applyCopy(blankConfig(), src, all, info())).toMatchObject({ private: false, sharedOnLimit: false, reserved: [] });
    // A new project private by default stays so.
    expect(applyCopy(blankConfig({ private: true }), src, all, info())).toMatchObject({ private: true, reserved: [] });
  });

  it('lists the harness with the agents’ settings it goes with', () => {
    const agents = (p: Project) => copyItems(p, info(), [PRINCIPAL]).find((i) => i.id === 'agents')!;
    expect(agents(source({ engine: 'codex' })).value).toBe('Codex · opus · high · auto');
    expect(agents(source()).value).toBe('opus · high · auto');
    expect(agents(source({ board: board({}) })).value).toBe('Par défaut');
    expect(copyItems(source(), info(), [PRINCIPAL]).find((i) => i.id === 'agents')!.label).toBe('Harness · modèle · effort · mode');
  });
});
