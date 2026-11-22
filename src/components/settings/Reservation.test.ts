import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// The settings' other tabs have logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

import { setLang } from '../../lib/i18n';
import { setLocks } from '../../lib/locks.svelte';
import { app } from '../../lib/state.svelte';
import type { LockedBy, Locks } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

// ESC-564: `accounts.reservation`, imposed by this device or a team, shown where one reserves an account.
const DEVICE: LockedBy = { kind: 'device', source: 'HKLM' };
const reservation = (value: 'denied' | 'required', by: LockedBy = DEVICE): Locks => ({
  locks: { 'accounts.reservation': { key: 'accounts.reservation', value, by, bounds: [] } },
  rows: {},
  warnings: [],
});

const LIMIT = 'Comptes partagés à la limite';
const pro = { id: 'pro', name: 'Pro', configDir: 'C:/acc/pro', claudePath: '', active: true };
const principal = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };

describe('the reservation of an account imposed (ESC-564)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project({ private: true }), project({ id: 'p2', name: 'site' })] });
    app.settings.accounts = [principal, pro];
    app.modal = { kind: 'settings' };
  });

  it('denied: a private project’s page shows the padlock, and greys « Réserver … » and « Ajouter un compte réservé… »', () => {
    setLocks(reservation('denied'));
    fakeBackend();
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    expect(screen.getByTestId('reservation-lock')).toHaveTextContent(
      'Imposé par la stratégie de ce poste · Aucun compte n’est réservé à un projet : un projet privé part sur les comptes partagés, sa clôture protège ses sessions.',
    );
    expect(screen.getByRole('button', { name: 'Réserver Pro à ce projet' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Ajouter un compte réservé…' })).toBeDisabled();
    // « Comptes partagés à la limite » stays the user’s: nothing is reserved to stop at.
    expect(screen.getByRole('switch', { name: LIMIT }).closest('.sr')).not.toHaveClass('locked');
  });

  it('required: « Comptes partagés à la limite » locked with who imposes it, and its last account kept', async () => {
    setLocks(reservation('required', { kind: 'team', team: 'Acme' }));
    app.settings.accounts = [principal, { ...pro, projects: ['p1'] }];
    const backend = fakeBackend();
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    const row = screen.getByRole('switch', { name: LIMIT }).closest('.sr') as HTMLElement;
    expect(row).toHaveClass('locked');
    expect(within(row).getByText('Imposé par l’équipe Acme')).toBeInTheDocument();
    expect(row.querySelector('.ctl')).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByTestId('reservation-lock')).toHaveTextContent(
      'Imposé par l’équipe Acme · Ses agents ne partent que sur un compte réservé à lui seul, jamais sur un compte partagé, même à la limite.',
    );
    // Its only account of its own: « Ne plus le réserver » greyed, nothing sent.
    const unreserve = within(screen.getByTestId('private-own')).getByRole('button', { name: 'Ne plus le réserver' });
    expect(unreserve).toBeDisabled();
    await userEvent.click(unreserve);
    expect(backend.called('account_update')).toHaveLength(0);
  });

  it('denied: « Réservé aux projets » of each account greyed in Comptes et harness, with the padlock', () => {
    setLocks(reservation('denied'));
    fakeBackend();
    render(SettingsModal, { page: 'accounts' });
    const groups = screen.getAllByRole('group', { name: 'Réservé aux projets' });
    expect(groups.length).toBeGreaterThan(0);
    for (const g of groups) {
      expect(g.querySelector('.chips')).toHaveAttribute('aria-disabled', 'true');
      expect(within(g).getByTestId('reservation-lock')).toHaveTextContent(/^Imposé par la stratégie de ce poste · Aucun compte/);
    }
    // « Réserver à « demo-api » », under an account the private project shares, is not offered.
    expect(screen.queryByRole('button', { name: /^Réserver à/ })).toBeNull();
  });

  it('shows nothing of it without the policy', () => {
    setLocks(undefined);
    fakeBackend();
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    expect(screen.queryByTestId('reservation-lock')).toBeNull();
    expect(screen.getByRole('button', { name: 'Réserver Pro à ce projet' })).toBeEnabled();
    expect(screen.getByRole('switch', { name: LIMIT }).closest('.sr')).not.toHaveClass('locked');
  });

  it('says it in English', () => {
    setLang('en');
    setLocks(reservation('denied'));
    fakeBackend();
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    expect(screen.getByTestId('reservation-lock')).toHaveTextContent(
      'Imposed by this device’s policy · No account is reserved to a project: a private project goes on the shared accounts, its fence keeps its sessions.',
    );
  });
});
