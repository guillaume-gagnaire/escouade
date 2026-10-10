import { beforeEach, describe, expect, it } from 'vitest';
import { agent, project, resetApp } from '../test/ipc';
import { newAgentAccount, overThreshold, pausedAccounts, pauseOf, quotaOf, quotaRows } from './accounts';
import { setLang } from './i18n';
import { app } from './state.svelte';
import type { Account, AccountUsage, AutopilotPause, RateWindow } from './types';

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true };
const TEAM: Account = { id: 'team', name: 'Équipe', configDir: 'C:\\claude\\team', claudePath: '', active: false };

const pause = (over: Partial<AutopilotPause> = {}): AutopilotPause => ({ reason: 'fiveHour', pct: 97, until: 5_000, ...over });

describe('the accounts of a project', () => {
  beforeEach(() => {
    resetApp();
    app.settings.accounts = [PRINCIPAL, PRO, TEAM];
  });

  it('goes to the account the project prefers when it is active, else to the one new agents go to now', () => {
    app.usage.current = 'pro';
    expect(newAgentAccount(project())?.id).toBe('pro');
    expect(newAgentAccount(project({ account: 'principal' }))?.id).toBe('principal');
    // Switched off, or gone: any account again.
    expect(newAgentAccount(project({ account: 'team' }))?.id).toBe('pro');
    expect(newAgentAccount(project({ account: 'parti' }))?.id).toBe('pro');
    expect(newAgentAccount(undefined)?.id).toBe('pro');
    // Before the backend told which: the first active one.
    app.usage.current = '';
    expect(newAgentAccount(project())?.id).toBe('principal');
  });

  it('is paused by the pause of any account, or by the one of the project that prefers an account', () => {
    app.autopilotPause = pause({ until: 1 });
    app.projectPauses = { p2: pause({ until: 2, accounts: ['pro'] }) };
    expect(pauseOf(project())?.until).toBe(1);
    expect(pauseOf(project({ id: 'p2' }))?.until).toBe(1);
    // A project that prefers an account waits for it alone.
    expect(pauseOf(project({ id: 'p2', account: 'pro' }))?.until).toBe(2);
    expect(pauseOf(project({ id: 'p3', account: 'pro' }))).toBeNull();
    // An account switched off is no preference.
    expect(pauseOf(project({ id: 'p2', account: 'team' }))?.until).toBe(1);
    app.autopilotPause = null;
    expect(pauseOf(project())).toBeNull();
  });

  it('names the accounts past the threshold, not after a usage limit and not with a single account', () => {
    expect(pausedAccounts(pause())).toBe('');
    expect(pausedAccounts(pause({ accounts: [] }))).toBe('');
    expect(pausedAccounts(pause({ accounts: ['pro'] }))).toBe('Le compte Pro a passé le seuil.');
    expect(pausedAccounts(pause({ accounts: ['principal', 'pro'] }))).toBe('Les comptes Principal et Pro ont passé le seuil.');
    expect(pausedAccounts(pause({ reason: 'limit', pct: null, accounts: ['principal', 'pro'] }))).toBe('');
    // An account the window does not know is named by its id.
    expect(pausedAccounts(pause({ accounts: ['parti'] }))).toBe('Le compte parti a passé le seuil.');
    setLang('en');
    expect(pausedAccounts(pause({ accounts: ['principal', 'pro'] }))).toBe('The Main and Pro accounts are past the threshold.');
    expect(pausedAccounts(pause({ accounts: ['pro'] }))).toBe('The Pro account is past the threshold.');
  });

  it('waits for the quota of an account only when every account the project could use waits for its own', () => {
    const waits = (id: string, account: string, resumeAt: number, over = {}) =>
      agent({ id, account, resumeAt, projectId: 'p9', status: 'idle', ...over });
    const give = (...agents: ReturnType<typeof agent>[]) => (app.agents = Object.fromEntries(agents.map((a) => [a.id, a])));
    const any = project();
    // Nobody waits.
    expect(quotaOf(any)).toBeNull();
    // Only one of the two active accounts: a ticket starts on the other.
    give(waits('a1', 'principal', 9_000));
    expect(quotaOf(any)).toBeNull();
    // Both: nothing starts before the first resume (the one switched off does not count).
    give(waits('a1', 'principal', 9_000), waits('a2', 'pro', 7_000), waits('a3', 'principal', 8_000));
    expect(quotaOf(any)).toBe(7_000);
    // A project that prefers Principal waits for it alone.
    give(waits('a1', 'principal', 9_000));
    expect(quotaOf(project({ account: 'principal' }))).toBe(9_000);
    expect(quotaOf(project({ account: 'pro' }))).toBeNull();
    // An agent archived, or on an account the window does not know (Principal's), is counted as the backend does.
    give(waits('a1', 'principal', 9_000, { archived: true }), waits('a2', 'pro', 7_000));
    expect(quotaOf(any)).toBeNull();
    give(waits('a1', 'parti', 9_000), waits('a2', 'pro', 7_000));
    expect(quotaOf(any)).toBe(7_000);
  });

  it('is unchanged with a single account', () => {
    app.settings.accounts = [PRINCIPAL];
    app.agents = { a1: agent({ id: 'a1', resumeAt: 6_000 }), a2: agent({ id: 'a2', resumeAt: 4_000, archived: true }) };
    expect(quotaOf(project())).toBe(6_000);
  });
});

