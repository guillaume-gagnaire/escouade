import { beforeEach, describe, expect, it } from 'vitest';
import { agent, project, resetApp } from '../test/ipc';
import { newAgentAccount, pausedAccounts, pauseOf, quotaOf } from './accounts';
import { setLang } from './i18n';
import { app } from './state.svelte';
import type { Account, AutopilotPause } from './types';

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
