import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { buffers } from '../lib/editor/buffers.svelte';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { PROJECT_COLORS } from '../lib/theme';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, gitInfo, project, resetApp } from '../test/ipc';
import ConfirmModal from './modals/ConfirmModal.svelte';
import TitleBar from './TitleBar.svelte';

async function closeProjectFromMenu(name: string) {
  await fireEvent.contextMenu(screen.getByRole('button', { name: new RegExp(name) }));
  menu.open!.items.find((i) => i.label.startsWith('Fermer le projet'))!.onClick!();
  if (app.modal?.kind !== 'confirm') throw new Error('no confirmation');
  render(ConfirmModal, app.modal);
  await userEvent.click(screen.getByRole('button', { name: 'Fermer le projet' }));
}

describe('TitleBar', () => {
  beforeEach(() =>
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web' })],
      agents: [agent({ status: 'waiting' }), agent({ id: 'b1', projectId: 'p2', status: 'running' })],
    }),
  );

  it('shows the app’s logo', () => {
    fakeBackend();
    render(TitleBar);
    expect(screen.getByRole('img', { name: 'Escouade' })).toHaveAttribute('src', '/logo.svg');
  });

  it('makes the tab of a project with an agent to look at blink, in the color of what happened', async () => {
    fakeBackend();
    render(TitleBar);
    const other = screen.getByRole('button', { name: /studio-web/ });
    expect(other).not.toHaveClass('alert');
    app.attention = { b1: true };
    app.agents.b1 = { ...app.agents.b1, status: 'error' };
    await Promise.resolve();
    expect(other).toHaveClass('alert');
    expect(other.style.getPropertyValue('--alert')).toBe('var(--del)');
    expect(other).toHaveAttribute('title', expect.stringContaining('À voir'));
    // The active project's own tab does not blink: its agent's card does.
    app.attention = { a1: true };
    await Promise.resolve();
    expect(screen.getByRole('button', { name: /demo-api/ })).not.toHaveClass('alert');
  });

  it('shows one tab per project with its waiting badge and git counter', () => {
    fakeBackend();
    app.git = { p1: gitInfo({ modified: 2, added: 1, total: 3 }) };
    render(TitleBar);
    const tab = screen.getByRole('button', { name: /demo-api/ });
    expect(tab).toHaveTextContent('Δ 3');
    expect(tab.querySelector('.pill')).toHaveTextContent('1');
    expect(screen.getByRole('button', { name: /studio-web/ }).querySelector('.pill')).toBeNull();
  });

  it('opens « Vue d’ensemble » from its button, left of the project tabs, and goes back with it', async () => {
    fakeBackend();
    app.openStats();
    render(TitleBar);
    const button = screen.getByRole('button', { name: 'Vue d’ensemble' });
    expect(
      button.compareDocumentPosition(screen.getByRole('button', { name: /demo-api/ })) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(button).toHaveAttribute('title', expect.stringContaining('Ctrl+Maj+A'));
    expect(button).toHaveAttribute('aria-pressed', 'false');
    await userEvent.click(button);
    expect(app.ui.view).toBe('overview');
    expect(button).toHaveAttribute('aria-pressed', 'true');
    // Nor a project's tab, nor the statistics', is the one on screen.
    expect(screen.getByRole('button', { name: /Stats/ })).not.toHaveClass('active');
    expect(screen.getByRole('button', { name: /demo-api/ })).not.toHaveClass('active');
    await userEvent.click(button);
    expect(app.ui.view).toBe('stats');
  });

  it('switches project and opens the stats', async () => {
    fakeBackend();
    render(TitleBar);
    await userEvent.click(screen.getByRole('button', { name: /studio-web/ }));
    expect(app.project?.id).toBe('p2');
    await userEvent.click(screen.getByRole('button', { name: /Stats/ }));
    expect(app.ui.view).toBe('stats');
  });

  it('closes a project once the backend removed it', async () => {
    const backend = fakeBackend();
    render(TitleBar);
    await closeProjectFromMenu('studio-web');
    expect(backend.called('remove_project')[0].args).toEqual({ id: 'p2' });
    expect(app.projects.map((p) => p.id)).toEqual(['p1']);
  });

  it('warns that a closed project’s unsaved files are lost, then forgets its editor', async () => {
    const backend = fakeBackend({
      fs_read: () => ({ kind: 'text', text: 'a\n', size: 2, hash: 'h1', eol: 'lf', bom: false }),
      fs_base: () => null,
      set_unsaved: () => null,
    });
    await app.openEditor({ projectId: 'p2', source: 'project', path: 'x.ts' });
    const k = (await buffers.open('p2', 'project', 'x.ts')).key;
    buffers.edit(k, 'mine\n');
    render(TitleBar);
    await fireEvent.contextMenu(screen.getByRole('button', { name: /studio-web/ }));
    menu.open!.items.find((i) => i.label.startsWith('Fermer le projet'))!.onClick!();
    expect((app.modal as any).body).toMatch(/ 1 fichier non enregistré dans l’éditeur sera perdu\.$/);
    await (app.modal as any).onConfirm(false);
    expect(app.editor.p2).toBeUndefined();
    expect(buffers.all[k]).toBeUndefined();
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 0 });
  });

  it('stops the launch commands of a closed project without calling it a crash', async () => {
    const run = { id: 'c9', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: '' };
    resetApp({ projects: [project(), project({ id: 'p2', name: 'studio-web', runCommands: [run] })] });
    app.launches.c9 = { status: 'running', ptyId: 't9', name: 'Front', stopping: false, code: null, startedAt: 1 };
    let stoppingDuringRemoval = false;
    const backend = fakeBackend({ remove_project: () => void (stoppingDuringRemoval = app.launches.c9.stopping) });
    render(TitleBar);
    await closeProjectFromMenu('studio-web');
    // The backend kills them while removing the project: their exits are expected.
    expect(stoppingDuringRemoval).toBe(true);
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't9' });
    expect(app.launches.c9).toBeUndefined();
  });

  it('stops the test launches of the agents of a closed project too, and forgets them', async () => {
    const run = { id: 'c9', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: '' };
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web', runCommands: [run] })],
      agents: [agent(), agent({ id: 'b1', projectId: 'p2' })],
    });
    const running = (ptyId: string) => ({ status: 'running' as const, ptyId, name: 'web', stopping: false, code: null, startedAt: 1 });
    app.launches.c9 = running('t9');
    app.launches['test:b1:run:0'] = running('t8');
    app.launches['test:a1:run:0'] = running('t7');
    const seen: Record<string, boolean> = {};
    const backend = fakeBackend({
      remove_project: () => {
        for (const id of ['c9', 'test:b1:run:0', 'test:a1:run:0']) seen[id] = app.launches[id].stopping;
      },
    });
    render(TitleBar);
    await closeProjectFromMenu('studio-web');
    // The backend kills them with the project: their exits are expected, and the other project's stay.
    expect(seen).toEqual({ c9: true, 'test:b1:run:0': true, 'test:a1:run:0': false });
    expect(backend.called('term_kill').map((c) => c.args.id)).toEqual(['t9', 't8']);
    expect(Object.keys(app.launches)).toEqual(['test:a1:run:0']);
  });

  it('leaves the launch commands alone when the project could not be closed', async () => {
    const run = { id: 'c9', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: '' };
    resetApp({ projects: [project(), project({ id: 'p2', name: 'studio-web', runCommands: [run] })] });
    app.launches.c9 = { status: 'running', ptyId: 't9', name: 'Front', stopping: false, code: null, startedAt: 1 };
    const backend = fakeBackend({
      remove_project: () => {
        throw new Error('projet verrouillé');
      },
    });
    render(TitleBar);
    await closeProjectFromMenu('studio-web');
    expect(backend.called('term_kill')).toHaveLength(0);
    expect(app.launches.c9).toMatchObject({ status: 'running', stopping: false });
  });

  it('sets a project’s color from its tab menu', async () => {
    const backend = fakeBackend();
    render(TitleBar);
    await fireEvent.contextMenu(screen.getByRole('button', { name: /studio-web/ }));
    const colors = menu.open!.items.find((i) => i.label === 'Couleur')!.colors!;
    expect(colors.values).toEqual(PROJECT_COLORS);
    expect(colors.selected).toBe(project().color);
    colors.onPick(PROJECT_COLORS[3]);
    expect(app.projects.find((p) => p.id === 'p2')?.color).toBe(PROJECT_COLORS[3]);
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p2', color: PROJECT_COLORS[3] });
  });

  it('opens the settings of a project, its launch commands with them, from its tab menu', async () => {
    fakeBackend();
    render(TitleBar);
    await fireEvent.contextMenu(screen.getByRole('button', { name: /studio-web/ }));
    menu.open!.items.find((i) => i.label === 'Réglages du projet…')!.onClick!();
    expect(app.modal).toEqual({ kind: 'settings', tab: 'projects', projectId: 'p2' });
  });

  it('keeps the project when the backend could not remove it', async () => {
    fakeBackend({
      remove_project: () => {
        throw new Error('projet verrouillé');
      },
    });
    render(TitleBar);
    await closeProjectFromMenu('studio-web');
    expect(app.projects.map((p) => p.id)).toEqual(['p1', 'p2']);
    expect(app.toasts.at(-1)?.text).toMatch(/projet verrouillé/);
  });
});

