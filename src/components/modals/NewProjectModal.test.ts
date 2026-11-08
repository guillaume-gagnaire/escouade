import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { PROJECT_COLORS } from '../../lib/theme';
import type { ClosedProject, FolderInfo } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import NewProjectModal from './NewProjectModal.svelte';

const folder = (over: Partial<FolderInfo> = {}): FolderInfo => ({
  exists: true,
  isRepo: true,
  branch: 'main',
  dirty: 0,
  name: 'demo',
  history: null,
  ...over,
});

/** A project closed with its history kept, in the folder `C:\code\demo`. */
const closed = (over: Partial<ClosedProject> = {}): ClosedProject => ({
  id: 'p7',
  name: 'ancien',
  color: PROJECT_COLORS[3],
  path: 'C:\\code\\demo',
  worktreePerAgent: true,
  agents: 12,
  tickets: 34,
  lastActivity: new Date(2026, 9, 14, 18, 0).getTime(),
  closedAt: new Date(2026, 9, 15, 9, 0).getTime(),
  size: 3 * 1024 * 1024,
  unreadable: false,
  ...over,
});

/** « Créer sans l'assistant »: the project made as before the assistant, with Escouade's defaults. */
const WITHOUT = /^Créer sans l.assistant$/;

/** Types a path and waits for the folder to be inspected (the modal waits a moment before it asks). */
async function typePath(path: string) {
  await userEvent.type(screen.getByPlaceholderText(/^C:/), path);
}

describe('NewProjectModal', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'newProject' };
  });

  it('is an assistant in steps, starting from the folder, its name, its color and where its settings start from', () => {
    fakeBackend();
    render(NewProjectModal);
    const rail = screen.getByRole('navigation', { name: 'Étapes' });
    expect(
      within(rail)
        .getAllByRole('button')
        .map((b) => b.textContent?.replace(/\s+/g, ' ').trim()),
    ).toEqual(['1 Départ', '2 Worktrees', '3 Lancement', '4 Kanban', '5 Compte et commit', '6 Intégrations facultatif', '7 Récapitulatif']);
    expect(screen.getByText('Étape 1 sur 7')).toBeInTheDocument();
    // The project copied comes first, when there is one to copy; the recommended setup is chosen.
    expect(screen.getAllByRole('radio').map((r) => r.querySelector('b')?.textContent)).toEqual([
      'Copier la configuration d’un projet',
      'Configuration recommandée',
      'Partir de zéro',
    ]);
    expect(screen.getByRole('radio', { name: /^Configuration recommandée/ })).toHaveAttribute('aria-checked', 'true');
    // Nothing goes on before the folder is read.
    expect(screen.getByRole('button', { name: 'Suivant' })).toBeDisabled();
    expect(screen.getByRole('button', { name: WITHOUT })).toBeDisabled();
  });

  it('asks for the folder first, then says what it holds', async () => {
    fakeBackend({ inspect_folder: () => folder({ dirty: 3 }) });
    render(NewProjectModal);
    expect(screen.getByRole('dialog', { name: 'Nouveau projet' })).toBeInTheDocument();
    expect(screen.getByText('Choisis le dossier du projet')).toBeInTheDocument();
    await typePath('C:\\code\\demo');
    await waitFor(() => expect(screen.getByText('Dépôt git détecté · branche main · 3 fichiers modifiés')).toBeInTheDocument());
    expect(screen.getByRole('button', { name: WITHOUT })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled();
  });

  it('says a clean repository, a single change, a folder that is not one and a folder that is missing', async () => {
    const answers = [folder(), folder({ dirty: 1 }), folder({ isRepo: false }), folder({ exists: false })];
    let n = 0;
    fakeBackend({ inspect_folder: () => answers[n++] });
    render(NewProjectModal);
    const said = [
      'Dépôt git détecté · branche main · propre',
      'Dépôt git détecté · branche main · 1 fichier modifié',
      'Aucun dépôt git · un dépôt sera initialisé',
      'Dossier introuvable',
    ];
    for (const [i, text] of said.entries()) {
      await typePath(i ? 'x' : 'C:\\code\\demo');
      await waitFor(() => expect(screen.getByText(text)).toBeInTheDocument());
    }
  });
});

