import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import { agent, resetApp } from '../test/ipc';
import type { Account, AccountUsage, RateWindow } from '../lib/types';
import AccountsPanel from './AccountsPanel.svelte';

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true };
const TEAM: Account = { id: 'team', name: 'Équipe', configDir: 'C:\\claude\\team', claudePath: '', active: false };

const NOW = new Date(2026, 9, 10, 14, 59).getTime();
const win = (pct: number, hours = 3.0167): RateWindow => ({ pct, resetsAt: NOW + hours * 3_600_000 });
const read = (id: string, over: Partial<AccountUsage> = {}): AccountUsage => ({
  id,
  fiveHour: null,
  sevenDay: null,
  connected: true,
  reason: null,
  todayCost: 0,
  updatedAt: 1,
  ...over,
});

const panel = () => screen.getByRole('dialog', { name: 'Quotas des comptes Claude' });
const group = (name: string) => within(panel()).getByRole('group', { name });

describe('AccountsPanel', () => {
  beforeEach(() => {
    resetApp();
    app.now = NOW;
    app.settings.accounts = [PRINCIPAL, PRO, TEAM];
    app.settings.quotaPause = 90;
    app.usage.current = 'pro';
    app.usage.accounts = [
      read('principal', { fiveHour: win(95), sevenDay: win(30, 100) }),
      read('pro', { fiveHour: win(42), sevenDay: win(12, 108) }),
      read('team', { fiveHour: win(5) }),
    ];
  });

  it('is a dialog over the status bar that names every account, the current one first and marked', () => {
    render(AccountsPanel, { onclose: () => {} });
    expect(panel()).toHaveAttribute('aria-modal', 'true');
    const names = within(panel())
      .getAllByRole('group')
      .map((g) => g.getAttribute('aria-label'));
    expect(names).toEqual(['Pro', 'Principal', 'Équipe']);
    expect(within(group('Pro')).getByText('en cours')).toBeInTheDocument();
    expect(group('Pro')).toHaveAttribute('aria-current', 'true');
    expect(within(group('Principal')).queryByText('en cours')).not.toBeInTheDocument();
    expect(group('Principal')).not.toHaveAttribute('aria-current');
    expect(screen.getAllByText('en cours')).toHaveLength(1);
  });

  it('gives the two windows of an account with their bar, the percentage written and the time left', () => {
    render(AccountsPanel, { onclose: () => {} });
    const pro = group('Pro');
    const five = within(pro).getByRole('meter', { name: 'Quota sur 5 heures' });
    const week = within(pro).getByRole('meter', { name: 'Quota sur 7 jours' });
    expect(five).toHaveAttribute('aria-valuenow', '42');
    expect(week).toHaveAttribute('aria-valuenow', '12');
    expect(pro).toHaveTextContent('5h');
    expect(pro).toHaveTextContent('42 %');
    expect(pro).toHaveTextContent('reset 3h01');
    expect(pro).toHaveTextContent('7j');
    expect(pro).toHaveTextContent('12 %');
    expect(pro).toHaveTextContent('reset 4j 12h');
  });

  it('signals an account with a window past the pause threshold, and only that one', () => {
    render(AccountsPanel, { onclose: () => {} });
    expect(within(group('Principal')).getByText('au-delà du seuil de pause')).toBeInTheDocument();
    expect(within(group('Pro')).queryByText('au-delà du seuil de pause')).not.toBeInTheDocument();
    expect(screen.getAllByText('au-delà du seuil de pause')).toHaveLength(1);
  });

  it('does not signal a window past the threshold on an account that is switched off', () => {
    app.usage.accounts = [read('team', { fiveHour: win(100) })];
    render(AccountsPanel, { onclose: () => {} });
    expect(screen.queryByText('au-delà du seuil de pause')).not.toBeInTheDocument();
  });

  it('dims an account that is switched off and says so', () => {
    render(AccountsPanel, { onclose: () => {} });
    expect(group('Équipe')).toHaveClass('off');
    expect(within(group('Équipe')).getByText('Inactif')).toBeInTheDocument();
    expect(group('Pro')).not.toHaveClass('off');
    expect(group('Principal')).not.toHaveClass('off');
  });

  it('dims an account that is not signed in, with the reason the backend gives, and no windows when it has none', () => {
    app.usage.accounts = [read('pro', { connected: false, reason: 'Pas connecté' })];
    render(AccountsPanel, { onclose: () => {} });
    const pro = group('Pro');
    expect(pro).toHaveClass('off');
    expect(pro).toHaveTextContent('Pas connecté');
    expect(within(pro).queryByRole('meter')).not.toBeInTheDocument();
  });

  it('dims an account whose sign-in expired, with the reason, and keeps the windows it was last read with', () => {
    app.usage.accounts = [
      read('pro', { reason: 'Connexion expirée : relance Claude Code pour ce compte.', fiveHour: win(42), sevenDay: win(12, 108) }),
    ];
    render(AccountsPanel, { onclose: () => {} });
    const pro = group('Pro');
    expect(pro).toHaveClass('off');
    expect(pro).toHaveTextContent('Connexion expirée : relance Claude Code pour ce compte.');
    expect(within(pro).getAllByRole('meter')).toHaveLength(2);
    expect(pro).toHaveTextContent('42 %');
  });

  it('says that Main is not signed in with a claude.ai account when it is run with an API key', () => {
    app.usage.accounts = [read('principal', { connected: false, reason: 'Pas connecté' })];
    render(AccountsPanel, { onclose: () => {} });
    expect(group('Principal')).toHaveTextContent('Pas connecté par un compte claude.ai (clé d’API ?)');
  });

  it('shows the dashes of the windows of an account that was not read yet', () => {
    app.usage.accounts = [];
    render(AccountsPanel, { onclose: () => {} });
    const pro = group('Pro');
    expect(within(pro).getAllByRole('meter')).toHaveLength(2);
    expect(within(pro).getAllByText('—')).toHaveLength(2);
    expect(pro).not.toHaveClass('off');
  });

  it('says what each account spent today, on the line of its name', () => {
    app.usage.accounts = [read('principal', { todayCost: 1.2 }), read('pro', { todayCost: 0.5 }), read('team', { todayCost: 0 })];
    render(AccountsPanel, { onclose: () => {} });
    expect(group('Principal')).toHaveTextContent("Aujourd'hui : 1,20 $");
    expect(group('Pro')).toHaveTextContent("Aujourd'hui : 0,50 $");
    // An account switched off or that spent nothing says so too, rather than leave a hole.
    expect(group('Équipe')).toHaveTextContent("Aujourd'hui : 0,00 $");
    const head = within(group('Pro')).getByText("Aujourd'hui : 0,50 $").parentElement!;
    expect(head).toHaveClass('head');
    expect(head).toHaveTextContent('Pro');
  });

  it('says it for an account that was never read, and counts the turn running on an agent as an estimate', () => {
    app.usage.accounts = [read('principal', { todayCost: 1.2 })];
    app.agents = { a1: agent({ id: 'a1', account: 'pro', liveCost: 0.3 }) };
    render(AccountsPanel, { onclose: () => {} });
    // Pro has no reading yet, but its agent is running.
    expect(group('Pro')).toHaveTextContent("Aujourd'hui : ≈ 0,30 $");
    expect(within(group('Pro')).getByText(/Aujourd'hui/)).toHaveAttribute('title', expect.stringContaining('Estimation'));
    expect(group('Principal')).toHaveTextContent("Aujourd'hui : 1,20 $");
    expect(within(group('Principal')).getByText(/Aujourd'hui/)).not.toHaveAttribute('title');
  });

  it('takes the focus when it opens', () => {
    render(AccountsPanel, { onclose: () => {} });
    expect(panel()).toHaveFocus();
  });

  it('closes on Escape, before Escape can reach anything else', async () => {
    const onclose = vi.fn();
    const elsewhere = vi.fn();
    document.addEventListener('keydown', elsewhere);
    render(AccountsPanel, { onclose });
    await userEvent.keyboard('{Escape}');
    document.removeEventListener('keydown', elsewhere);
    expect(onclose).toHaveBeenCalledTimes(1);
    expect(elsewhere).not.toHaveBeenCalled();
  });

  it('closes on a click outside it', async () => {
    const onclose = vi.fn();
    const { container } = render(AccountsPanel, { onclose });
    const backdrop = container.ownerDocument.querySelector('.backdrop')!;
    await fireEvent.click(backdrop);
    expect(onclose).toHaveBeenCalledTimes(1);
  });

  it('stays open on a click inside it', async () => {
    const onclose = vi.fn();
    render(AccountsPanel, { onclose });
    await userEvent.click(group('Pro'));
    expect(onclose).not.toHaveBeenCalled();
  });

  it('keeps the focus in the panel when Tab is pressed: nothing in it takes one', async () => {
    render(AccountsPanel, { onclose: () => {} });
    await userEvent.tab();
    expect(panel()).toHaveFocus();
  });

  it('follows the clock and the quota as they are read', async () => {
    render(AccountsPanel, { onclose: () => {} });
    expect(group('Pro')).toHaveTextContent('reset 3h01');
    app.now = NOW + 3_600_000;
    app.usage.accounts = [read('pro', { fiveHour: win(60), sevenDay: win(12, 108) })];
    await vi.waitFor(() => expect(group('Pro')).toHaveTextContent('60 %'));
    expect(group('Pro')).toHaveTextContent('reset 2h01');
  });

  it('writes it all in English', () => {
    setLang('en');
    render(AccountsPanel, { onclose: () => {} });
    const dialog = screen.getByRole('dialog', { name: 'Claude accounts quota' });
    expect(within(dialog).getByRole('group', { name: 'Pro' })).toHaveTextContent('current');
    // Principal is Main in English while it keeps its default name.
    expect(within(dialog).getByRole('group', { name: 'Main' })).toHaveTextContent('past the pause threshold');
    expect(within(dialog).getByRole('group', { name: 'Équipe' })).toHaveTextContent('Inactive');
    const pro = within(dialog).getByRole('group', { name: 'Pro' });
    expect(pro).toHaveTextContent('5h');
    expect(pro).toHaveTextContent('W');
    expect(pro).toHaveTextContent('42%');
    expect(pro).toHaveTextContent('reset 4d 12h');
    expect(within(pro).getByRole('meter', { name: '7-day quota' })).toBeInTheDocument();
  });

  it('writes what was spent today in English', () => {
    app.usage.accounts = [read('principal', { todayCost: 1.2 }), read('pro', { todayCost: 0.5 })];
    setLang('en');
    render(AccountsPanel, { onclose: () => {} });
    expect(screen.getByRole('group', { name: 'Main' })).toHaveTextContent('Today: $1.20');
    expect(screen.getByRole('group', { name: 'Pro' })).toHaveTextContent('Today: $0.50');
  });
});
