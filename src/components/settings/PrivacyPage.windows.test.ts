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

  it('says the fence is an instruction to the harness, and that a third-party plugin runs only once turned on', () => {
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    const desc = screen.getByText(/^Les agents des autres projets ne le voient pas/);
    expect(desc).toHaveTextContent(
      'Sous Windows, la clôture d’un agent est une consigne donnée à son harness, pas un cloisonnement du système',
    );
    expect(desc).toHaveTextContent('Un plugin tiers n’y est lancé qu’une fois activé par toi pour ce projet.');
  });
});