describe('NewProjectModal on the folder of a closed project', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    app.modal = { kind: 'newProject' };
  });

  const created = (restore: boolean) => project({ id: restore ? 'p7' : 'p8', name: 'ancien', path: 'C:\\code\\demo' });
  const switchNamed = (name: string) => screen.getByRole('switch', { name });

  it('says what the history holds: its agents, its tickets and its last activity', async () => {
    fakeBackend({ inspect_folder: () => folder({ history: closed() }) });
    render(NewProjectModal);
    expect(screen.queryByText(/^Historique retrouvé/)).not.toBeInTheDocument();
    await typePath('C:\\code\\demo');
    expect(
      await screen.findByText('Historique retrouvé : 12 agents, 34 tickets, dernière activité le 14 octobre 2026'),
    ).toBeInTheDocument();
    expect(screen.getByText("L'historique reste dans Réglages › Tous les projets, où l'effacer.")).toBeInTheDocument();
  });

  it('writes a plural of agents and of tickets for each count', async () => {
    fakeBackend({ inspect_folder: () => folder({ history: closed({ agents: 1, tickets: 0 }) }) });
    render(NewProjectModal);
    await typePath('C:\\code\\demo');
    expect(await screen.findByText(/^Historique retrouvé : 1 agent, 0 ticket, /)).toBeInTheDocument();
  });

  it('starts from the history: its name, its color and its worktrees, no first agent, « Rouvrir le projet »', async () => {
    const backend = fakeBackend({ inspect_folder: () => folder({ history: closed() }), create_project: () => created(true) });
    render(NewProjectModal);
    await typePath('C:\\code\\demo');
    await screen.findByText(/^Historique retrouvé/);
    expect(screen.getByDisplayValue('ancien')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Couleur 4' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.queryByRole('switch', { name: 'Créer un premier agent' })).not.toBeInTheDocument();
    // Its settings come back with it: no step to go through.
    expect(screen.queryByRole('button', { name: 'Suivant' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Worktrees/ })).toBeDisabled();
    expect(screen.queryByRole('button', { name: 'Sonnet' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: WITHOUT })).not.toBeInTheDocument();
    expect(switchNamed("Partir d'un projet vierge")).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(screen.getByRole('button', { name: 'Rouvrir le projet' }));
    expect(backend.called('create_project')[0].args).toEqual({
      path: 'C:\\code\\demo',
      name: 'ancien',
      color: PROJECT_COLORS[3],
      worktreePerAgent: true,
      firstAgent: null,
      restore: true,
    });
    expect(app.projects.map((p) => p.id)).toEqual(['p1', 'p7']);
    expect(app.ui.activeProject).toBe('p7');
    expect(app.modal).toBeNull();
  });

  it('keeps what the user typed over what the history says', async () => {
    const backend = fakeBackend({ inspect_folder: () => folder({ history: closed() }), create_project: () => created(true) });
    render(NewProjectModal);
    await typePath('C:\\code\\demo');
    await screen.findByText(/^Historique retrouvé/);
    const name = screen.getByDisplayValue('ancien');
    await userEvent.clear(name);
    await userEvent.type(name, 'renommé');
    await userEvent.click(screen.getByRole('button', { name: 'Couleur 2' }));
    await userEvent.click(switchNamed("Partir d'un projet vierge"));
    await userEvent.click(switchNamed("Partir d'un projet vierge"));
    await userEvent.click(screen.getByRole('button', { name: 'Rouvrir le projet' }));
    expect(backend.called('create_project')[0].args).toMatchObject({
      name: 'renommé',
      color: PROJECT_COLORS[1],
      worktreePerAgent: true,
      restore: true,
    });
  });

  it('goes back to a window like any other with « Partir d’un projet vierge », and asks not to restore', async () => {
    const backend = fakeBackend({ inspect_folder: () => folder({ history: closed() }), create_project: () => created(false) });
    render(NewProjectModal);
    await typePath('C:\\code\\demo');
    await screen.findByText(/^Historique retrouvé/);
    await userEvent.click(switchNamed("Partir d'un projet vierge"));
    expect(switchNamed("Partir d'un projet vierge")).toHaveAttribute('aria-checked', 'true');
    // The folder's own name, the first color not used, the steps: as without a history.
    expect(screen.getByDisplayValue('demo')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Couleur 2' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled();
    expect(screen.queryByRole('button', { name: 'Rouvrir le projet' })).not.toBeInTheDocument();
    // The history is still said, so that the switch can be turned off again.
    expect(screen.getByText(/^Historique retrouvé/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: WITHOUT }));
    expect(backend.called('create_project')[0].args).toEqual({
      path: 'C:\\code\\demo',
      name: 'demo',
      color: PROJECT_COLORS[1],
      worktreePerAgent: false,
      firstAgent: 'sonnet',
      restore: false,
    });
  });

  it('comes back to the history when the switch is turned off again', async () => {
    fakeBackend({ inspect_folder: () => folder({ history: closed() }) });
    render(NewProjectModal);
    await typePath('C:\\code\\demo');
    await screen.findByText(/^Historique retrouvé/);
    await userEvent.click(switchNamed("Partir d'un projet vierge"));
    await userEvent.click(switchNamed("Partir d'un projet vierge"));
    expect(screen.getByDisplayValue('ancien')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Couleur 4' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: 'Rouvrir le projet' })).toBeInTheDocument();
  });

  it('shows no history, no switch and restores nothing on a folder that was never a project', async () => {
    const backend = fakeBackend({ inspect_folder: () => folder(), create_project: () => created(false) });
    render(NewProjectModal);
    await typePath('C:\\code\\demo');
    await screen.findByText('Dépôt git détecté · branche main · propre');
    expect(screen.queryByText(/^Historique retrouvé/)).not.toBeInTheDocument();
    expect(screen.queryByRole('switch', { name: "Partir d'un projet vierge" })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: WITHOUT })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: WITHOUT }));
    expect(backend.called('create_project')[0].args).toMatchObject({ name: 'demo', firstAgent: 'sonnet', restore: false });
  });

  it('follows the folder: the history of the next one, or none, and the switch starts off again', async () => {
    const answers = [
      folder({ history: closed() }),
      folder({ name: 'autre' }),
      folder({ name: 'troisieme', history: closed({ id: 'p9', name: 'neuf' }) }),
    ];
    let n = 0;
    fakeBackend({ inspect_folder: () => answers[n++] });
    render(NewProjectModal);
    await typePath('C:\\code\\demo');
    await screen.findByText(/^Historique retrouvé/);
    await userEvent.click(switchNamed("Partir d'un projet vierge"));
    await typePath('2');
    await waitFor(() => expect(screen.queryByText(/^Historique retrouvé/)).not.toBeInTheDocument());
    expect(screen.getByDisplayValue('autre')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: WITHOUT })).toBeInTheDocument();
    await typePath('3');
    await screen.findByText(/^Historique retrouvé/);
    expect(screen.getByDisplayValue('neuf')).toBeInTheDocument();
    expect(switchNamed("Partir d'un projet vierge")).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByRole('button', { name: 'Rouvrir le projet' })).toBeInTheDocument();
  });

  it('waits for the folder typed to be read before it creates or reopens anything', async () => {
    let answer: ((f: FolderInfo) => void) | undefined;
    const backend = fakeBackend({
      inspect_folder: ({ path }) => (path.endsWith('2') ? new Promise((r) => (answer = r)) : folder({ history: closed() })),
      create_project: () => created(false),
    });
    render(NewProjectModal);
    await typePath('C:\\code\\demo');
    await screen.findByText(/^Historique retrouvé/);
    await typePath('2');
    // The former folder's history is still on the screen: nothing is sent for the new folder with it.
    expect(screen.getByRole('button', { name: 'Rouvrir le projet' })).toBeDisabled();
    await waitFor(() => expect(answer).toBeDefined());
    answer!(folder({ name: 'demo2' }));
    const create = await screen.findByRole('button', { name: WITHOUT });
    await waitFor(() => expect(create).toBeEnabled());
    await userEvent.click(create);
    expect(backend.called('create_project')[0].args).toMatchObject({ path: 'C:\\code\\demo2', firstAgent: 'sonnet', restore: false });
  });

  it('says that it is reopening while the project comes back', async () => {
    let back!: (p: unknown) => void;
    fakeBackend({ inspect_folder: () => folder({ history: closed() }), create_project: () => new Promise((r) => (back = r)) });
    render(NewProjectModal);
    await typePath('C:\\code\\demo');
    await screen.findByText(/^Historique retrouvé/);
    await userEvent.click(screen.getByRole('button', { name: 'Rouvrir le projet' }));
    expect(screen.getByRole('button', { name: 'Réouverture…' })).toBeDisabled();
    back(created(true));
    await waitFor(() => expect(app.modal).toBeNull());
  });
});

