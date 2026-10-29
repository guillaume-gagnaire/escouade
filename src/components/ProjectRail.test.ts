import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { buffers } from '../lib/editor/buffers.svelte';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { initials } from '../lib/project-rail';
import { handleShortcut } from '../lib/shortcuts';
import { app } from '../lib/state.svelte';
import { PROJECT_COLORS } from '../lib/theme';
import { agent, fakeBackend, gitInfo, project, resetApp } from '../test/ipc';
import ConfirmModal from './modals/ConfirmModal.svelte';
import ProjectRail from './ProjectRail.svelte';

const tile = (name: string | RegExp) => screen.getByRole('tab', { name: typeof name === 'string' ? new RegExp(`^${name}`) : name });
const tiles = () => screen.getAllByRole('tab');

async function closeProjectFromMenu(name: string) {
  await fireEvent.contextMenu(tile(name));
  menu.open!.items.find((i) => i.label.startsWith('Fermer le projet'))!.onClick!();
  if (app.modal?.kind !== 'confirm') throw new Error('no confirmation');
  render(ConfirmModal, app.modal);
  await userEvent.click(screen.getByRole('button', { name: 'Fermer le projet' }));
}

describe('initials', () => {
  it('takes the initials of the first two words, else the first two letters', () => {
    expect(initials('demo-api')).toBe('DA');
    expect(initials('mobile-app')).toBe('MA');
    expect(initials('studio_web front')).toBe('SW');
    expect(initials('infra')).toBe('In');
    expect(initials('x')).toBe('X');
    expect(initials('élan-vital')).toBe('ÉV');
  });
});

