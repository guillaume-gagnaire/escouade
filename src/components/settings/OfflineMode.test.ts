import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import { afterGesture } from '../../test/settings';
import SettingsModal from '../modals/SettingsModal.svelte';
import StatusBar from '../StatusBar.svelte';

// « Mode hors ligne » (audit SEC-18, ESC-596): a switch of Réglages › Avancé › Réseau, saved at the gesture, which the
// status bar says while it lasts; locked on, and said so, when a policy of this device imposes it.

beforeEach(() => {
  setLang('fr');
  resetApp({});
  app.offline = { on: false, imposed: false };
});

describe('Mode hors ligne', () => {
  it('is a switch of Réseau, saved at the gesture, and the status bar says it once on', async () => {
    let on = false;
    const be = fakeBackend({
      save_settings: (a: { settings: { offline?: boolean } }) => {
        on = !!a.settings.offline;
        return [];
      },
      offline_state: () => ({ on, imposed: false }),
    });
    render(SettingsModal, { page: 'advanced' });
    const sw = screen.getByRole('switch', { name: 'Mode hors ligne' });
    expect(sw).toHaveAttribute('aria-checked', 'false');
    expect(sw).not.toBeDisabled();
    expect(screen.getByText(/ni les mises à jour, ni le store, ni la liste de révocation, ni les statistiques/)).toBeTruthy();
    await userEvent.click(sw);
    await afterGesture();
    expect(be.called('save_settings').at(-1)!.args.settings).toMatchObject({ offline: true });
    await expect.poll(() => app.offline.on).toBe(true);
    render(StatusBar);
    expect(screen.getByTestId('status-offline')).toHaveTextContent('Hors ligne');
  });

  it('is locked on, and says by whom, when a policy of this device imposes it', async () => {
    fakeBackend({ offline_state: () => ({ on: true, imposed: true }) });
    app.offline = { on: true, imposed: true };
    render(SettingsModal, { page: 'advanced' });
    const sw = screen.getByRole('switch', { name: 'Mode hors ligne' });
    expect(sw).toHaveAttribute('aria-checked', 'true');
    expect(sw).toBeDisabled();
    const group = sw.closest('.sr') as HTMLElement;
    expect(within(group).getByText(/Imposé par la stratégie de ce poste\./)).toBeTruthy();
  });

  it('says nothing in the status bar while it is off', () => {
    fakeBackend();
    render(StatusBar);
    expect(screen.queryByTestId('status-offline')).toBeNull();
  });
});