describe('NewProjectModal opened by « Rouvrir » on a closed project', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    // The folder has two histories: p7 is the one asked for.
    app.modal = { kind: 'newProject', path: 'C:\\code\\demo', historyId: 'p7' };
  });

  const created = project({ id: 'p7', name: 'ancien', path: 'C:\\code\\demo' });

  it('starts from that folder, reads it at once for that history, and says what it holds', async () => {
    const backend = fakeBackend({ inspect_folder: () => folder({ history: closed() }) });
    render(NewProjectModal);
    expect(screen.getByPlaceholderText(/^C:/)).toHaveValue('C:\\code\\demo');
    // No waiting for a key to be typed: the folder is read as the window opens.
    await tick();
    expect(backend.called('inspect_folder')).toHaveLength(1);
    expect(backend.called('inspect_folder')[0].args).toEqual({ path: 'C:\\code\\demo', historyId: 'p7' });
    expect(
      await screen.findByText('Historique retrouvé : 12 agents, 34 tickets, dernière activité le 14 octobre 2026'),
    ).toBeInTheDocument();
    expect(screen.getByDisplayValue('ancien')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Rouvrir le projet' })).toBeEnabled();
    // Read once, not again by the effect of the path it was given.
    await new Promise((r) => setTimeout(r, 300));
    expect(backend.called('inspect_folder')).toHaveLength(1);
  });

  it('reopens that history, and names it to the backend', async () => {
    const backend = fakeBackend({ inspect_folder: () => folder({ history: closed() }), create_project: () => created });
    render(NewProjectModal);
    await userEvent.click(await screen.findByRole('button', { name: 'Rouvrir le projet' }));
    expect(backend.called('create_project')[0].args).toEqual({
      path: 'C:\\code\\demo',
      name: 'ancien',
      color: PROJECT_COLORS[3],
      worktreePerAgent: true,
      firstAgent: null,
      restore: true,
      historyId: 'p7',
    });
    expect(app.projects.map((p) => p.id)).toEqual(['p1', 'p7']);
    expect(app.ui.activeProject).toBe('p7');
    expect(app.modal).toBeNull();
  });

  it('lets go of that history as soon as the path is not the one it started with, and takes it back with the path', async () => {
    const backend = fakeBackend({
      inspect_folder: ({ historyId }) => folder({ history: historyId ? closed() : null }),
      create_project: () => created,
    });
    render(NewProjectModal);
    await screen.findByText(/^Historique retrouvé/);
    await typePath('2');
    await waitFor(() => expect(backend.called('inspect_folder')).toHaveLength(2));
    expect(backend.called('inspect_folder')[1].args).toEqual({ path: 'C:\\code\\demo2' });
    expect(Object.keys(backend.called('inspect_folder')[1].args)).toEqual(['path']);
    // Another folder, which has no history: a project like any other, sent without a history.
    const create = await screen.findByRole('button', { name: WITHOUT });
    await waitFor(() => expect(create).toBeEnabled());
    await userEvent.click(create);
    expect(backend.called('create_project')[0].args).toMatchObject({ path: 'C:\\code\\demo2', restore: false });
    expect(Object.keys(backend.called('create_project')[0].args)).not.toContain('historyId');
  });

  it('asks again for that history when the path typed is the one it started with once more', async () => {
    const backend = fakeBackend({ inspect_folder: () => folder({ history: closed() }) });
    render(NewProjectModal);
    await screen.findByText(/^Historique retrouvé/);
    await typePath('2');
    await waitFor(() => expect(backend.called('inspect_folder')).toHaveLength(2));
    await userEvent.type(screen.getByPlaceholderText(/^C:/), '{Backspace}');
    await waitFor(() => expect(backend.called('inspect_folder')).toHaveLength(3));
    expect(backend.called('inspect_folder')[2].args).toEqual({ path: 'C:\\code\\demo', historyId: 'p7' });
  });

  it('still names the history when the project starts blank: the backend ignores it then', async () => {
    const backend = fakeBackend({ inspect_folder: () => folder({ history: closed() }), create_project: () => created });
    render(NewProjectModal);
    await screen.findByText(/^Historique retrouvé/);
    await userEvent.click(screen.getByRole('switch', { name: "Partir d'un projet vierge" }));
    await userEvent.click(screen.getByRole('button', { name: WITHOUT }));
    expect(backend.called('create_project')[0].args).toMatchObject({ restore: false, historyId: 'p7' });
  });

  it('is a window like any other for a folder with no history left, such as one erased meanwhile', async () => {
    fakeBackend({ inspect_folder: () => folder() });
    render(NewProjectModal);
    expect(await screen.findByText('Dépôt git détecté · branche main · propre')).toBeInTheDocument();
    expect(screen.queryByText(/^Historique retrouvé/)).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: WITHOUT })).toBeEnabled();
  });

  it('closes on « Annuler » without asking anything more', async () => {
    const backend = fakeBackend({ inspect_folder: () => folder({ history: closed() }) });
    render(NewProjectModal);
    await screen.findByText(/^Historique retrouvé/);
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(app.modal).toBeNull();
    expect(backend.called('create_project')).toHaveLength(0);
  });
});