describe('ProjectRail', () => {
  beforeEach(() => {
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web', color: PROJECT_COLORS[1] }), project({ id: 'p3', name: 'infra' })],
      agents: [
        agent({ status: 'waiting' }),
        agent({ id: 'a2', status: 'waiting' }),
        agent({ id: 'b1', projectId: 'p2', status: 'running' }),
        agent({ id: 'b2', projectId: 'p2', status: 'running' }),
      ],
    });
    menu.close();
  });

  it('has a tile per open project, in their order, with their colour and their two letters', () => {
    fakeBackend();
    render(ProjectRail);
    expect(screen.getByRole('tablist', { name: 'Projets ouverts' })).toBeInTheDocument();
    // Their letters and badges, their labels (ESC-168) aside.
    const shown = (t: HTMLElement) => {
      const c = t.cloneNode(true) as HTMLElement;
      c.querySelector('.lbl')?.remove();
      return c.textContent?.replace(/\s+/g, '');
    };
    expect(tiles().map(shown)).toEqual(['DA2', 'SW', 'In']);
    expect(tile('studio-web').style.getPropertyValue('--c')).toBe(PROJECT_COLORS[1]);
  });

  it('marks the active project, aria-selected, and none behind « Vue d’ensemble », whose own tile is pressed', async () => {
    fakeBackend();
    render(ProjectRail);
    expect(tile('demo-api')).toHaveAttribute('aria-selected', 'true');
    expect(tile('studio-web')).toHaveAttribute('aria-selected', 'false');
    // Only the active one is in the tab order; the arrows move along the others.
    expect(tile('demo-api')).toHaveAttribute('tabindex', '0');
    expect(tile('studio-web')).toHaveAttribute('tabindex', '-1');
    const overview = screen.getByRole('button', { name: 'Vue d’ensemble' });
    expect(overview).toHaveAttribute('aria-pressed', 'false');
    await userEvent.click(overview);
    expect(app.ui.view).toBe('overview');
    expect(overview).toHaveAttribute('aria-pressed', 'true');
    expect(tile('demo-api')).toHaveAttribute('aria-selected', 'false');
  });

  it('shows the agents waiting in an amber pill, at work in a dot, and says them to a screen reader', () => {
    fakeBackend();
    app.git = { p1: gitInfo({ total: 7 }) };
    render(ProjectRail);
    expect(tile('demo-api').querySelector('.ask')).toHaveTextContent('2');
    expect(tile('demo-api').querySelector('.busy')).toBeNull();
    expect(tile('studio-web').querySelector('.busy')).not.toBeNull();
    expect(tile('studio-web').querySelector('.ask')).toBeNull();
    expect(tile('demo-api')).toHaveAccessibleName('demo-api, 2 en attente, Δ 7');
    expect(tile('studio-web')).toHaveAccessibleName('studio-web, 2 au travail');
  });

  it('puts the name, Ctrl 1…9, what waits and works and the Δ in a tooltip beside the tile', () => {
    fakeBackend();
    app.git = { p1: gitInfo({ total: 7 }) };
    render(ProjectRail);
    expect(tile('demo-api')).toHaveAttribute(
      'data-tip',
      'demo-api (Ctrl 1)\n2 agents attendent ta réponse\nΔ 7 · modifications non commitées',
    );
    expect(tile('studio-web')).toHaveAttribute('data-tip', 'studio-web (Ctrl 2)\n2 agents travaillent');
    expect(tile('infra')).toHaveAttribute('data-tip', 'infra (Ctrl 3)\nRien en cours, rien de modifié');
  });

  it('marks a private project with a lock on its tile, said in its tooltip (choice 28)', () => {
    fakeBackend();
    app.projects[2] = { ...app.projects[2], private: true };
    render(ProjectRail);
    expect(tile('infra').querySelector('.lock [data-icon="lock"]')).not.toBeNull();
    expect(tile('infra')).toHaveAccessibleName('infra, privé');
    expect(tile('infra').dataset.tip).toContain('Projet privé');
    expect(tile('demo-api').querySelector('.lock')).toBeNull();
  });

  it('rings a project with an agent to look at, in the colour of what happened, and says who in its tooltip', async () => {
    fakeBackend();
    render(ProjectRail);
    expect(tile('studio-web')).not.toHaveClass('alert');
    app.attention = { b1: true };
    app.agents.b1 = { ...app.agents.b1, status: 'error' };
    await tick();
    expect(tile('studio-web')).toHaveClass('alert');
    expect(tile('studio-web').style.getPropertyValue('--alert')).toBe('var(--del)');
    expect(tile('studio-web').dataset.tip).toContain('À voir : refacto-auth');
    // The active project's tile does not: its agent's card does.
    app.attention = { a1: true };
    await tick();
    expect(tile('demo-api')).not.toHaveClass('alert');
  });

  it('changes project by a click, and by Ctrl+1…9, on an AZERTY keyboard too', async () => {
    fakeBackend();
    render(ProjectRail);
    await userEvent.click(tile('studio-web'));
    expect(app.ui.activeProject).toBe('p2');
    await tick();
    expect(tile('studio-web')).toHaveAttribute('aria-selected', 'true');
    // AZERTY: the key « 3 » gives « " », its code is still Digit3.
    expect(handleShortcut(new KeyboardEvent('keydown', { key: '"', code: 'Digit3', ctrlKey: true }), false)).toBe(true);
    expect(app.ui.activeProject).toBe('p3');
  });

  it('brings the active tile into view, however its project was chosen', async () => {
    fakeBackend();
    const seen = vi.fn();
    Element.prototype.scrollIntoView = seen;
    render(ProjectRail);
    seen.mockClear();
    app.selectProject('p3');
    await tick();
    expect(seen).toHaveBeenCalled();
    expect(seen.mock.contexts.at(-1)).toBe(tile('infra'));
  });

  it('moves along the tiles with the arrows', async () => {
    fakeBackend();
    render(ProjectRail);
    tile('demo-api').focus();
    await fireEvent.keyDown(tile('demo-api'), { key: 'ArrowDown' });
    expect(tile('studio-web')).toHaveFocus();
    await fireEvent.keyDown(tile('studio-web'), { key: 'End' });
    expect(tile('infra')).toHaveFocus();
  });

  it('reorders by Alt+↑ and Alt+↓ and saves the order, the tile keeping the focus', async () => {
    const backend = fakeBackend();
    render(ProjectRail);
    tile('studio-web').focus();
    await fireEvent.keyDown(tile('studio-web'), { key: 'ArrowUp', altKey: true });
    expect(app.projects.map((p) => p.id)).toEqual(['p2', 'p1', 'p3']);
    expect(backend.called('reorder_projects').at(-1)?.args).toEqual({ ids: ['p2', 'p1', 'p3'] });
    await tick();
    await Promise.resolve();
    expect(tile('studio-web')).toHaveFocus();
    // Nothing past the top.
    await fireEvent.keyDown(tile('studio-web'), { key: 'ArrowUp', altKey: true });
    expect(backend.called('reorder_projects')).toHaveLength(1);
    await fireEvent.keyDown(tile('studio-web'), { key: 'ArrowDown', altKey: true });
    expect(app.projects.map((p) => p.id)).toEqual(['p1', 'p2', 'p3']);
  });

  it('reorders by drag and drop, the dragged tile hollow and a line where it lands, and saves the order', async () => {
    const backend = fakeBackend();
    render(ProjectRail);
    const rect = (top: number) => ({ top, bottom: top + 36, height: 36, left: 8, right: 44, width: 36, x: 8, y: top }) as DOMRect;
    tiles().forEach((t, i) => (t.getBoundingClientRect = () => rect(10 + i * 44)));
    await fireEvent.dragStart(tile('demo-api'));
    expect(tile('demo-api')).toHaveClass('dragging');
    // Over the lower half of the last tile: after it.
    await fireEvent.dragOver(tile('infra'), { clientY: 10 + 2 * 44 + 30 });
    expect(document.querySelector('.drop')).not.toBeNull();
    await fireEvent.drop(tile('infra'));
    expect(app.projects.map((p) => p.id)).toEqual(['p2', 'p3', 'p1']);
    expect(backend.called('reorder_projects').at(-1)?.args).toEqual({ ids: ['p2', 'p3', 'p1'] });
    expect(document.querySelector('.drop')).toBeNull();
  });

  it('opens the project’s menu on a right click, with Monter and Descendre', async () => {
    fakeBackend();
    render(ProjectRail);
    await fireEvent.contextMenu(tile('studio-web'));
    const labels = menu.open!.items.filter((i) => !i.separator).map((i) => i.label);
    expect(labels).toEqual([
      'Renommer…',
      'Couleur',
      'Activer le worktree par agent',
      'Réglages du projet…',
      'Ouvrir le dossier',
      'Monter',
      'Descendre',
      'Fermer le projet…',
    ]);
    expect(menu.open!.items.find((i) => i.label === 'Monter')).toMatchObject({ hint: 'Alt ↑', disabled: false });
    menu.open!.items.find((i) => i.label === 'Monter')!.onClick!();
    expect(app.projects.map((p) => p.id)).toEqual(['p2', 'p1', 'p3']);
    // The first can go no higher, the last no lower.
    await fireEvent.contextMenu(tile('studio-web'));
    expect(menu.open!.items.find((i) => i.label === 'Monter')!.disabled).toBe(true);
    await fireEvent.contextMenu(tile('infra'));
    expect(menu.open!.items.find((i) => i.label === 'Descendre')!.disabled).toBe(true);
  });

  it('sets a project’s color from its menu', async () => {
    const backend = fakeBackend();
    render(ProjectRail);
    await fireEvent.contextMenu(tile('studio-web'));
    const colors = menu.open!.items.find((i) => i.label === 'Couleur')!.colors!;
    expect(colors.values).toEqual(PROJECT_COLORS);
    expect(colors.selected).toBe(PROJECT_COLORS[1]);
    colors.onPick(PROJECT_COLORS[3]);
    expect(app.projects.find((p) => p.id === 'p2')?.color).toBe(PROJECT_COLORS[3]);
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p2', color: PROJECT_COLORS[3] });
  });

  it('opens the settings of a project from its menu', async () => {
    fakeBackend();
    render(ProjectRail);
    await fireEvent.contextMenu(tile('studio-web'));
    menu.open!.items.find((i) => i.label === 'Réglages du projet…')!.onClick!();
    expect(app.modal).toEqual({ kind: 'settings', tab: 'projects', projectId: 'p2' });
  });

  it('closes a project once the backend removed it, its history kept', async () => {
    const backend = fakeBackend();
    render(ProjectRail);
    await closeProjectFromMenu('studio-web');
    expect(backend.called('remove_project')[0].args).toEqual({ id: 'p2', erase: false });
    expect(app.projects.map((p) => p.id)).toEqual(['p1', 'p3']);
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
    render(ProjectRail);
    await fireEvent.contextMenu(tile('studio-web'));
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
    render(ProjectRail);
    await closeProjectFromMenu('studio-web');
    expect(stoppingDuringRemoval).toBe(true);
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't9' });
    expect(app.launches.c9).toBeUndefined();
  });

  it('keeps the project when the backend could not remove it', async () => {
    fakeBackend({
      remove_project: () => {
        throw new Error('projet verrouillé');
      },
    });
    render(ProjectRail);
    await closeProjectFromMenu('studio-web');
    expect(app.projects.map((p) => p.id)).toEqual(['p1', 'p2', 'p3']);
    expect(app.toasts.at(-1)?.text).toMatch(/projet verrouillé/);
  });

  it('opens the assistant from « + » when no project was closed', async () => {
    fakeBackend({ closed_projects: () => [] });
    render(ProjectRail);
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter un projet' }));
    expect(app.modal).toEqual({ kind: 'newProject' });
  });

  it('opens a menu from « + » with the closed projects to reopen, and all of them in the settings (choice 47)', async () => {
    const closed = { id: 'old', name: 'legacy-billing', color: PROJECT_COLORS[6], path: 'C:\\code\\legacy', worktreePerAgent: false };
    fakeBackend({
      closed_projects: () => [
        { ...closed, agents: 23, tickets: 0, lastActivity: 1, closedAt: Date.UTC(2026, 9, 18, 12), size: 10, unreadable: false },
      ],
    });
    render(ProjectRail);
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter un projet' }));
    const items = menu.open!.items.filter((i) => !i.separator);
    expect(items.map((i) => i.label)).toEqual(['Ajouter un projet…', 'legacy-billing', 'Tous les projets fermés…']);
    expect(items[1]).toMatchObject({ swatch: PROJECT_COLORS[6], hint: 'Rouvrir', detail: 'Fermé le 18 octobre 2026' });
    items[1].onClick!();
    expect(app.modal).toEqual({ kind: 'newProject', path: 'C:\\code\\legacy', historyId: 'old' });
    items[2].onClick!();
    expect(app.modal).toEqual({ kind: 'settings', tab: 'projects' });
    items[0].onClick!();
    expect(app.modal).toEqual({ kind: 'newProject' });
  });

  it('has the stats and the settings at its bottom, the stats a toggle back to the project', async () => {
    fakeBackend();
    render(ProjectRail);
    const stats = screen.getByRole('button', { name: 'Stats' });
    await userEvent.click(stats);
    expect(app.ui.view).toBe('stats');
    expect(stats).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(stats);
    expect(app.ui.view).toBe('project');
    await userEvent.click(screen.getByRole('button', { name: 'Réglages (Ctrl+,)' }));
    expect(app.modal).toEqual({ kind: 'settings' });
  });

  it('keeps « + » and the settings alone at the first start (choice 49)', () => {
    resetApp({ projects: [] });
    fakeBackend();
    render(ProjectRail);
    const rail = screen.getByRole('navigation', { name: 'Projets' });
    expect(
      within(rail)
        .getAllByRole('button')
        .map((b) => b.getAttribute('aria-label')),
    ).toEqual(['Ajouter un projet', 'Réglages (Ctrl+,)']);
    expect(screen.queryByRole('tablist')).not.toBeInTheDocument();
  });

  it('draws icons, no character, on the tiles of what no project owns', () => {
    fakeBackend();
    render(ProjectRail);
    const rail = screen.getByRole('navigation', { name: 'Projets' });
    const icons = [...rail.querySelectorAll(':scope > button [data-icon]')].map((i) => i.getAttribute('data-icon'));
    expect(icons).toEqual(['overview', 'plus', 'stats', 'puzzle', 'sliders']);
  });
});

