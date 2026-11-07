import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../../lib/state.svelte';
import type { VectorsModelInfo } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

// The model of the search by meaning (ESC-254), for every project of this machine (« Tous les projets », ESC-310):
// downloaded with its progress, called off, deleted.

const MODEL: VectorsModelInfo = {
  id: 'multilingual-e5-base',
  license: 'MIT',
  size: 295_266_822,
  have: 0,
  downloaded: false,
  downloading: false,
};

const group = () => screen.getByText('Tous les projets', { selector: 'h2' }).closest('section') as HTMLElement;

describe('the search by meaning in the projects’ settings', () => {
  beforeEach(() => {
    app.modal = { kind: 'settings' };
    app.vectorsProgress = null;
  });

  it('downloads the model of the search by meaning with its progress, and calls it off', async () => {
    resetApp({ projects: [project()] });
    let finish: (v: VectorsModelInfo) => void = () => {};
    const be = fakeBackend({
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

  it('deletes a model downloaded, with no project open too', async () => {
    resetApp({ projects: [] });
    const be = fakeBackend({
      vectors_model: () => ({ ...MODEL, downloaded: true, have: MODEL.size }),
      vectors_model_delete: () => MODEL,
    });
    render(SettingsModal, { tab: 'projects' });
    await screen.findByText('Téléchargé');
    await userEvent.click(screen.getByRole('button', { name: 'Effacer' }));
    expect(be.called('vectors_model_delete')).toHaveLength(1);
    expect(await screen.findByRole('button', { name: /Télécharger/ })).toBeInTheDocument();
  });
});
