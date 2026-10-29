import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../../lib/state.svelte';
import type { CodeIndexInfo, VectorsModelInfo } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

// « Index du code » of a project's settings (ESC-254): offered off, what it does, that it stays here, what a private
// project leaves out; and the model of the search by meaning, downloaded with its progress, called off.

const STATUS = { files: 0, symbols: 0, links: 0, worktrees: 0, bytes: 0, indexedAt: 0 };
const off: CodeIndexInfo = { enabled: false, chosen: false, reading: false, waiting: false, status: STATUS, excluded: [] };
const on: CodeIndexInfo = {
  enabled: true,
  chosen: true,
  reading: false,
  waiting: false,
  status: { files: 120, symbols: 2400, links: 3100, worktrees: 0, bytes: 2_097_152, indexedAt: Date.now() },
  excluded: [],
};
const MODEL: VectorsModelInfo = {
  id: 'multilingual-e5-base',
  license: 'MIT',
  size: 295_266_822,
  have: 0,
  downloaded: false,
  downloading: false,
};

const group = () => screen.getByText('Index du code', { selector: 'h2' }).closest('section') as HTMLElement;

describe('the code index in the project’s settings (ESC-254)', () => {
  beforeEach(() => {
    app.modal = { kind: 'settings' };
    app.vectorsProgress = null;
  });

  it('is off by default, says what it does and that nothing leaves the machine, and turns on at once', async () => {
    resetApp({ projects: [project()] });
    const be = fakeBackend({ get_code_index: () => off, set_code_index: () => on, vectors_model: () => MODEL });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await screen.findByText('Index du code', { selector: 'h2' });
    const g = group();
    expect(within(g).getByText(/trouvent le code plus vite et pour moins de tokens/)).toBeInTheDocument();
    expect(within(g).getByText(/Tout reste sur cette machine/)).toBeInTheDocument();
    const sw = within(g).getByRole('switch', { name: 'Indexer ce projet' });
    await waitFor(() => expect(sw).toHaveAttribute('aria-checked', 'false'));
    await userEvent.click(sw);
    expect(be.called('set_code_index')[0].args).toEqual({ id: 'p1', enabled: true });
    // Its own choice now.
    await waitFor(() => expect(within(g).getByText('Choisi pour ce projet.')).toBeInTheDocument());
    await waitFor(() => expect(within(g).getByText(/120 fichiers · 2 400 symboles/)).toBeInTheDocument());
    expect(sw).toHaveAttribute('aria-checked', 'true');
    // Not the draft's: « Enregistrer » is not needed, nothing else saved.
    expect(be.called('update_project')).toHaveLength(0);
  });

  it('says it follows the app’s setting, and where that setting is', async () => {
    resetApp({ projects: [project()] });
    app.settings.codeIndexAll = true;
    fakeBackend({ get_code_index: () => ({ ...on, chosen: false }), vectors_model: () => MODEL });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await screen.findByText('Index du code', { selector: 'h2' });
    const g = group();
    expect(await within(g).findByText('Suit le réglage de l’app (allumé).')).toBeInTheDocument();
    expect(within(g).getByRole('switch', { name: 'Indexer ce projet' })).toHaveAttribute('aria-checked', 'true');
    expect(within(g).queryByRole('button', { name: 'Suivre le réglage de l’app' })).toBeNull();
    // « Voir le réglage » goes to it, further down the same tab.
    const all = screen.getByText('Tous les projets', { selector: 'h2' }).closest('section') as HTMLElement;
    const scrolled = vi.fn();
    all.scrollIntoView = scrolled;
    await userEvent.click(within(g).getByRole('button', { name: 'Voir le réglage' }));
    expect(scrolled).toHaveBeenCalled();
  });

  it('says a choice of the project’s is its own, and goes back to the app’s setting', async () => {
    resetApp({ projects: [project({ codeIndexChoice: false })] });
    app.settings.codeIndexAll = true;
    let info: CodeIndexInfo = { ...off, chosen: true };
    const be = fakeBackend({
      get_code_index: () => info,
      set_code_index: () => (info = { ...on, chosen: false }),
      vectors_model: () => MODEL,
    });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await screen.findByText('Index du code', { selector: 'h2' });
    const g = group();
    expect(await within(g).findByText('Choisi pour ce projet.')).toBeInTheDocument();
    const sw = within(g).getByRole('switch', { name: 'Indexer ce projet' });
    expect(sw).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(within(g).getByRole('button', { name: 'Suivre le réglage de l’app' }));
    expect(be.called('set_code_index')[0].args).toEqual({ id: 'p1', enabled: null });
    await waitFor(() => expect(sw).toHaveAttribute('aria-checked', 'true'));
    expect(within(g).getByText(/Suit le réglage de l’app/)).toBeInTheDocument();
    expect(app.toasts.map((t) => t.text)).toContain('Ce projet suit de nouveau le réglage de l’app.');
  });

  it('says it waits for its turn, then that it is read, as the backend tells', async () => {
    resetApp({ projects: [project()] });
    app.settings.codeIndexAll = true;
    let info: CodeIndexInfo = { ...on, chosen: false, waiting: true, status: STATUS };
    fakeBackend({ get_code_index: () => info, vectors_model: () => MODEL });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await screen.findByText('Index du code', { selector: 'h2' });
    const g = group();
    expect(await within(g).findByText('En attente de lecture, après les autres projets…')).toBeInTheDocument();
    info = { ...info, waiting: false, reading: true };
    app.codeIndexTicks.p1 = 1;
    expect(await within(g).findByText('Lecture du projet…')).toBeInTheDocument();
    info = { ...on, chosen: false };
    app.codeIndexTicks.p1 = 2;
    expect(await within(g).findByText(/120 fichiers · 2 400 symboles/)).toBeInTheDocument();
  });

  it('says what a private project keeps to itself and which of its folders stays out', async () => {
    resetApp({ projects: [project({ private: true })] });
    fakeBackend({ get_code_index: () => ({ ...off, excluded: ['vendor/vault'] }), vectors_model: () => MODEL });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await screen.findByText('Index du code', { selector: 'h2' });
    const g = group();
    expect(await within(g).findByText(/Laissé hors de l’index : vendor\/vault/)).toBeInTheDocument();
    expect(within(g).getByText(/son index ne sert qu’à ses propres agents/)).toBeInTheDocument();
  });

  it('downloads the model of the search by meaning with its progress, and calls it off', async () => {
    resetApp({ projects: [project()] });
    let finish: (v: VectorsModelInfo) => void = () => {};
    const be = fakeBackend({
      get_code_index: () => off,
      vectors_model: () => MODEL,
      vectors_model_download: () => new Promise((r) => (finish = r)),
      vectors_model_cancel: () => null,
    });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const g = await waitFor(() => {
      const s = group();
      within(s).getByText('Recherche par le sens');
      return s;
    });
    expect(within(g).getByText(/multilingual-e5-base, licence MIT/)).toBeInTheDocument();
    await userEvent.click(within(g).getByRole('button', { name: /Télécharger \(282 Mo\)/ }));
    expect(be.called('vectors_model_download')).toHaveLength(1);
    app.vectorsProgress = { done: MODEL.size * 0.4, total: MODEL.size };
    await waitFor(() => expect(within(g).getByRole('progressbar')).toHaveAttribute('aria-valuenow', '40'));
    await userEvent.click(within(g).getByRole('button', { name: 'Annuler' }));
    expect(be.called('vectors_model_cancel')).toHaveLength(1);
    // Called off: what came is kept, offered to resume.
    finish({ ...MODEL, have: Math.round(MODEL.size * 0.4) });
    expect(await within(g).findByRole('button', { name: 'Reprendre (40 %)' })).toBeInTheDocument();
    expect(within(g).queryByRole('progressbar')).toBeNull();
  });

  it('deletes a model downloaded', async () => {
    resetApp({ projects: [project()] });
    const be = fakeBackend({
      get_code_index: () => off,
      vectors_model: () => ({ ...MODEL, downloaded: true, have: MODEL.size }),
      vectors_model_delete: () => MODEL,
    });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await screen.findByText('Téléchargé');
    await userEvent.click(screen.getByRole('button', { name: 'Effacer' }));
    expect(be.called('vectors_model_delete')).toHaveLength(1);
    expect(await screen.findByRole('button', { name: /Télécharger/ })).toBeInTheDocument();
  });
});