describe('ProjectRail with more projects than it shows', () => {
  beforeEach(() => {
    resetApp({
      projects: Array.from({ length: 12 }, (_, i) => project({ id: `p${i + 1}`, name: `projet-${i + 1}` })),
      agents: [agent({ id: 'w', projectId: 'p12', status: 'waiting' })],
    });
    menu.close();
  });

  /** The list shows the first `shown` tiles, 44 px apart. */
  function layout(shown: number) {
    const list = screen.getByRole('tablist');
    Object.defineProperty(list, 'clientHeight', { configurable: true, value: shown * 44 });
    tiles().forEach((t, i) => {
      Object.defineProperty(t, 'offsetTop', { configurable: true, value: i * 44 });
      Object.defineProperty(t, 'offsetHeight', { configurable: true, value: 36 });
    });
    return list;
  }

  it('signals a project out of view that waits, below the list, and brings it into view on a click', async () => {
    fakeBackend();
    const seen = vi.fn();
    Element.prototype.scrollIntoView = seen;
    render(ProjectRail);
    const list = layout(8);
    await fireEvent.scroll(list);
    const more = screen.getByRole('button', { name: 'projet-12 : 1 en attente, plus bas' });
    expect(more).toHaveTextContent('1');
    expect(more.querySelector('[data-icon="down"]')).not.toBeNull();
    expect(list).toHaveClass('more-down');
    expect(list).not.toHaveClass('more-up');
    await userEvent.click(more);
    expect(seen.mock.contexts.at(-1)).toBe(tile('projet-12'));
    // Scrolled to the end: in view, nothing to signal.
    list.scrollTop = 4 * 44;
    await fireEvent.scroll(list);
    expect(screen.queryByRole('button', { name: /plus bas/ })).not.toBeInTheDocument();
    expect(list).toHaveClass('more-up');
  });
});