describe('the quota of each account', () => {
  const NOW = 1_000_000;
  const win = (pct: number, resetsAt: number | null = NOW + 3_600_000): RateWindow => ({ pct, resetsAt });
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

  beforeEach(() => {
    resetApp();
    app.settings.accounts = [PRINCIPAL, PRO, TEAM];
    app.settings.quotaPause = 90;
  });

  it('holds a window back from the threshold up, until an end still to come', () => {
    expect(overThreshold(win(89), 90, NOW)).toBe(false);
    expect(overThreshold(win(90), 90, NOW)).toBe(true);
    // Claude Code gives a fraction times 100: 0.9 may come as 89.99999….
    expect(overThreshold(win(89.9999999), 90, NOW)).toBe(true);
    expect(overThreshold(win(100), 90, NOW)).toBe(true);
    // Reset already, or an end nobody knows: it holds nothing back (a reading that no longer comes would hold forever).
    expect(overThreshold(win(100, NOW - 1), 90, NOW)).toBe(false);
    expect(overThreshold(win(100, null), 90, NOW)).toBe(false);
    expect(overThreshold(null, 90, NOW)).toBe(false);
  });

  it('lists every account, the current one first and the others in the order of the settings', () => {
    app.usage.current = 'pro';
    expect(quotaRows(NOW).map((r) => [r.account.id, r.current])).toEqual([
      ['pro', true],
      ['principal', false],
      ['team', false],
    ]);
    app.usage.current = 'principal';
    expect(quotaRows(NOW).map((r) => r.account.id)).toEqual(['principal', 'pro', 'team']);
    // Not told yet: the order of the settings, none marked.
    app.usage.current = '';
    expect(quotaRows(NOW).map((r) => [r.account.id, r.current])).toEqual([
      ['principal', false],
      ['pro', false],
      ['team', false],
    ]);
  });

  it('gives each account the windows it was read with, and none before it was', () => {
    app.usage.accounts = [read('pro', { fiveHour: win(42), sevenDay: win(12) })];
    const rows = quotaRows(NOW);
    expect(rows.find((r) => r.account.id === 'pro')?.usage).toMatchObject({ fiveHour: { pct: 42 }, sevenDay: { pct: 12 } });
    expect(rows.find((r) => r.account.id === 'principal')?.usage).toBeNull();
  });

  it('flags an active account with a window past the pause threshold', () => {
    app.usage.accounts = [
      read('principal', { fiveHour: win(95), sevenDay: win(10) }),
      read('pro', { fiveHour: win(40), sevenDay: win(91) }),
      // Switched off: nothing goes to it, the threshold does not matter.
      read('team', { fiveHour: win(100) }),
    ];
    expect(quotaRows(NOW).map((r) => [r.account.id, r.over])).toEqual([
      ['principal', true],
      ['pro', true],
      ['team', false],
    ]);
    app.settings.quotaPause = 100;
    expect(quotaRows(NOW).map((r) => r.over)).toEqual([false, false, false]);
  });

  it('says why an account is dimmed: switched off, not signed in, sign-in expired', () => {
    app.usage.accounts = [
      read('principal', { connected: false, reason: 'Pas connecté' }),
      read('pro', { connected: true, reason: 'Connexion expirée : relance Claude Code pour ce compte.', fiveHour: win(20) }),
      read('team', { connected: false, reason: 'Pas connecté' }),
    ];
    const reasons = Object.fromEntries(quotaRows(NOW).map((r) => [r.account.id, r.reasons]));
    // Main, signed in with a key and not claude.ai: the wording of the tab of the accounts.
    expect(reasons.principal).toEqual(['Pas connecté par un compte claude.ai (clé d’API ?)']);
    expect(reasons.pro).toEqual(['Connexion expirée : relance Claude Code pour ce compte.']);
    expect(reasons.team).toEqual(['Inactif', 'Pas connecté']);
    // Not read yet: nothing to say.
    app.usage.accounts = [];
    expect(Object.fromEntries(quotaRows(NOW).map((r) => [r.account.id, r.reasons]))).toEqual({
      principal: [],
      pro: [],
      team: ['Inactif'],
    });
  });

  it('gives what each account spent today, with the turns running on its agents as an estimate', () => {
    app.usage.accounts = [read('principal', { todayCost: 1.2 }), read('pro', { todayCost: 0.5 })];
    app.agents = {
      a1: agent({ id: 'a1', account: 'pro', liveCost: 0.3 }),
      a2: agent({ id: 'a2', account: 'principal' }),
      // On an account the settings no longer list: the backend takes it for Principal's.
      a3: agent({ id: 'a3', account: 'parti', liveCost: 0.1 }),
      // Archived: its turn is still running, as the status bar counts it.
      a4: agent({ id: 'a4', account: 'pro', liveCost: 0.2, archived: true }),
    };
    const today = Object.fromEntries(quotaRows(NOW).map((r) => [r.account.id, r.today]));
    expect(today.principal.cost).toBeCloseTo(1.3);
    expect(today.principal.estimated).toBe(true);
    expect(today.pro.cost).toBeCloseTo(1.0);
    expect(today.pro.estimated).toBe(true);
    // Not read yet, nothing running: nothing spent, nothing estimated.
    expect(today.team).toEqual({ cost: 0, estimated: false });
    // Together they are what the status bar says (the total and every running turn).
    app.usage.todayCost = 1.7;
    expect(Object.values(today).reduce((sum, t) => sum + t.cost, 0)).toBeCloseTo(app.usage.todayCost + app.liveCost);
  });

  it('says it in the language of the interface', () => {
    app.usage.accounts = [
      read('principal', { connected: false, reason: 'Not signed in' }),
      read('pro', { connected: false, reason: 'Not signed in' }),
    ];
    setLang('en');
    const reasons = Object.fromEntries(quotaRows(NOW).map((r) => [r.account.id, r.reasons]));
    expect(reasons.principal).toEqual(['Not signed in with a claude.ai account (API key?)']);
    expect(reasons.pro).toEqual(['Not signed in']);
    expect(reasons.team).toEqual(['Inactive']);
  });
});
