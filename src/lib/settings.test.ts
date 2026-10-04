import { beforeEach, describe, expect, it, vi } from 'vitest';
import { board, fakeBackend, project, resetApp, SETTINGS } from '../test/ipc';
import { settingsForm } from './settings.svelte';
import { app } from './state.svelte';
import type { RunCommand } from './types';

vi.mock('./terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const FRONT: RunCommand = { id: 'c1', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: 'web' };
const P2 = () => project({ id: 'p2', name: 'site', color: 'oklch(0.7 0.1 200)', board: board({ action: 'pr' }) });

describe('settingsForm', () => {
  beforeEach(() => {
    resetApp({ projects: [project({ runCommands: [FRONT] }), P2()] });
  });

  it('opens on Claude Code and the project on screen, or on the tab and project asked for', () => {
    app.ui.activeProject = 'p2';
    settingsForm.open();
    expect(settingsForm.tab).toBe('claude');
    expect(settingsForm.projectId).toBe('p2');
    settingsForm.open({ tab: 'board', projectId: 'p1' });
    expect(settingsForm.tab).toBe('board');
    expect(settingsForm.projectId).toBe('p1');
    expect(settingsForm.project?.board.action).toBe('merge');
    // A project closed since the shortcut was made: the one on screen instead.
    settingsForm.open({ tab: 'projects', projectId: 'gone' });
    expect(settingsForm.projectId).toBe('p2');
  });

  it('has no project to set when there is none', () => {
    resetApp({ projects: [] });
    settingsForm.open({ tab: 'projects' });
    expect(settingsForm.projectId).toBeNull();
    expect(settingsForm.project).toBeUndefined();
  });

  it('saves nothing when nothing changed', async () => {
    const backend = fakeBackend();
    settingsForm.open();
    expect(await settingsForm.save()).toBe(true);
    expect(backend.calls).toEqual([]);
  });

  it('saves the app settings changed, and takes the shells they find', async () => {
    const backend = fakeBackend({ save_settings: () => [{ id: 'pwsh', label: 'PowerShell', path: 'pwsh.exe' }] });
    settingsForm.open();
    settingsForm.settings.proxyUrl = 'http://proxy:3128';
    settingsForm.settings.sound = false;
    // Set elsewhere meanwhile: kept, the modal did not change it.
    app.settings.osNotifications = false;
    expect(await settingsForm.save()).toBe(true);
    expect(backend.called('save_settings')[0].args.settings).toEqual({
      ...SETTINGS,
      proxyUrl: 'http://proxy:3128',
      sound: false,
      osNotifications: false,
    });
    expect(app.settings).toMatchObject({ proxyUrl: 'http://proxy:3128', sound: false });
    expect(app.shells.map((s) => s.id)).toEqual(['pwsh']);
    expect(backend.called('update_project')).toHaveLength(0);
    expect(backend.called('board_set')).toHaveLength(0);
  });

  it('turns an emptied number of minutes into 0 instead of sending null', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    settingsForm.open();
    (settingsForm.settings as any).idleStopMinutes = null;
    await settingsForm.save();
    expect(backend.called('save_settings')[0].args.settings.idleStopMinutes).toBe(0);
  });

  it('saves what changed of each project, its board apart, over what the app has now', async () => {
    const backend = fakeBackend({ board_set: (a: any) => ({ ...app.projects.find((p) => p.id === a.projectId), board: a.settings }) });
    settingsForm.open({ projectId: 'p1' });
    settingsForm.projects.p1.name = '  api  ';
    settingsForm.projects.p1.worktreePerAgent = true;
    settingsForm.projects.p2.color = 'oklch(0.8 0.1 100)';
    settingsForm.projects.p2.board.strategy = 'rebase';
    // Meanwhile the backend moved the board on: its first ticket set the target, a key was used.
    app.projects[1].board = { ...app.projects[1].board, target: 'develop', nextNumber: 4 };
    expect(await settingsForm.save()).toBe(true);

    const updates = backend.called('update_project').map((c) => c.args.project);
    expect(updates).toHaveLength(2);
    expect(updates[0]).toMatchObject({ id: 'p1', name: 'api', worktreePerAgent: true, color: 'oklch(0.72 0.12 48)' });
    expect(updates[1]).toMatchObject({ id: 'p2', name: 'site', color: 'oklch(0.8 0.1 100)' });
    expect(app.projects[0].name).toBe('api');
    expect(app.projects[1].color).toBe('oklch(0.8 0.1 100)');
    const boards = backend.called('board_set');
    expect(boards).toHaveLength(1);
    expect(boards[0].args).toMatchObject({
      projectId: 'p2',
      settings: { action: 'pr', strategy: 'rebase', target: 'develop', nextNumber: 4 },
    });
    expect(app.projects[1].board).toMatchObject({ strategy: 'rebase', target: 'develop' });
  });

  it('saves the launch commands trimmed and the copied files one per line, and stops a removed one that ran', async () => {
    const backend = fakeBackend();
    app.launches.c1 = { status: 'running', ptyId: 't1', name: 'Front', stopping: false, code: null, startedAt: 1 };
    settingsForm.open({ tab: 'projects', projectId: 'p1' });
    settingsForm.addCommand();
    const added = settingsForm.projects.p1.runCommands[1];
    Object.assign(added, { name: ' API ', command: ' cargo run ', cwd: ' back ' });
    settingsForm.projects.p1.runCommands.splice(0, 1);
    settingsForm.projects.p1.copy = '.env*\n\n  **/.env.local  \n';
    expect(await settingsForm.save()).toBe(true);

    const saved = backend.called('update_project')[0].args.project;
    expect(saved.runCommands).toEqual([{ id: added.id, name: 'API', command: 'cargo run', shell: 'pwsh', cwd: 'back' }]);
    expect(saved.worktreeCopy).toEqual(['.env*', '**/.env.local']);
    expect(app.projects[0].runCommands).toEqual(saved.runCommands);
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
    expect(app.launches.c1).toBeUndefined();
  });

  it('gives a new command the first shell found', () => {
    app.shells = [{ id: 'bash', label: 'Git Bash', path: 'bash.exe' }];
    settingsForm.open({ projectId: 'p2' });
    settingsForm.addCommand();
    expect(settingsForm.projects.p2.runCommands[0]).toMatchObject({ name: '', command: '', shell: 'bash', cwd: '' });
    expect(settingsForm.projects.p1.runCommands).toHaveLength(1);
  });

  it('keeps the tests off when their command is emptied', async () => {
    const backend = fakeBackend({ board_set: (a: any) => project({ board: a.settings }) });
    app.projects[0].board = board({ testCommand: 'npm test', testsFirst: true });
    settingsForm.open({ projectId: 'p1' });
    settingsForm.projects.p1.board.testCommand = '   ';
    await settingsForm.save();
    expect(backend.called('board_set')[0].args.settings).toMatchObject({ testCommand: '', testsFirst: false });
  });

  it('refuses to save a project without a name or a command without a name or a command line', async () => {
    const backend = fakeBackend();
    settingsForm.open({ projectId: 'p1' });
    expect(settingsForm.problem).toBeNull();
    settingsForm.projects.p2.name = ' ';
    expect(settingsForm.problem).toBe('Le projet « site » doit garder un nom.');
    expect(await settingsForm.save()).toBe(false);
    settingsForm.projects.p2.name = 'site';
    settingsForm.projects.p1.runCommands[0].command = '';
    expect(settingsForm.problem).toBe('Chaque commande de lancement de « demo-api » demande un nom et une ligne de commande.');
    expect(await settingsForm.save()).toBe(false);
    expect(backend.calls).toEqual([]);
  });

  it('marks the tabs whose settings changed', () => {
    settingsForm.open();
    expect(settingsForm.changed('claude')).toBe(false);
    settingsForm.settings.defaultModel = 'opus';
    settingsForm.settings.proxyTerminals = true;
    settingsForm.projects.p2.board.maxParallel = 5;
    expect(settingsForm.changed('claude')).toBe(true);
    expect(settingsForm.changed('network')).toBe(true);
    expect(settingsForm.changed('board')).toBe(true);
    expect(settingsForm.changed('notifications')).toBe(false);
    expect(settingsForm.changed('terminals')).toBe(false);
    expect(settingsForm.changed('projects')).toBe(false);
    settingsForm.projects.p1.copy = '.env*\n';
    // The same patterns: nothing to save.
    expect(settingsForm.changed('projects')).toBe(false);
    settingsForm.projects.p1.copy = '.env.local';
    expect(settingsForm.changed('projects')).toBe(true);
  });

  it('keeps what could not be saved to try again, without saving twice what was', async () => {
    let fail = true;
    const backend = fakeBackend({
      save_settings: () => [],
      board_set: (a: any) => {
        if (fail) throw 'disque plein';
        return project({ board: a.settings });
      },
    });
    settingsForm.open({ projectId: 'p1' });
    settingsForm.settings.sound = false;
    settingsForm.projects.p1.name = 'api';
    settingsForm.projects.p1.board.strategy = 'rebase';
    expect(await settingsForm.save()).toBe(false);
    expect(app.toasts.at(-1)?.text).toContain('disque plein');
    expect(app.projects[0].board.strategy).toBe('squash');
    expect(settingsForm.changed('notifications')).toBe(false);
    expect(settingsForm.changed('projects')).toBe(false);
    expect(settingsForm.changed('board')).toBe(true);

    fail = false;
    expect(await settingsForm.save()).toBe(true);
    expect(backend.called('save_settings')).toHaveLength(1);
    expect(backend.called('board_set')).toHaveLength(2);
    expect(app.projects[0].board.strategy).toBe('rebase');
    // All saved: nothing is sent again.
    expect(await settingsForm.save()).toBe(true);
    expect(backend.called('save_settings')).toHaveLength(1);
    expect(backend.called('update_project')).toHaveLength(1);
    expect(backend.called('board_set')).toHaveLength(2);
  });

  it('keeps as a change what was typed while it was being saved', async () => {
    fakeBackend({
      update_project: async () => {
        settingsForm.projects.p1.name = 'api-v2';
      },
      board_set: () => {
        throw 'disque plein';
      },
    });
    settingsForm.open({ projectId: 'p1' });
    settingsForm.projects.p1.name = 'api';
    settingsForm.projects.p1.board.strategy = 'rebase';
    expect(await settingsForm.save()).toBe(false);
    expect(app.projects[0].name).toBe('api');
    expect(settingsForm.changed('projects')).toBe(true);
  });

  it('resumes the draft it had, on the tab and project asked for', () => {
    settingsForm.open({ projectId: 'p1' });
    settingsForm.settings.proxyUrl = 'http://proxy:3128';
    settingsForm.resume({ tab: 'projects', projectId: 'p2' });
    expect(settingsForm.tab).toBe('projects');
    expect(settingsForm.projectId).toBe('p2');
    expect(settingsForm.settings.proxyUrl).toBe('http://proxy:3128');
    // Its project closed meanwhile: the one on screen.
    app.projects = app.projects.filter((p) => p.id !== 'p2');
    settingsForm.resume({ tab: 'projects', projectId: 'p2' });
    expect(settingsForm.projectId).toBe('p1');
  });

  it('skips a project closed while its settings were open', async () => {
    const backend = fakeBackend();
    settingsForm.open({ projectId: 'p2' });
    settingsForm.projects.p2.name = 'autre';
    app.projects = app.projects.filter((p) => p.id !== 'p2');
    expect(await settingsForm.save()).toBe(true);
    expect(backend.called('update_project')).toHaveLength(0);
  });
});
