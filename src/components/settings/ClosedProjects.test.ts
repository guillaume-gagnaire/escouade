import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { settingsForm } from '../../lib/settings.svelte';
import { app } from '../../lib/state.svelte';
import type { ClosedProject } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import ClosedProjects from './ClosedProjects.svelte';

const MB = 1024 * 1024;

const closed = (over: Partial<ClosedProject> = {}): ClosedProject => ({
  id: 'c1',
  name: 'ancien',
  color: 'oklch(0.72 0.12 300)',
  path: 'C:\\code\\ancien',
  worktreePerAgent: false,
  agents: 3,
  tickets: 5,
  lastActivity: new Date(2026, 9, 14, 18, 0).getTime(),
  closedAt: new Date(2026, 9, 15, 9, 0).getTime(),
  size: 3.26 * MB,
  unreadable: false,
  ...over,
});

const OLD = closed();
const OTHER = closed({
  id: 'c2',
  name: 'vieux',
  path: 'C:\\code\\vieux',
  size: 12 * 1024,
  lastActivity: new Date(2026, 8, 2, 10, 0).getTime(),
});
/** A file of `closed-projects/` the app cannot read: its name without `.json` is all it has. */
const BROKEN = closed({
  id: 'abc123',
  name: '',
  color: '',
  path: '',
  agents: 0,
  tickets: 0,
  size: 2 * 1024,
  unreadable: true,
  lastActivity: new Date(2026, 9, 10, 8, 0).getTime(),
  closedAt: new Date(2026, 9, 10, 8, 0).getTime(),
});