describe('ProjectRail in English', () => {
  beforeEach(() => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'studio-web' })], agents: [agent({ status: 'waiting' })] });
    setLang('en');
    menu.close();
  });

  it('names its tiles, its tooltips and the project’s menu in English', async () => {
    fakeBackend();
    app.git = { p1: gitInfo({ total: 3 }) };
    render(ProjectRail);
    expect(screen.getByRole('navigation', { name: 'Projects' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Overview' })).toHaveAttribute(
      'data-tip',
      'Overview (Ctrl Shift A)\nEvery agent of every project',
    );
    expect(screen.getByRole('button', { name: 'Add a project' })).toHaveAttribute('data-tip', 'Add a project\nOr reopen a closed project');
    expect(screen.getByRole('button', { name: 'Settings (Ctrl+,)' })).toHaveAttribute('data-tip', 'Settings (Ctrl ,)');
    expect(tile('demo-api')).toHaveAttribute('data-tip', 'demo-api (Ctrl 1)\n1 agent waiting for you\nΔ 3 · uncommitted changes');
    expect(tile('studio-web')).toHaveAttribute('data-tip', 'studio-web (Ctrl 2)\nNothing running, nothing changed');
    await fireEvent.contextMenu(tile('demo-api'));
    const labels = menu.open!.items.map((i) => i.label);
    expect(labels).toEqual(
      expect.arrayContaining(['Rename…', 'Color', 'Project settings…', 'Open the folder', 'Move up', 'Move down', 'Close the project…']),
    );
    expect(labels).toContain('Turn on a worktree per agent');
    menu.open!.items.find((i) => i.label === 'Rename…')!.onClick!();
    expect(app.modal).toMatchObject({ kind: 'rename', title: 'Rename the project' });
  });
});
