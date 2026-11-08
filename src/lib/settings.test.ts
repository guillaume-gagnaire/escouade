import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import { board, fakeBackend, project, resetApp, SETTINGS } from '../test/ipc';
import { setLang } from './i18n';
import { SAVE_PAUSE_MS, SETTINGS_PAGES, settingsForm } from './settings.svelte';
import { app } from './state.svelte';
import type { RunCommand } from './types';

vi.mock('./terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const FRONT: RunCommand = { id: 'c1', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: 'web' };
const P2 = () => project({ id: 'p2', name: 'site', color: 'oklch(0.7 0.1 200)', board: board({ action: 'pr' }) });

describe('settingsForm', () => {
  beforeEach(() => {
    resetApp({ projects: [project({ runCommands: [FRONT] }), P2()] });
  });

  it('opens on Comptes et harness and the project on screen, or on the page and project asked for', () => {
    app.ui.activeProject = 'p2';
    settingsForm.open();
    expect(settingsForm.page).toBe('accounts');
    expect(settingsForm.projectId).toBe('p2');
    settingsForm.open({ page: 'board', projectId: 'p1' });
    expect(settingsForm.page).toBe('board');
    expect(settingsForm.projectId).toBe('p1');
    expect(settingsForm.project?.board.action).toBe('merge');
    // A project closed since the shortcut was made: the one on screen instead.
    settingsForm.open({ page: 'project', projectId: 'gone' });
    expect(settingsForm.projectId).toBe('p2');
  });

  it('has no project to set when there is none', () => {
    resetApp({ projects: [] });
    settingsForm.open({ page: 'project' });
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

  it('asks again whether Claude Code is found once its path or the distribution of WSL is saved', async () => {
    app.claudeFound = false;
    app.claudePathFound = false;
    app.enginesFound = [];
    const backend = fakeBackend({
      save_settings: () => [],
      claude_found: () => ({ claudeFound: true, claudePathFound: true, enginesFound: ['claude'] }),
    });
    settingsForm.open();
    settingsForm.settings.sound = false;
    await settingsForm.save();
    // Nothing of Claude Code changed: not asked.
    expect(backend.called('claude_found')).toHaveLength(0);
    expect(app.claudeFound).toBe(false);
    settingsForm.settings.claudePath = '~/.local/bin/claude';
    await settingsForm.save();
    expect(backend.called('claude_found')).toHaveLength(1);
    expect([app.claudeFound, app.claudePathFound]).toEqual([true, true]);
    // The harnesses offered with it: « Aucun harness trouvé » goes.
    expect(app.enginesFound).toEqual(['claude']);
    settingsForm.settings.wslDistro = 'Ubuntu-24.04';
    await settingsForm.save();
    expect(backend.called('claude_found')).toHaveLength(2);
  });

  it('does not save an emptied or negative number, says it, and saves the rest', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    settingsForm.open();
    (settingsForm.settings as any).idleStopMinutes = null;
    settingsForm.settings.attachWarnMb = -5;
    settingsForm.settings.attachKeepDays = 2.7;
    settingsForm.settings.sound = false;
    expect([...settingsForm.badNumbers].sort()).toEqual(['attachWarnMb', 'idleStopMinutes']);
    await settingsForm.save();
    const sent = backend.called('save_settings')[0].args.settings;
    // The values saved before stay; the whole number of the other is taken.
    expect(sent).toMatchObject({
      idleStopMinutes: SETTINGS.idleStopMinutes,
      attachWarnMb: SETTINGS.attachWarnMb,
      attachKeepDays: 2,
      sound: false,
    });
    expect(app.settings.idleStopMinutes).toBe(SETTINGS.idleStopMinutes);
    // Fixed: saved in its turn.
    settingsForm.settings.attachWarnMb = 40;
    (settingsForm.settings as any).idleStopMinutes = 15;
    expect(settingsForm.badNumbers.size).toBe(0);
    await settingsForm.save();
    expect(backend.called('save_settings')[1].args.settings).toMatchObject({ attachWarnMb: 40, idleStopMinutes: 15 });
  });

  it('sees a change of the attachments on the page Avancé, and has none once it is saved', async () => {
    fakeBackend({ save_settings: () => [] });
    settingsForm.open();
    expect(settingsForm.changed('advanced')).toBe(false);
    settingsForm.settings.attachWarnMb = 50;
    expect(settingsForm.changed('advanced')).toBe(true);
    settingsForm.settings.attachWarnMb = 200;
    settingsForm.settings.attachBigText = 'refuse';
    expect(settingsForm.changed('advanced')).toBe(true);
    await settingsForm.save();
    expect(settingsForm.changed('advanced')).toBe(false);
    expect(app.settings.attachBigText).toBe('refuse');
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
    settingsForm.open({ page: 'project', projectId: 'p1' });
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

  it('saves the worktree steps trimmed, without those that have no command', async () => {
    const backend = fakeBackend();
    app.shells = [{ id: 'bash', label: 'Git Bash', path: 'bash.exe' }];
    settingsForm.open({ page: 'project', projectId: 'p1' });
    settingsForm.addStep('setup');
    settingsForm.addStep('setup');
    settingsForm.addStep('teardown');
    const [first, empty] = settingsForm.projects.p1.worktreeSetup;
    expect(first).toMatchObject({ command: '', shell: 'bash', cwd: '' });
    Object.assign(first, { command: ' npm ci ', cwd: ' web ' });
    empty.command = '   ';
    settingsForm.projects.p1.worktreeTeardown[0].command = 'docker compose down';
    expect(settingsForm.changed('git')).toBe(true);
    expect(await settingsForm.save()).toBe(true);

    const saved = backend.called('update_project')[0].args.project;
    expect(saved.worktreeSetup).toEqual([{ id: first.id, command: 'npm ci', shell: 'bash', cwd: 'web' }]);
    expect(saved.worktreeTeardown).toMatchObject([{ command: 'docker compose down', shell: 'bash', cwd: '' }]);
    expect(app.projects[0].worktreeSetup).toEqual(saved.worktreeSetup);
    expect(settingsForm.changed('git')).toBe(false);
  });

  it("fills the worktree steps with Claude's suggestion, and says when it has none", async () => {
    let answer!: (v: unknown) => void;
    const backend = fakeBackend({
      suggest_worktree_steps: () => new Promise((r) => (answer = r)),
    });
    settingsForm.open({ page: 'project', projectId: 'p1' });
    settingsForm.addStep('teardown');
    const asked = settingsForm.suggest('p1');
    expect(settingsForm.suggesting.p1).toBe(true);
    expect(backend.called('suggest_worktree_steps')[0].args).toEqual({ projectId: 'p1' });
    const ci = { id: 's1', command: 'npm ci', shell: 'pwsh', cwd: '' };
    answer({ setup: [ci], teardown: [], refused: 0 });
    await asked;
    expect(settingsForm.suggesting.p1).toBe(false);
    expect(app.toasts.at(-1)?.text).toBe('1 commande proposée : relis-la avant de remplacer les commandes.');
    // Shown apart, to be read in full: the draft is left as it is until it is taken.
    expect(settingsForm.proposal.p1).toEqual({ setup: [ci], teardown: [], refused: 0 });
    expect(settingsForm.projects.p1.worktreeSetup).toEqual([]);
    expect(settingsForm.projects.p1.worktreeTeardown).toHaveLength(1);
    settingsForm.takeProposal('p1');
    expect(settingsForm.proposal.p1).toBeUndefined();
    expect(settingsForm.projects.p1.worktreeSetup).toEqual([ci]);
    expect(settingsForm.projects.p1.worktreeTeardown).toEqual([]);

    // Some were refused (a line break, invisible characters, a long run of blanks, too long): said, as for the launch commands.
    const down = { id: 's2', command: 'docker compose down', shell: 'pwsh', cwd: '' };
    fakeBackend({ suggest_worktree_steps: () => ({ setup: [ci], teardown: [down], refused: 1 }) });
    await settingsForm.suggest('p1');
    settingsForm.takeProposal('p1');
    expect(settingsForm.projects.p1.worktreeTeardown).toEqual([down]);
    expect(app.toasts.at(-1)?.text).toBe(
      '2 commandes proposées, 1 écartée (caractères invisibles ou trop longue) : relis-les avant de remplacer les commandes.',
    );
    fakeBackend({ suggest_worktree_steps: () => ({ setup: [ci], teardown: [], refused: 2 }) });
    await settingsForm.suggest('p1');
    expect(app.toasts.at(-1)?.text).toBe(
      '1 commande proposée, 2 écartées (caractères invisibles ou trop longues) : relis-la avant de remplacer les commandes.',
    );

    settingsForm.takeProposal('p1');

    fakeBackend({ suggest_worktree_steps: () => ({ setup: [], teardown: [], refused: 0 }) });
    await settingsForm.suggest('p1');
    // Nothing found: what was there stays, and nothing is proposed.
    expect(settingsForm.projects.p1.worktreeSetup).toEqual([ci]);
    expect(settingsForm.proposal.p1).toBeUndefined();
    expect(app.toasts.at(-1)?.text).toBe("Claude n'a trouvé aucune commande à lancer pour ce projet.");

    // All refused is told by the backend, as a failure and not as nothing found.
    fakeBackend({
      suggest_worktree_steps: () => Promise.reject("Claude a proposé des commandes illisibles : aucune n'a été gardée."),
    });
    await settingsForm.suggest('p1');
    expect(settingsForm.projects.p1.worktreeSetup).toEqual([ci]);
    expect(app.toasts.at(-1)).toMatchObject({ text: "Claude a proposé des commandes illisibles : aucune n'a été gardée.", kind: 'error' });

    fakeBackend({
      suggest_worktree_steps: () => Promise.reject("Claude n'a pas proposé de commandes lisibles."),
    });
    await settingsForm.suggest('p1');
    expect(settingsForm.suggesting.p1).toBe(false);
    expect(settingsForm.projects.p1.worktreeSetup).toHaveLength(1);
    expect(app.toasts.at(-1)).toMatchObject({ text: "Claude n'a pas proposé de commandes lisibles.", kind: 'error' });
  });

  it("fills the launch commands with Claude's suggestion in place of the draft's, and says when it has none", async () => {
    let answer!: (v: unknown) => void;
    const backend = fakeBackend({ suggest_run_commands: () => new Promise((r) => (answer = r)) });
    settingsForm.open({ page: 'project', projectId: 'p1' });
    expect(settingsForm.changed('launch')).toBe(false);
    const asked = settingsForm.suggestLaunch('p1');
    expect(settingsForm.suggestingLaunch.p1).toBe(true);
    // The worktree steps' reading is another one.
    expect(settingsForm.suggesting.p1).toBeFalsy();
    // Asked once at a time.
    void settingsForm.suggestLaunch('p1');
    expect(backend.called('suggest_run_commands')).toHaveLength(1);
    expect(backend.called('suggest_run_commands')[0].args).toEqual({ projectId: 'p1' });
    const web = { id: 's1', name: 'Front', command: 'npm run dev', shell: 'bash', cwd: 'web' };
    const db = { id: 's2', name: 'Base', command: 'docker compose up db', shell: 'bash', cwd: '' };
    answer({ commands: [web, db], refused: 0 });
    await asked;
    expect(settingsForm.suggestingLaunch.p1).toBe(false);
    expect(app.toasts.at(-1)?.text).toBe('2 commandes proposées : relis-les avant de remplacer les commandes.');
    // Shown apart, to be read in full: the draft keeps FRONT until it is taken.
    expect(settingsForm.launchProposal.p1).toEqual([web, db]);
    expect(settingsForm.projects.p1.runCommands).toEqual([FRONT]);
    expect(settingsForm.changed('launch')).toBe(false);
    // Taken, it replaces the draft's command.
    settingsForm.takeLaunchProposal('p1');
    expect(settingsForm.launchProposal.p1).toBeUndefined();
    expect(settingsForm.projects.p1.runCommands).toEqual([web, db]);
    // A draft like another: it shows as a change, and cancelling drops it.
    expect(settingsForm.changed('launch')).toBe(true);
    expect(app.projects[0].runCommands).toEqual([FRONT]);
    settingsForm.open({ page: 'project', projectId: 'p1' });
    expect(settingsForm.projects.p1.runCommands).toEqual([FRONT]);

    fakeBackend({ suggest_run_commands: () => ({ commands: [web], refused: 0 }) });
    await settingsForm.suggestLaunch('p1');
    expect(app.toasts.at(-1)?.text).toBe('1 commande proposée : relis-la avant de remplacer les commandes.');
    // A proposal not taken goes with the draft too.
    settingsForm.open({ page: 'project', projectId: 'p1' });
    expect(settingsForm.launchProposal.p1).toBeUndefined();

    // Some were refused (a line break, invisible characters, a long run of blanks, too long): said, in the singular and the plural.
    fakeBackend({ suggest_run_commands: () => ({ commands: [web, db], refused: 1 }) });
    await settingsForm.suggestLaunch('p1');
    settingsForm.takeLaunchProposal('p1');
    expect(settingsForm.projects.p1.runCommands).toEqual([web, db]);
    expect(app.toasts.at(-1)?.text).toBe(
      '2 commandes proposées, 1 écartée (caractères invisibles ou trop longue) : relis-les avant de remplacer les commandes.',
    );
    fakeBackend({ suggest_run_commands: () => ({ commands: [web], refused: 3 }) });
    await settingsForm.suggestLaunch('p1');
    expect(app.toasts.at(-1)?.text).toBe(
      '1 commande proposée, 3 écartées (caractères invisibles ou trop longues) : relis-la avant de remplacer les commandes.',
    );
    settingsForm.takeLaunchProposal('p1');

    fakeBackend({ suggest_run_commands: () => ({ commands: [], refused: 0 }) });
    await settingsForm.suggestLaunch('p1');
    // Nothing found: what was there stays.
    expect(settingsForm.projects.p1.runCommands).toEqual([web]);
    expect(app.toasts.at(-1)?.text).toBe("Claude n'a trouvé aucune commande à lancer pour ce projet.");

    // All refused is told by the backend, as a failure and not as nothing found.
    fakeBackend({ suggest_run_commands: () => Promise.reject("Claude a proposé des commandes illisibles : aucune n'a été gardée.") });
    await settingsForm.suggestLaunch('p1');
    expect(settingsForm.projects.p1.runCommands).toEqual([web]);
    expect(app.toasts.at(-1)).toMatchObject({ text: "Claude a proposé des commandes illisibles : aucune n'a été gardée.", kind: 'error' });

    fakeBackend({ suggest_run_commands: () => Promise.reject("Claude n'a pas proposé de commandes lisibles.") });
    await settingsForm.suggestLaunch('p1');
    expect(settingsForm.suggestingLaunch.p1).toBe(false);
    expect(settingsForm.projects.p1.runCommands).toEqual([web]);
    expect(app.toasts.at(-1)).toMatchObject({ text: "Claude n'a pas proposé de commandes lisibles.", kind: 'error' });
  });

  it('saves the launch commands Claude suggested like those typed', async () => {
    const backend = fakeBackend({
      suggest_run_commands: () => ({
        commands: [{ id: 's1', name: ' Front ', command: ' npm run dev ', shell: 'bash', cwd: 'web' }],
        refused: 0,
      }),
    });
    settingsForm.open({ page: 'project', projectId: 'p1' });
    await settingsForm.suggestLaunch('p1');
    settingsForm.takeLaunchProposal('p1');
    expect(await settingsForm.save()).toBe(true);
    expect(backend.called('update_project')[0].args.project.runCommands).toEqual([
      { id: 's1', name: 'Front', command: 'npm run dev', shell: 'bash', cwd: 'web' },
    ]);
    // The command it replaced is gone for good, like one removed by hand.
    expect(app.projects[0].runCommands.map((c) => c.id)).toEqual(['s1']);
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

  it('does not save a project without a name nor commands one of which lacks a name or a line, and saves the rest', async () => {
    const backend = fakeBackend();
    settingsForm.open({ projectId: 'p1' });
    expect(settingsForm.projectProblems('p2')).toMatchObject({ name: false, commands: false });
    settingsForm.projects.p2.name = ' ';
    expect(settingsForm.projectProblems('p2').name).toBe(true);
    expect(await settingsForm.save()).toBe(true);
    expect(backend.calls).toEqual([]);
    settingsForm.projects.p1.runCommands[0].command = '';
    settingsForm.projects.p1.worktreePerAgent = true;
    expect(settingsForm.projectProblems('p1').commands).toBe(true);
    await settingsForm.save();
    // Only what can be: the commands saved stay those of before.
    expect(backend.called('update_project')).toHaveLength(1);
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p1', worktreePerAgent: true, runCommands: [FRONT] });
    expect(app.projects[1].name).toBe('site');
    // Still a change, to save once fixed.
    expect(settingsForm.changed('launch', 'p1')).toBe(true);
    settingsForm.projects.p1.runCommands[0].command = 'npm start';
    await settingsForm.save();
    expect(backend.called('update_project')[1].args.project.runCommands[0].command).toBe('npm start');
  });

  it('saves once after a pause, and at once when flushed', async () => {
    vi.useFakeTimers();
    try {
      const backend = fakeBackend({ save_settings: () => [] });
      settingsForm.open();
      settingsForm.settings.proxyUrl = 'http://p';
      settingsForm.schedule();
      settingsForm.settings.proxyUrl = 'http://proxy:3128';
      settingsForm.schedule();
      await vi.advanceTimersByTimeAsync(SAVE_PAUSE_MS - 1);
      expect(backend.called('save_settings')).toHaveLength(0);
      await vi.advanceTimersByTimeAsync(1);
      await vi.runAllTimersAsync();
      expect(backend.called('save_settings')).toHaveLength(1);
      expect(backend.called('save_settings')[0].args.settings.proxyUrl).toBe('http://proxy:3128');
      // A choice: at once, the pause waiting dropped.
      settingsForm.settings.sound = false;
      settingsForm.schedule();
      await settingsForm.flush();
      expect(backend.called('save_settings')).toHaveLength(2);
      await vi.runAllTimersAsync();
      expect(backend.called('save_settings')).toHaveLength(2);
    } finally {
      vi.useRealTimers();
    }
  });

  it('saves one at a time: what changes while a save runs is saved by the next', async () => {
    let release!: () => void;
    let n = 0;
    const backend = fakeBackend({
      save_settings: () => (++n === 1 ? new Promise((r) => (release = () => r([]))) : []),
    });
    settingsForm.open();
    settingsForm.settings.sound = false;
    const first = settingsForm.flush();
    await new Promise((r) => setTimeout(r));
    settingsForm.settings.proxyUrl = 'http://proxy:3128';
    const second = settingsForm.flush();
    await new Promise((r) => setTimeout(r));
    expect(backend.called('save_settings')).toHaveLength(1);
    release();
    await first;
    await second;
    expect(backend.called('save_settings')).toHaveLength(2);
    expect(backend.called('save_settings')[1].args.settings).toMatchObject({ sound: false, proxyUrl: 'http://proxy:3128' });
  });

  it('marks the pages whose settings changed', () => {
    settingsForm.open();
    expect(settingsForm.changed('agents')).toBe(false);
    settingsForm.settings.idleStopMinutes = 5;
    settingsForm.settings.proxyTerminals = true;
    settingsForm.projects.p2.board.maxParallel = 5;
    expect(settingsForm.changed('agents')).toBe(true);
    expect(settingsForm.changed('advanced')).toBe(true);
    expect(settingsForm.changed('board')).toBe(true);
    expect(settingsForm.changed('notifications')).toBe(false);
    expect(settingsForm.changed('voice')).toBe(false);
    expect(settingsForm.changed('git')).toBe(false);
    settingsForm.projects.p1.copy = '.env*\n';
    // The same patterns: nothing to save.
    expect(settingsForm.changed('git')).toBe(false);
    settingsForm.projects.p1.copy = '.env.local';
    expect(settingsForm.changed('git')).toBe(true);
  });

  it('marks the quota pause on Comptes et harness, not on Kanban', () => {
    settingsForm.open();
    expect(settingsForm.changed('accounts')).toBe(false);
    settingsForm.settings.quotaPause = 90;
    expect(settingsForm.changed('accounts')).toBe(true);
    expect(settingsForm.changed('board')).toBe(false);
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
    expect(settingsForm.changed('project')).toBe(false);
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
    expect(settingsForm.changed('project')).toBe(true);
  });

  it('resumes the draft it had, on the page and project asked for, or the page of a 2.0 tab', () => {
    settingsForm.open({ projectId: 'p1' });
    settingsForm.settings.proxyUrl = 'http://proxy:3128';
    settingsForm.resume({ page: 'project', projectId: 'p2' });
    expect(settingsForm.page).toBe('project');
    expect(settingsForm.projectId).toBe('p2');
    expect(settingsForm.settings.proxyUrl).toBe('http://proxy:3128');
    // Its project closed meanwhile: the one on screen.
    app.projects = app.projects.filter((p) => p.id !== 'p2');
    settingsForm.resume({ page: 'project', projectId: 'p2' });
    expect(settingsForm.projectId).toBe('p1');
  });

  it('saves a project’s links on its page Tickets importés alone, without its empty ones', async () => {
    const backend = fakeBackend();
    settingsForm.open({ page: 'tickets', projectId: 'p1' });
    expect(settingsForm.changed('tickets')).toBe(false);
    settingsForm.projects.p1.integrations.links.push(
      { service: 'jira', container: 'ATL', name: 'ATL — Atlas', states: { doing: { id: '3', name: 'In Progress' } } },
      { service: 'trello', container: '', name: '', states: {} },
    );
    settingsForm.projects.p1.integrations.comments = ['done'];
    expect(settingsForm.changed('tickets')).toBe(true);
    expect(settingsForm.changed('project')).toBe(false);
    expect(await settingsForm.save()).toBe(true);
    const sent = backend.called('update_project')[0].args.project;
    expect(sent.integrations).toEqual({
      links: [{ service: 'jira', container: 'ATL', name: 'ATL — Atlas', states: { doing: { id: '3', name: 'In Progress' } } }],
      comments: ['done'],
    });
    expect(sent.name).toBe('demo-api');
    expect(app.projects[0].integrations.links.map((l) => l.container)).toEqual(['ATL']);
    expect(settingsForm.changed('tickets')).toBe(false);
  });

  it('counts the sync settings as the page Services connectés’s', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    settingsForm.open({ page: 'services' });
    settingsForm.settings.integrations.autoImport = true;
    settingsForm.settings.integrations.importLabel = 'escouade';
    expect(settingsForm.changed('services')).toBe(true);
    expect(settingsForm.changed('tickets')).toBe(false);
    await settingsForm.save();
    expect(backend.called('save_settings')[0].args.settings.integrations).toMatchObject({ autoImport: true, importLabel: 'escouade' });
    expect(backend.called('update_project')).toHaveLength(0);
  });

  it('skips a project closed while its settings were open', async () => {
    const backend = fakeBackend();
    settingsForm.open({ projectId: 'p2' });
    settingsForm.projects.p2.name = 'autre';
    app.projects = app.projects.filter((p) => p.id !== 'p2');
    expect(await settingsForm.save()).toBe(true);
    expect(backend.called('update_project')).toHaveLength(0);
  });

  it('names the pages in the language of the interface, read when asked', () => {
    const names = () => SETTINGS_PAGES.map((x) => x.label);
    expect(names()[0]).toBe('Général');
    setLang('en');
    flushSync();
    expect(names()).toEqual([
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
  });

  it('tells in English what Claude suggested, in the singular and the plural of what it proposed and what was left out', async () => {
    setLang('en');
    const ci = { id: 's1', name: 'Front', command: 'npm run dev', shell: 'bash', cwd: '' };
    const db = { ...ci, id: 's2', name: 'Base' };
    settingsForm.open({ page: 'project', projectId: 'p1' });
    const said = async (commands: unknown[], refused: number) => {
      fakeBackend({ suggest_run_commands: () => ({ commands, refused }) });
      await settingsForm.suggestLaunch('p1');
      return app.toasts.at(-1)?.text;
    };
    expect(await said([ci], 0)).toBe('1 command suggested: review it before replacing the commands.');
    expect(await said([ci, db], 0)).toBe('2 commands suggested: review them before replacing the commands.');
    expect(await said([ci], 1)).toBe(
      '1 command suggested, 1 left out (invisible characters or too long): review it before replacing the commands.',
    );
    expect(await said([ci, db], 1)).toBe(
      '2 commands suggested, 1 left out (invisible characters or too long): review them before replacing the commands.',
    );
    expect(await said([ci], 3)).toBe(
      '1 command suggested, 3 left out (invisible characters or too long): review it before replacing the commands.',
    );
    expect(await said([ci, db], 3)).toBe(
      '2 commands suggested, 3 left out (invisible characters or too long): review them before replacing the commands.',
    );
    expect(await said([], 0)).toBe('Claude found no command to run for this project.');
  });
});
