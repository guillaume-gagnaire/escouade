import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { menu } from '../../lib/menu.svelte';
import { app } from '../../lib/state.svelte';
import type { Account, AccountUsage, Agent, RateWindow, TurnItem } from '../../lib/types';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import TurnCard from './TurnCard.svelte';

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true };
const TEAM: Account = { id: 'team', name: 'Équipe', configDir: 'C:\\claude\\team', claudePath: '', active: true };
const OFF: Account = { id: 'off', name: 'Ancien', configDir: 'C:\\claude\\off', claudePath: '', active: false };

const NOW = new Date(2026, 9, 10, 14, 0).getTime();
const win = (pct: number): RateWindow => ({ pct, resetsAt: NOW + 3_600_000 });
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

const turn = (over: Partial<TurnItem> = {}): TurnItem => ({
  kind: 'turn',
  id: 'r1',
  ts: 1,
  durationMs: 100,
  cost: 0,
  tokens: 0,
  isError: true,
  interrupted: false,
  error: "You've hit your limit · resets 3pm",
  limited: true,
  ...over,
});

/** An agent stopped by the usage limit, in a window with these accounts. */
function stopped(over: Partial<Agent> = {}, accounts: Account[] = [PRINCIPAL, PRO], usage: AccountUsage[] = []) {
  resetApp();
  app.now = NOW;
  app.settings.accounts = accounts;
  app.settings.quotaPause = 90;
  app.usage.accounts = usage;
  return agent({ status: 'error', account: 'principal', resumeAt: NOW + 3_600_000, ...over });
}

const resume = (name: string) => screen.queryByRole('button', { name: `Reprendre sur ${name}` });