describe('ClosedProjects', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    settingsForm.open({ page: 'allProjects', projectId: 'p1' });
  });

  it('shows nothing when no project was closed', async () => {
    const backend = fakeBackend({ closed_projects: () => [] });
    render(ClosedProjects);
    await waitFor(() => expect(backend.called('closed_projects')).toHaveLength(1));
    await new Promise((r) => setTimeout(r));
    expect(document.body.textContent).toBe('');
  });

  it('shows nothing, and no error, when the list cannot be read', async () => {
    const backend = fakeBackend({
      closed_projects: () => {
        throw new Error('disque illisible');
      },
    });
    render(ClosedProjects);
    await waitFor(() => expect(backend.called('closed_projects')).toHaveLength(1));
    await new Promise((r) => setTimeout(r));
    expect(document.body.textContent).toBe('');
    expect(app.toasts).toEqual([]);
  });

  it('lists each closed project with its folder, its size on the disk and its last activity', async () => {
    fakeBackend({ closed_projects: () => [OLD, OTHER] });
    render(ClosedProjects);
    expect(await screen.findByRole('heading', { name: 'Projets fermés' })).toBeInTheDocument();
    expect(screen.getByText('Leur historique est gardé : ajouter un projet dans leur dossier le retrouve.')).toBeInTheDocument();
    const rows = screen.getAllByRole('listitem');
    expect(rows).toHaveLength(2);
    expect(within(rows[0]).getByText('ancien')).toBeInTheDocument();
    expect(within(rows[0]).getByText('C:\\code\\ancien')).toHaveClass('mono');
    expect(within(rows[0]).getByText('3,3 Mo · dernière activité le 14 octobre 2026')).toBeInTheDocument();
    expect(within(rows[0]).getByRole('button', { name: "Effacer l'historique…" })).toBeInTheDocument();
    expect(within(rows[1]).getByText('vieux')).toBeInTheDocument();
    expect(within(rows[1]).getByText('12 Ko · dernière activité le 2 septembre 2026')).toBeInTheDocument();
  });

  it('tells which project a button erases the history of', async () => {
    fakeBackend({ closed_projects: () => [OLD, OTHER] });
    render(ClosedProjects);
    const buttons = await screen.findAllByRole('button', { name: "Effacer l'historique…" });
    expect(buttons[0]).toHaveAccessibleDescription('ancien');
    expect(buttons[1]).toHaveAccessibleDescription('vieux');
  });

  it('asks before erasing the history, and erases nothing until it is confirmed', async () => {
    const backend = fakeBackend({ closed_projects: () => [OLD, OTHER] });
    render(ClosedProjects);
    const rows = await screen.findAllByRole('listitem');
    await userEvent.click(within(rows[1]).getByRole('button', { name: "Effacer l'historique…" }));
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: "Effacer l'historique de « vieux » ?",
      body: 'Ses conversations, ses tickets et ses réglages sont effacés pour de bon. Les fichiers et les worktrees sur le disque ne sont pas touchés.',
      confirm: "Effacer l'historique",
      danger: true,
    });
    expect(backend.called('erase_closed_project')).toHaveLength(0);
  });

  it('erases the history once confirmed, and comes back to « Tous les projets », the project that was on screen kept', async () => {
    const backend = fakeBackend({ closed_projects: () => [OLD], erase_closed_project: () => undefined });
    render(ClosedProjects);
    await userEvent.click(await screen.findByRole('button', { name: "Effacer l'historique…" }));
    await (app.modal as any).onConfirm(false);
    expect(backend.called('erase_closed_project')[0].args).toEqual({ id: 'c1' });
    expect(app.modal).toEqual({ kind: 'settings', page: 'allProjects', projectId: 'p1', resume: true });
  });

  it('comes back to the settings when the question is cancelled, and when the erasing failed', async () => {
    const backend = fakeBackend({
      closed_projects: () => [OLD],
      erase_closed_project: () => {
        throw new Error('fichier verrouillé');
      },
    });
    render(ClosedProjects);
    await userEvent.click(await screen.findByRole('button', { name: "Effacer l'historique…" }));
    (app.modal as any).onCancel();
    expect(app.modal).toEqual({ kind: 'settings', page: 'allProjects', projectId: 'p1', resume: true });
    await userEvent.click(screen.getByRole('button', { name: "Effacer l'historique…" }));
    await (app.modal as any).onConfirm(false);
    expect(backend.called('erase_closed_project')).toHaveLength(1);
    expect(app.toasts.at(-1)?.text).toMatch(/fichier verrouillé/);
    expect(app.modal).toEqual({ kind: 'settings', page: 'allProjects', projectId: 'p1', resume: true });
  });

  it('comes back to the settings without a project when none is open', async () => {
    resetApp({ projects: [] });
    settingsForm.open({ page: 'allProjects' });
    fakeBackend({ closed_projects: () => [OLD], erase_closed_project: () => undefined });
    render(ClosedProjects);
    await userEvent.click(await screen.findByRole('button', { name: "Effacer l'historique…" }));
    await (app.modal as any).onConfirm(false);
    expect(app.modal).toEqual({ kind: 'settings', page: 'allProjects', projectId: undefined, resume: true });
  });

  it('reopens a project from its row: the window that adds a project, on its folder and on that history', async () => {
    const backend = fakeBackend({ closed_projects: () => [OLD, OTHER] });
    render(ClosedProjects);
    const rows = await screen.findAllByRole('listitem');
    const buttons = screen.getAllByRole('button', { name: 'Rouvrir' });
    expect(buttons).toHaveLength(2);
    expect(buttons[0]).toHaveAccessibleDescription('ancien');
    expect(buttons[1]).toHaveAccessibleDescription('vieux');
    // The second one: not the latest closed, nor the first of its folder, but this history.
    await userEvent.click(within(rows[1]).getByRole('button', { name: 'Rouvrir' }));
    expect(app.modal).toEqual({ kind: 'newProject', path: 'C:\\code\\vieux', historyId: 'c2' });
    expect(backend.called('erase_closed_project')).toHaveLength(0);
  });

  describe('an entry the app cannot read', () => {
    it('is told apart: its file, its size, nothing to reopen, and a button to erase it', async () => {
      fakeBackend({ closed_projects: () => [OLD, BROKEN] });
      render(ClosedProjects);
      const rows = await screen.findAllByRole('listitem');
      expect(rows).toHaveLength(2);
      expect(within(rows[1]).getByText('Historique illisible')).toBeInTheDocument();
      expect(within(rows[1]).getByText('abc123.json')).toHaveClass('mono');
      expect(within(rows[1]).getByText('2 Ko')).toBeInTheDocument();
      expect(within(rows[1]).queryByRole('button', { name: 'Rouvrir' })).not.toBeInTheDocument();
      expect(within(rows[1]).queryByRole('button', { name: "Effacer l'historique…" })).not.toBeInTheDocument();
      expect(within(rows[1]).getByRole('button', { name: 'Effacer' })).toHaveAccessibleDescription('abc123.json');
      // The readable one is as before.
      expect(within(rows[0]).getByRole('button', { name: 'Rouvrir' })).toBeInTheDocument();
      expect(within(rows[0]).queryByRole('button', { name: 'Effacer' })).not.toBeInTheDocument();
    });

    it('is erased on its own once confirmed, after a question that says only the file goes', async () => {
      const backend = fakeBackend({ closed_projects: () => [BROKEN], erase_closed_project: () => undefined });
      render(ClosedProjects);
      await userEvent.click(await screen.findByRole('button', { name: 'Effacer' }));
      expect(app.modal).toMatchObject({
        kind: 'confirm',
        title: 'Effacer le fichier « abc123.json » ?',
        body: "Ce fichier, que l'application ne peut pas lire, est effacé pour de bon. Les autres historiques, les fichiers et les worktrees sur le disque ne sont pas touchés.",
        confirm: 'Effacer le fichier',
        danger: true,
      });
      expect(backend.called('erase_closed_project')).toHaveLength(0);
      await (app.modal as any).onConfirm(false);
      expect(backend.called('erase_closed_project')[0].args).toEqual({ id: 'abc123' });
      expect(app.modal).toEqual({ kind: 'settings', page: 'allProjects', projectId: 'p1', resume: true });
    });

    it('comes back to the settings when the question is cancelled', async () => {
      const backend = fakeBackend({ closed_projects: () => [BROKEN] });
      render(ClosedProjects);
      await userEvent.click(await screen.findByRole('button', { name: 'Effacer' }));
      (app.modal as any).onCancel();
      expect(app.modal).toEqual({ kind: 'settings', page: 'allProjects', projectId: 'p1', resume: true });
      expect(backend.called('erase_closed_project')).toHaveLength(0);
    });

    it('is the only thing listed when it is all there is', async () => {
      fakeBackend({ closed_projects: () => [BROKEN] });
      render(ClosedProjects);
      expect(await screen.findByRole('heading', { name: 'Projets fermés' })).toBeInTheDocument();
      expect(screen.getAllByRole('listitem')).toHaveLength(1);
    });
  });

  it('is written in English', async () => {
    setLang('en');
    fakeBackend({ closed_projects: () => [OLD, BROKEN] });
    render(ClosedProjects);
    expect(await screen.findByRole('heading', { name: 'Closed projects' })).toBeInTheDocument();
    expect(screen.getByText('Their history is kept: adding a project in their folder brings it back.')).toBeInTheDocument();
    expect(screen.getByText('3.3 MB · last activity on October 14, 2026')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reopen' })).toBeInTheDocument();
    expect(screen.getByText('Unreadable history')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Erase history…' }));
    expect(app.modal).toMatchObject({
      title: 'Erase the history of “ancien”?',
      body: 'Its conversations, tickets, and settings are erased for good. The files and worktrees on disk are not touched.',
      confirm: 'Erase history',
      danger: true,
    });
    await userEvent.click(screen.getByRole('button', { name: 'Erase' }));
    expect(app.modal).toMatchObject({
      title: 'Erase the file “abc123.json”?',
      body: 'This file, which the app cannot read, is erased for good. The other histories, and the files and worktrees on disk, are not touched.',
      confirm: 'Erase the file',
      danger: true,
    });
  });
});
