import { render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// The settings' other tabs have logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

import { setLang } from '../../lib/i18n';
import { setLocks } from '../../lib/locks.svelte';
import { app } from '../../lib/state.svelte';
import type { Locks } from '../../lib/types';
import { fakeBackend, resetApp, SETTINGS } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

const DEVICE = { kind: 'device', source: 'HKLM' } as const;
const LOCKS: Locks = {
  locks: {
    'telemetry.enabled': { key: 'telemetry.enabled', value: false, by: DEVICE, bounds: [] },
    'permissions.allowBypass': { key: 'permissions.allowBypass', value: false, by: DEVICE, bounds: [] },
    'permissions.modes': {
      key: 'permissions.modes',
      value: null,
      by: null,
      bounds: [{ by: { kind: 'team', team: 'Acme' }, allowed: ['plan', 'default'] }],
    },
  },
  rows: { telemetry: ['telemetry.enabled'], allowBypass: ['permissions.allowBypass'], agentMode: ['permissions.modes'] },
  warnings: ['policy “some.future” of the device: unknown key, ignored'],
};

const backend = () =>
  fakeBackend({
    mcp_status: () => ({ running: false, port: 0, error: null }),
    mcp_declare_status: () => [],
    mcp_activity: () => [],
  });

/** The row (`.sr`) that holds the switch named `name`. */
const rowOf = (name: string) => screen.getByRole('switch', { name }).closest('.sr') as HTMLElement;

describe('a setting imposed (TS-11), greyed with its padlock and who imposes it', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
    app.settings = { ...SETTINGS, telemetry: false, allowBypass: false, agentMode: 'default' };
    setLocks(LOCKS);
  });

  it('greys « Statistiques d’usage » and says it is imposed by this device’s policy; the warnings are listed', () => {
    backend();
    app.modal = { kind: 'settings', page: 'general' };
    render(SettingsModal, { page: 'general' });
    const row = rowOf('Statistiques d’usage');
    expect(row).toHaveClass('locked');
    expect(within(row).getByText('Imposé par la stratégie de ce poste')).toBeInTheDocument();
    expect(row.querySelector('.ctl')).toHaveAttribute('aria-disabled', 'true');
    // A setting not locked stays as it was.
    expect(rowOf('Installer les mises à jour automatiquement')).not.toHaveClass('locked');
    expect(rowOf('Installer les mises à jour automatiquement').querySelector('.ctl')).not.toHaveAttribute('aria-disabled');
    expect(screen.getByRole('heading', { name: 'Stratégies de ce poste' })).toBeInTheDocument();
    expect(screen.getByText('policy “some.future” of the device: unknown key, ignored')).toBeInTheDocument();
  });

  it('greys « Autoriser le mode Bypass », and the modes a bound denies, with the reason', () => {
    backend();
    app.modal = { kind: 'settings', page: 'agents' };
    render(SettingsModal, { page: 'agents' });
    const bypass = rowOf('Autoriser le mode Bypass');
    expect(bypass).toHaveClass('locked');
    expect(within(bypass).getByText('Imposé par la stratégie de ce poste')).toBeInTheDocument();
    const modes = screen.getByRole('group', { name: 'Mode par défaut des agents' });
    const chip = (name: RegExp) => within(modes).getByRole('button', { name });
    expect(chip(/^Plan/)).toBeEnabled();
    expect(chip(/^Demander/)).toBeEnabled();
    expect(chip(/^Auto/)).toBeDisabled();
    expect(chip(/^Édits auto/)).toBeDisabled();
    // The row itself is not locked: a bound greys its choices, not the row.
    expect(modes.closest('.sr')).not.toHaveClass('locked');
  });

  it('shows nothing locked when the core sends no lock', () => {
    setLocks(undefined);
    backend();
    app.modal = { kind: 'settings', page: 'general' };
    render(SettingsModal, { page: 'general' });
    expect(rowOf('Statistiques d’usage')).not.toHaveClass('locked');
    expect(screen.queryByText('Imposé par la stratégie de ce poste')).toBeNull();
    expect(screen.queryByRole('heading', { name: 'Stratégies de ce poste' })).toBeNull();
  });
});
