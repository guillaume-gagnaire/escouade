import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import { agent, BARE_ENGINE, CURSOR_ENGINE, CLAUDE_ENGINE, VIBE_ENGINE, resetApp, usageOf } from '../test/ipc';
import type { Account, AccountUsage, RateWindow } from '../lib/types';
import AccountsPanel from './AccountsPanel.svelte';

const opened = vi.hoisted(() => [] as string[]);
vi.mock('../lib/opener', () => ({ openUrl: vi.fn(async (u: string) => void opened.push(u)) }));

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true };
const TEAM: Account = { id: 'team', name: 'Équipe', configDir: 'C:\\claude\\team', claudePath: '', active: false };

const NOW = new Date(2026, 9, 10, 14, 59).getTime();
const win = (pct: number, hours = 3.0167): RateWindow => ({ pct, resetsAt: NOW + hours * 3_600_000 });
const read = (id: string, over: Parameters<typeof usageOf>[1] = {}): AccountUsage => usageOf(id, over);

const panel = () => screen.getByRole('dialog', { name: 'Quotas des comptes' });
const group = (name: string) => within(panel()).getByRole('group', { name });

describe('AccountsPanel', () => {
  beforeEach(() => {
    resetApp();
    app.now = NOW;
    app.settings.accounts = [PRINCIPAL, PRO, TEAM];
    app.settings.quotaPause = 90;
    app.usage.current = { claude: 'pro' };
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

  it('says how old a reading is once what it shows may no longer be true', () => {
    const at = { pro: app.now - 2 * 3_600_000, team: app.now - 5 * 60_000 } as Record<string, number>;
    app.usage = {
      ...app.usage,
      accounts: app.usage.accounts.map((a) => ({ ...a, updatedAt: at[a.id] ?? a.updatedAt })),
    };
    render(AccountsPanel, { onclose: () => {} });
    expect(group('Pro')).toHaveTextContent('Dernière lecture il y a 2 h : ce compte a pu consommer depuis.');
    // Read a moment ago, or not yet in this run: nothing to say.
    expect(group('Équipe')).not.toHaveTextContent('Dernière lecture');
    expect(group('Principal')).not.toHaveTextContent('Dernière lecture');
  });

  it('gives the two windows of an account with their bar, the percentage written and the time left', () => {
    render(AccountsPanel, { onclose: () => {} });
    const pro = group('Pro');
    const five = within(pro).getByRole('meter', { name: 'Quota sur 5 heures' });
    const week = within(pro).getByRole('meter', { name: 'Quota sur 7 jours' });
    expect(five).toHaveAttribute('aria-valuenow', '42');
    expect(week).toHaveAttribute('aria-valuenow', '12');
    expect(pro).toHaveTextContent('5 h');
    expect(pro).toHaveTextContent('42 %');
    expect(pro).toHaveTextContent('reset 3h01');
    expect(pro).toHaveTextContent('7 j');
    expect(pro).toHaveTextContent('12 %');
    expect(pro).toHaveTextContent('reset 4j 12h');
  });

  it('signals an account with a window past the pause threshold, and only that one', () => {
    render(AccountsPanel, { onclose: () => {} });
    expect(within(group('Principal')).getByText('au-delà du seuil de pause ou de changement de compte')).toBeInTheDocument();
    expect(within(group('Pro')).queryByText('au-delà du seuil de pause ou de changement de compte')).not.toBeInTheDocument();
    expect(screen.getAllByText('au-delà du seuil de pause ou de changement de compte')).toHaveLength(1);
  });

  it('does not signal a window past the threshold on an account that is switched off', () => {
    app.usage.accounts = [read('team', { fiveHour: win(100) })];
    render(AccountsPanel, { onclose: () => {} });
    expect(screen.queryByText('au-delà du seuil de pause ou de changement de compte')).not.toBeInTheDocument();
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
    // On the line of its name: its cell, beside the name's.
    const today = within(group('Pro')).getByText("Aujourd'hui : 0,50 $");
    expect(today).toHaveClass('today');
    expect(today.previousElementSibling).toHaveTextContent(/^Pro/);
  });

  it('says it for an account that was never read, and counts the turn running on an agent as an estimate', () => {
    app.usage.accounts = [read('principal', { todayCost: 1.2 })];
    app.agents = { a1: agent({ id: 'a1', account: 'pro', liveCost: 0.3 }) };
    render(AccountsPanel, { onclose: () => {} });
    // Pro has no reading yet, but its agent is running.
    expect(group('Pro')).toHaveTextContent("Aujourd'hui : ≈ 0,30 $");
    expect(within(group('Pro')).getByText(/Aujourd'hui/)).toHaveAttribute('data-tip', expect.stringContaining('Estimation'));
    expect(group('Principal')).toHaveTextContent("Aujourd'hui : 1,20 $");
    expect(within(group('Principal')).getByText(/Aujourd'hui/)).not.toHaveAttribute('data-tip');
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
    const dialog = screen.getByRole('dialog', { name: 'Accounts quota' });
    expect(within(dialog).getByRole('group', { name: 'Pro' })).toHaveTextContent('current');
    // Principal is Main in English while it keeps its default name.
    expect(within(dialog).getByRole('group', { name: 'Main' })).toHaveTextContent('past the pause or switch threshold');
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

describe('AccountsPanel with the accounts of two harnesses (ESC-30)', () => {
  const WORK: Account = { id: 'work', name: 'Travail', engine: 'codex', configDir: 'C:\codex\work', claudePath: '', active: true };

  beforeEach(() => {
    resetApp();
    app.now = NOW;
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.settings.accounts = [PRINCIPAL, PRO, WORK];
    app.usage.current = { claude: 'pro', codex: 'work' };
    app.usage.accounts = [read('principal', { fiveHour: win(10) }), read('pro', { fiveHour: win(42) }), read('work', { engine: 'codex' })];
  });

  it('says a Cursor account’s usage is on its dashboard, with no cost of the day (ESC-43)', async () => {
    const CURSOR_MAIN: Account = {
      id: 'principal-cursor',
      name: 'Principal',
      engine: 'cursor',
      configDir: '',
      claudePath: '',
      active: true,
    };
    app.engines = [CLAUDE_ENGINE, CURSOR_ENGINE];
    app.settings.accounts = [PRINCIPAL, CURSOR_MAIN];
    app.usage.current = { claude: 'principal', cursor: 'principal-cursor' };
    app.usage.accounts = [read('principal', { fiveHour: win(10) }), read('principal-cursor', { engine: 'cursor' })];
    opened.length = 0;
    render(AccountsPanel, { onclose: () => {} });
    const cursor = within(panel()).getByRole('region', { name: 'Comptes Cursor CLI' });
    expect(within(cursor).queryByRole('meter')).not.toBeInTheDocument();
    expect(within(cursor).queryByText(/Aujourd’hui/)).not.toBeInTheDocument();
    expect(within(cursor).getByText(/Cursor CLI ne donne ni son usage ni son coût à Escouade/)).toBeInTheDocument();
    await userEvent.click(within(cursor).getByRole('button', { name: 'Voir l’usage sur cursor.com' }));
    expect(opened).toEqual(['https://cursor.com/dashboard?tab=usage']);
  });

  it('shows what a Vibe account spent today, with no quota and its usage on Mistral’s console (ESC-267)', async () => {
    const VIBE_MAIN: Account = { id: 'principal-vibe', name: 'Principal', engine: 'vibe', configDir: '', claudePath: '', active: true };
    app.engines = [CLAUDE_ENGINE, VIBE_ENGINE];
    app.settings.accounts = [PRINCIPAL, VIBE_MAIN];
    app.usage.current = { claude: 'principal', vibe: 'principal-vibe' };
    app.usage.accounts = [read('principal', { fiveHour: win(10) }), { ...read('principal-vibe', { engine: 'vibe' }), todayCost: 0.37 }];
    opened.length = 0;
    render(AccountsPanel, { onclose: () => {} });
    const vibe = within(panel()).getByRole('region', { name: 'Comptes Mistral Vibe' });
    expect(within(vibe).queryByRole('meter')).not.toBeInTheDocument();
    expect(within(vibe).getByText(/Aujourd.hui.:.0,37/)).toBeInTheDocument();
    expect(within(vibe).getByText('Pas de quota : seul le coût de ses tours compte.')).toBeInTheDocument();
    expect(within(vibe).queryByText(/ne donne ni son usage ni son coût/)).not.toBeInTheDocument();
    await userEvent.click(within(vibe).getByRole('button', { name: 'Voir l’usage sur admin.mistral.ai' }));
    expect(opened).toEqual(['https://admin.mistral.ai/organization/usage']);
  });

  it('groups the accounts by harness, each with its current one first', () => {
    render(AccountsPanel, { onclose: () => {} });
    const groups = within(panel()).getAllByRole('region');
    expect(groups.map((g) => g.getAttribute('aria-label'))).toEqual(['Comptes Claude Code', 'Comptes Harness nu']);
    expect(
      within(groups[0])
        .getAllByRole('group')
        .map((g) => g.getAttribute('aria-label')),
    ).toEqual(['Pro', 'Principal']);
    // Each harness has its own current account.
    expect(within(groups[1]).getByRole('group', { name: 'Travail' })).toHaveAttribute('aria-current', 'true');
    // A harness without a quota has no bar: what its turns cost, alone.
    expect(within(groups[1]).queryByRole('meter')).not.toBeInTheDocument();
    expect(within(groups[1]).getByText('Pas de quota : seul le coût de ses tours compte.')).toBeInTheDocument();
    // Each Claude account was read with its 5-hour window alone: one bar each.
    expect(within(groups[0]).getAllByRole('meter')).toHaveLength(2);
  });

  it('follows the order of the harnesses', () => {
    app.settings.engineOrder = ['codex', 'claude'];
    render(AccountsPanel, { onclose: () => {} });
    expect(
      within(panel())
        .getAllByRole('heading')
        // After the harness's mark.
        .map((h) => h.lastChild?.textContent),
    ).toEqual(['Comptes Harness nu', 'Comptes Claude Code']);
  });
});

describe('AccountsPanel, what the lot 0 bis adds (choices 21 to 23)', () => {
  const WORK: Account = { id: 'work', name: 'Travail', engine: 'codex', configDir: 'C:\codex\work', claudePath: '', active: true };

  beforeEach(() => {
    resetApp();
    app.now = NOW;
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.settings.accounts = [PRO, WORK];
    app.usage.current = { claude: 'pro', codex: 'work' };
    app.usage.accounts = [read('pro', { fiveHour: win(20, 3) }), read('work', { engine: 'codex', todayCost: 0.42, dailyBudget: 5 })];
  });

  it('writes every projection, green ones too, with a note on their colours', () => {
    render(AccountsPanel, { onclose: () => {} });
    expect(within(group('Pro')).getByText('→ 50 %')).toHaveClass('ok');
    expect(panel()).toHaveTextContent('La barre d’état ne montre que celles qui passent 80 %.');
  });

  it('says what an account answering as extra usage does, under it, with an icon', () => {
    app.usage.accounts = app.usage.accounts.map((u) => (u.id === 'pro' ? { ...u, overage: true } : u));
    render(AccountsPanel, { onclose: () => {} });
    const bill = within(group('Pro')).getByText(/^Pro : usage supplémentaire en cours, facturé à part/);
    expect(bill.parentElement).toHaveClass('bill');
    expect(bill.parentElement!.querySelector('[data-icon="info"]')).not.toBeNull();
  });

  it('gives a harness without a quota its cost of the day out of its daily budget, on a gauge', () => {
    render(AccountsPanel, { onclose: () => {} });
    const work = group('Travail');
    expect(work).toHaveTextContent('Budget par jour');
    expect(work).toHaveTextContent('0,42 $ / 5,00 $');
    expect(work).toHaveTextContent('Pas de quota : seul le coût de ses tours compte. Remise à zéro à minuit.');
    expect(work.querySelector<HTMLElement>('.gauge .fill')!.style.width).toMatch(/^8\.(4|39+)\d*%$/);
  });
});