describe('TitleBar editor button', () => {
  it('opens the editor on the active project', async () => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'studio-web' })] });
    app.ui.activeProject = 'p2';
    fakeBackend();
    render(TitleBar);
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }));
    expect(app.editor.p2).toMatchObject({ on: true, source: 'project' });
  });

  it('is not offered behind the statistics, where no project is shown', async () => {
    resetApp();
    app.ui.view = 'stats';
    fakeBackend();
    render(TitleBar);
    expect(screen.queryByRole('button', { name: 'Ouvrir l’éditeur du projet' })).not.toBeInTheDocument();
  });
});

describe('TitleBar in English', () => {
  beforeEach(() => {
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web' })],
      agents: [agent({ status: 'waiting' }), agent({ id: 'b1', projectId: 'p2', status: 'error' })],
    });
    setLang('en');
  });

  it('names its buttons, its tooltips and the shortcut of the overview in English', () => {
    fakeBackend();
    app.git = { p1: gitInfo({ total: 3 }) };
    app.attention = { b1: true };
    render(TitleBar);
    expect(screen.getByRole('button', { name: 'Overview' })).toHaveAttribute('title', 'All agents of all projects (Ctrl+Shift+A)');
    expect(screen.getByRole('button', { name: 'Stats' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add a project' })).toHaveAttribute('title', 'Add a project');
    expect(screen.getByRole('button', { name: 'Open the project’s editor' })).toHaveAttribute(
      'title',
      'Browse and edit the project’s files',
    );
    expect(screen.getByTitle('Uncommitted git changes')).toHaveTextContent('Δ 3');
    expect(screen.getByTitle('Agents waiting for an answer')).toHaveTextContent('1');
    expect(screen.getByRole('button', { name: /studio-web/ })).toHaveAttribute(
      'title',
      expect.stringContaining('Needs a look: refacto-auth'),
    );
    expect(screen.getByRole('button', { name: 'Minimize' })).toHaveAttribute('title', 'Minimize');
    expect(screen.getByRole('button', { name: 'Close' })).toHaveAttribute('title', 'Close (the app stays in the notification area)');
  });

  it('writes the menu of a project’s tab in English', async () => {
    fakeBackend();
    render(TitleBar);
    await fireEvent.contextMenu(screen.getByRole('button', { name: /demo-api/ }));
    const labels = menu.open!.items.map((i) => i.label);
    expect(labels).toEqual(expect.arrayContaining(['Rename…', 'Color', 'Project settings…', 'Open the folder', 'Close the project…']));
    expect(labels.some((l) => /a worktree per agent$/.test(l))).toBe(true);
    menu.open!.items.find((i) => i.label === 'Rename…')!.onClick!();
    expect(app.modal).toMatchObject({ kind: 'rename', title: 'Rename the project' });
  });
});