describe('NewProjectModal in English', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'newProject' };
    setLang('en');
  });

  it('writes the labels, the switches and the buttons in English', () => {
    fakeBackend();
    render(NewProjectModal);
    expect(screen.getByRole('dialog', { name: 'New project' })).toBeInTheDocument();
    expect(screen.getByText('Folder')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Browse…' })).toBeInTheDocument();
    expect(screen.getByText('Choose the project folder')).toBeInTheDocument();
    expect(screen.getByText('Color')).toBeInTheDocument();
    expect(
      screen.getByText('Where the settings of new-project come from: each next step can be reviewed, and skipped.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Color 1' })).toBeInTheDocument();
    expect(screen.getByRole('navigation', { name: 'Steps' })).toHaveTextContent(
      /Start.*Worktrees.*Launch.*Kanban.*Account and commits.*Integrations.*optional.*Summary/,
    );
    expect(screen.getByText('Step 1 of 7')).toBeInTheDocument();
    expect(screen.getByRole('radiogroup', { name: 'Starting point' })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /^Copy a project’s setup/ })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /^Recommended setup/ })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('radio', { name: /^Start from scratch/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Create without the assistant' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Next' })).toBeDisabled();
    expect(screen.getByPlaceholderText('C:\\path\\to\\the\\project')).toBeInTheDocument();
  });

  it('says what the folder holds, with a plural', async () => {
    fakeBackend({ inspect_folder: () => folder({ dirty: 3 }) });
    render(NewProjectModal);
    await typePath('C:\\code\\demo');
    await waitFor(() => expect(screen.getByText('Git repository detected · branch main · 3 files changed')).toBeInTheDocument());
  });

  it('says in English that the history of the folder is found, and what reopening does', async () => {
    let back!: (p: unknown) => void;
    fakeBackend({ inspect_folder: () => folder({ history: closed({ agents: 1 }) }), create_project: () => new Promise((r) => (back = r)) });
    render(NewProjectModal);
    await typePath('C:\\code\\demo');
    expect(await screen.findByText('History found: 1 agent, 34 tickets, last activity on October 14, 2026')).toBeInTheDocument();
    expect(screen.getByText('The history stays in Settings › All projects, where you can erase it.')).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'Start from a blank project' })).toHaveAttribute('aria-checked', 'false');
    expect(screen.queryByRole('switch', { name: 'Create a first agent' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Reopen project' }));
    expect(screen.getByRole('button', { name: 'Reopening…' })).toBeDisabled();
    back(project({ id: 'p7' }));
    await waitFor(() => expect(app.modal).toBeNull());
  });
});