describe('TurnCard of a turn stopped by the usage limit, with several accounts', () => {
  beforeEach(() => {
    resetApp();
    menu.close();
  });

  it('offers to go on on the other account, and the backend is asked with the agent and the account', async () => {
    const backend = fakeBackend({ resume_on_account: () => null });
    const a = stopped();
    render(TurnCard, { item: turn(), agent: a, last: true });
    // Beside the wait for the reset, which stays.
    expect(screen.getByText(/^Reprise automatique/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Reprendre sur Pro' }));
    expect(backend.called('resume_on_account')[0].args).toEqual({ agentId: 'a1', account: 'pro' });
  });

  it('is there with no resume planned too (the automatic resume turned off, no reset known)', () => {
    const a = stopped({ resumeAt: null });
    render(TurnCard, { item: turn(), agent: a, last: true });
    expect(screen.queryByText(/^Reprise automatique/)).not.toBeInTheDocument();
    expect(resume('Pro')).toBeInTheDocument();
  });

  it('names the first account under the threshold, in the order of the settings, with a menu for the others', async () => {
    const backend = fakeBackend({ resume_on_account: () => null });
    // Pro is past « Pause au-delà du quota » (90): Équipe is the first; switched off, Ancien is never offered.
    const a = stopped({}, [PRINCIPAL, PRO, TEAM, OFF], [read('pro', { fiveHour: win(95) })]);
    const alone = render(TurnCard, { item: turn(), agent: a, last: true });
    expect(resume('Pro')).not.toBeInTheDocument();
    expect(resume('Équipe')).toBeInTheDocument();
    // Alone: no menu.
    expect(screen.queryByRole('button', { name: 'Reprendre sur un autre compte' })).not.toBeInTheDocument();
    alone.unmount();
    const withMore = stopped({}, [PRINCIPAL, PRO, TEAM, OFF], [read('pro', { fiveHour: win(40) }), read('team', { sevenDay: win(10) })]);
    const { unmount } = render(TurnCard, { item: turn(), agent: withMore, last: true });
    expect(screen.getAllByRole('button', { name: 'Reprendre sur Pro' })).toHaveLength(1);
    const more = screen.getByRole('button', { name: 'Reprendre sur un autre compte' });
    expect(more).toHaveAttribute('aria-haspopup', 'menu');
    await userEvent.click(more);
    expect(menu.open?.items.map((i) => i.label)).toEqual(['Reprendre sur Équipe']);
    menu.open?.items[0].onClick?.();
    await waitFor(() => expect(backend.called('resume_on_account').at(-1)?.args).toEqual({ agentId: 'a1', account: 'team' }));
    unmount();
  });

  it('leaves out the agent’s own account, the accounts past the threshold, switched off or not signed in', () => {
    const accounts = [PRINCIPAL, PRO, TEAM, OFF];
    const usage = [read('pro', { fiveHour: win(90), sevenDay: win(10) }), read('team', { connected: false, reason: 'Pas connecté' })];
    // An agent on Pro: Principal is the only one left.
    render(TurnCard, { item: turn(), agent: stopped({ account: 'pro' }, accounts, usage), last: true });
    expect(resume('Principal')).toBeInTheDocument();
    expect(screen.getAllByRole('button').map((b) => b.textContent?.trim())).toEqual(['Annuler la reprise', 'Reprendre sur Principal']);
  });

  it('offers nothing with a single account, with no other one to go to, or while the agent works', () => {
    // A single account.
    const alone = render(TurnCard, { item: turn(), agent: stopped({}, [PRINCIPAL]), last: true });
    expect(screen.queryByRole('button', { name: /^Reprendre sur/ })).not.toBeInTheDocument();
    alone.unmount();
    // The others are past the threshold.
    const over = render(TurnCard, {
      item: turn(),
      agent: stopped({}, [PRINCIPAL, PRO], [read('pro', { sevenDay: win(99) })]),
      last: true,
    });
    expect(screen.queryByRole('button', { name: /^Reprendre sur/ })).not.toBeInTheDocument();
    over.unmount();
    // It goes on already (the resume sent "continue"): the card of its earlier turn does not offer it again.
    const busy = render(TurnCard, { item: turn(), agent: stopped({ status: 'running', resumeAt: null }), last: true });
    expect(screen.queryByRole('button', { name: /^Reprendre sur/ })).not.toBeInTheDocument();
    busy.unmount();
  });

  it('is only on the conversation’s latest turn, and only for a turn stopped by the limit', () => {
    const earlier = render(TurnCard, { item: turn(), agent: stopped(), last: false, latest: false });
    expect(screen.queryByRole('button', { name: /^Reprendre sur/ })).not.toBeInTheDocument();
    earlier.unmount();
    // A turn that failed otherwise has no usage limit to go around.
    const failed = render(TurnCard, {
      item: turn({ limited: false, error: 'Rate limit reached' }),
      agent: stopped({ resumeAt: null }),
      last: true,
    });
    expect(screen.queryByRole('button', { name: /^Reprendre sur/ })).not.toBeInTheDocument();
    failed.unmount();
    // A turn saved before there were accounts (no `limited`) still has its wait for the reset.
    const old = render(TurnCard, { item: turn({ limited: undefined }), agent: stopped(), last: true });
    expect(resume('Pro')).toBeInTheDocument();
    old.unmount();
  });

  it('says it in English', async () => {
    setLang('en');
    fakeBackend({ resume_on_account: () => null });
    app.settings.accounts = [PRINCIPAL, PRO, TEAM];
    app.now = NOW;
    render(TurnCard, { item: turn(), agent: agent({ status: 'error', account: 'principal', resumeAt: null }), last: true });
    expect(screen.getByRole('button', { name: 'Resume on Pro' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Resume on another account' }));
    expect(menu.open?.items.map((i) => i.label)).toEqual(['Resume on Équipe']);
  });
});

describe('TurnCard of a resume that failed on the other account', () => {
  beforeEach(() => {
    resetApp();
    menu.close();
  });

  const failed = (over: Partial<Agent> = {}) =>
    stopped({ account: 'pro', movedFrom: 'principal', resumeAt: null, status: 'error', ...over });

  it('tells which account failed and why, and offers to go back to the one before', async () => {
    const backend = fakeBackend({ back_to_previous_account: () => null });
    const a = failed();
    render(TurnCard, { item: turn({ limited: false, error: 'Invalid API key · Please run /login' }), agent: a, last: true });
    expect(screen.getByText('La reprise sur Pro a échoué : Invalid API key · Please run /login.')).toBeInTheDocument();
    // The error is said once, in that sentence.
    expect(screen.queryByText('Invalid API key · Please run /login')).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Revenir sur Principal' }));
    expect(backend.called('back_to_previous_account')[0].args).toEqual({ agentId: 'a1' });
  });

  it('is the card of an agent that came from nowhere as it always was', () => {
    const a = failed({ movedFrom: null });
    render(TurnCard, { item: turn({ limited: false, error: 'Rate limit reached' }), agent: a, last: true });
    expect(screen.getByText('Rate limit reached')).toBeInTheDocument();
    expect(screen.queryByText(/La reprise sur/)).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^Revenir sur/ })).not.toBeInTheDocument();
  });

  it('says nothing of it for an earlier turn, while the agent works, or when the account before is gone or off', () => {
    const earlier = render(TurnCard, { item: turn({ limited: false }), agent: failed(), last: false, latest: false });
    expect(screen.queryByText(/La reprise sur/)).not.toBeInTheDocument();
    earlier.unmount();
    const busy = render(TurnCard, { item: turn({ limited: false }), agent: failed({ status: 'running' }), last: true });
    expect(screen.queryByRole('button', { name: /^Revenir sur/ })).not.toBeInTheDocument();
    busy.unmount();
    // The account before was removed: nothing to go back to, but the failure is told.
    const gone = render(TurnCard, { item: turn({ limited: false }), agent: failed({ movedFrom: 'removed' }), last: true });
    expect(screen.getByText(/^La reprise sur Pro a échoué/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^Revenir sur/ })).not.toBeInTheDocument();
    gone.unmount();
  });

  it('says it in English', () => {
    setLang('en');
    app.settings.accounts = [PRINCIPAL, PRO];
    render(TurnCard, {
      item: turn({ limited: false, error: 'Invalid API key' }),
      agent: agent({ status: 'error', account: 'pro', movedFrom: 'principal' }),
      last: true,
    });
    expect(screen.getByText('Resuming on Pro failed: Invalid API key.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Go back to Main' })).toBeInTheDocument();
  });
});
