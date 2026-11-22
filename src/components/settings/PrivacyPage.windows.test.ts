import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../../lib/state.svelte';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));
vi.mock('../../lib/platform', async (original) => ({ ...(await original<object>()), IS_WINDOWS: true }));

// SEC-08: under Windows, « Projet privé » says what its fence is: an instruction to the harness, not a partition.
describe('PrivacyPage under Windows', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    app.modal = { kind: 'settings' };
    fakeBackend();
  });

  it('says the fence is an instruction to the harness', () => {
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    const desc = screen.getByText(/^Les agents des autres projets ne le voient pas/);
    expect(desc).toHaveTextContent(
      'Sous Windows, la clôture d’un agent est une consigne donnée à son harness, pas un cloisonnement du système',
    );
    expect(desc).not.toHaveTextContent('activé par toi');
  });
  // SEC-02 (ESC-590): a private project says a third party's harness is not launched there.
  it('says, for a private project, that a third party’s harness can’t guarantee its fence', () => {
    resetApp({ projects: [project({ private: true })] });
    app.modal = { kind: 'settings' };
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    expect(screen.getByTestId('private-third-party')).toHaveTextContent(
      'Un harness d’un plugin tiers ne peut pas garantir la clôture d’un projet privé : il n’y est pas lancé, et un agent qui en a un s’arrête.',
    );
  });

  it('says nothing of it for a project that is not private', () => {
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    expect(screen.queryByTestId('private-third-party')).toBeNull();
  });
});
