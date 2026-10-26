import { beforeEach, describe, expect, it } from 'vitest';
import { agent, project, resetApp, usageOf } from '../test/ipc';
import {
  dedicated,
  fProjection,
  newAgentAccount,
  overThreshold,
  pausedAccounts,
  pauseOf,
  projectedDayCost,
  projectedPct,
  projectionInBar,
  projectionLevel,
  projectionMode,
  projectionTip,
  projectsWithoutAccount,
  quotaOf,
  pickThreshold,
  quotaRows,
  sharedBy,
} from './accounts';
import { setLang } from './i18n';
import { app } from './state.svelte';
import type { Account, AccountUsage, AutopilotPause, RateWindow } from './types';

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true };
const TEAM: Account = { id: 'team', name: 'Équipe', configDir: 'C:\\claude\\team', claudePath: '', active: false };

const pause = (over: Partial<AutopilotPause> = {}): AutopilotPause => ({ reason: 'window', minutes: 300, pct: 97, until: 5_000, ...over });

describe('the accounts of a project', () => {
  beforeEach(() => {
    resetApp();
    app.settings.accounts = [PRINCIPAL, PRO, TEAM];
  });

  it('goes to the account the project prefers when it is active, else to the one new agents go to now', () => {
    app.usage.current = { claude: 'pro' };
    expect(newAgentAccount(project())?.id).toBe('pro');
    expect(newAgentAccount(project({ account: 'principal' }))?.id).toBe('principal');
    // Switched off, or gone: any account again.
    expect(newAgentAccount(project({ account: 'team' }))?.id).toBe('pro');
    expect(newAgentAccount(project({ account: 'parti' }))?.id).toBe('pro');
    expect(newAgentAccount(undefined)?.id).toBe('pro');
    // Before the backend told which: the first active one.
    app.usage.current = { claude: '' };
    expect(newAgentAccount(project())?.id).toBe('principal');
  });

  it('sends a private project’s new agents to its own accounts alone, past the threshold too, unless it allowed shared ones (ESC-141)', () => {
    const client = project({ id: 'client', private: true });
    app.projects = [client];
    app.settings.accounts = [PRINCIPAL, { ...PRO, projects: ['client'] }, TEAM];
    // The backend says Principal is where new agents go now (Pro past the threshold): not the client's.
    app.usage.current = { claude: 'principal' };
    expect(newAgentAccount(client)?.id).toBe('pro');
    expect(sharedBy(client)).toEqual([]);
    expect(dedicated({ projects: ['client'] }, 'client')).toBe(true);
    expect(dedicated({ projects: ['client', 'site'] }, 'client')).toBe(false);
    // Allowed: as any project.
    app.projects = [{ ...client, sharedOnLimit: true }];
    expect(newAgentAccount(app.projects[0])?.id).toBe('principal');
    expect(sharedBy(app.projects[0]).map((a) => a.id)).toEqual(['principal']);
    // Not private: nothing it shares is told.
    expect(sharedBy(project({ id: 'client' }))).toEqual([]);
  });

  it('never goes to an account reserved to other projects, and a project with accounts of its own waits for them', () => {
    app.settings.accounts = [PRINCIPAL, { ...PRO, projects: ['client'] }, TEAM];
    app.usage.current = { claude: 'principal' };
    expect(newAgentAccount(project({ id: 'other', account: 'pro' }))?.id).toBe('principal');
    expect(newAgentAccount(project({ id: 'client', account: 'pro' }))?.id).toBe('pro');
    app.autopilotPause = pause({ until: 1 });
    app.projectPauses = { client: pause({ until: 2 }) };
    expect(pauseOf(project({ id: 'client' }))?.until).toBe(2);
    expect(pauseOf(project({ id: 'other' }))?.until).toBe(1);
  });

  it('names the projects no active account serves: Principal may be reserved like any account', () => {
    app.projects = [project({ id: 'client', name: 'client' }), project({ id: 'other', name: 'autre' })];
    expect(projectsWithoutAccount()).toEqual([]);
    app.settings.accounts = [{ ...PRINCIPAL, projects: ['client'] }, { ...PRO, projects: ['client'] }, TEAM];
    expect(projectsWithoutAccount().map((p) => p.name)).toEqual(['autre']);
    expect(newAgentAccount(project({ id: 'other' }))).toBeUndefined();
    expect(newAgentAccount(project({ id: 'client' }))?.id).toBe('principal');
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
    // An agent archived, or on an account the window does not know any more, is counted as the backend does: the latter
    // for no account, never for Principal's (ESC-128: it gets another at its next start).
    give(waits('a1', 'principal', 9_000, { archived: true }), waits('a2', 'pro', 7_000));
    expect(quotaOf(any)).toBeNull();
    give(waits('a1', 'parti', 9_000), waits('a2', 'pro', 7_000));
    expect(quotaOf(any)).toBeNull();
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
  const read = (id: string, over: Parameters<typeof usageOf>[1] = {}): AccountUsage => usageOf(id, over);

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
    app.usage.current = { claude: 'pro' };
    expect(quotaRows(NOW).map((r) => [r.account.id, r.current])).toEqual([
      ['pro', true],
      ['principal', false],
      ['team', false],
    ]);
    app.usage.current = { claude: 'principal' };
    expect(quotaRows(NOW).map((r) => r.account.id)).toEqual(['principal', 'pro', 'team']);
    // Not told yet: the order of the settings, none marked.
    app.usage.current = { claude: '' };
    expect(quotaRows(NOW).map((r) => [r.account.id, r.current])).toEqual([
      ['principal', false],
      ['pro', false],
      ['team', false],
    ]);
  });

  it('gives each account the windows it was read with, and none before it was', () => {
    app.usage.accounts = [read('pro', { fiveHour: win(42), sevenDay: win(12) })];
    const rows = quotaRows(NOW);
    expect(rows.find((r) => r.account.id === 'pro')?.usage).toMatchObject({
      meters: [
        { minutes: 300, pct: 42 },
        { minutes: 10080, pct: 12 },
      ],
    });
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

  it('flags an account past « Changer de compte à » when it is lower than the pause (ESC-148)', () => {
    app.settings.quotaPause = 100;
    app.settings.switchAt = 95;
    app.usage.accounts = [read('principal', { fiveHour: win(96) }), read('pro', { fiveHour: win(94), sevenDay: win(95) })];
    expect(quotaRows(NOW).map((r) => [r.account.id, r.over])).toEqual([
      ['principal', true],
      ['pro', true],
      ['team', false],
    ]);
    expect(pickThreshold()).toBe(95);
    app.settings.quotaPause = 80;
    expect(pickThreshold()).toBe(80);
    // « À la limite »: the pause alone.
    app.settings.switchAt = 100;
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
      // On an account the settings no longer list: no account's, never Principal's (ESC-128); it runs no turn before it
      // gets another.
      a3: agent({ id: 'a3', account: 'parti' }),
      // Archived: its turn is still running, as the status bar counts it.
      a4: agent({ id: 'a4', account: 'pro', liveCost: 0.2, archived: true }),
    };
    const today = Object.fromEntries(quotaRows(NOW).map((r) => [r.account.id, r.today]));
    expect(today.principal.cost).toBeCloseTo(1.2);
    expect(today.principal.estimated).toBe(false);
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

describe('the projection of a window at its reset', () => {
  const H = 3600_000;
  const NOW = 1_000 * H;

  it('spreads what is used over the time gone, to the whole window', () => {
    // 2 h of 5 gone, 40 % used: 100 % at the reset.
    expect(projectedPct(300, { pct: 40, resetsAt: NOW + 3 * H }, NOW)).toBeCloseTo(100);
    // Half the week gone, 30 % used.
    expect(projectedPct(10080, { pct: 30, resetsAt: NOW + 84 * H }, NOW)).toBeCloseTo(60);
    expect(projectedPct(300, { pct: 0, resetsAt: NOW + 3 * H }, NOW)).toBe(0);
  });

  it('takes the month’s credits as a window of the month that ends at their reset', () => {
    // Back on November 1st, 00:00 UTC: October has 31 days; 10 days gone, 20 % used.
    const reset = Date.UTC(2026, 10, 1);
    expect(projectedPct(null, { pct: 20, resetsAt: reset }, Date.UTC(2026, 9, 11))).toBeCloseTo(62);
    // Half a day in: too early.
    expect(projectedPct(null, { pct: 1, resetsAt: reset }, Date.UTC(2026, 9, 1, 12))).toBeNull();
    // A reset on the 31st: the month before ends on the 30th (or the 28th), not on the 1st or 3rd of the next one.
    // From April 30th to May 31st: 31 days, 15 and a half gone, 25 % used.
    expect(projectedPct(null, { pct: 25, resetsAt: Date.UTC(2026, 4, 31) }, Date.UTC(2026, 4, 15, 12))).toBeCloseTo(50);
    // From February 28th to March 31st: 31 days.
    expect(projectedPct(null, { pct: 25, resetsAt: Date.UTC(2026, 2, 31) }, Date.UTC(2026, 2, 15, 12))).toBeCloseTo(50);
  });

  it('says nothing for an unknown or past end, nor too early: a tenth of a 5 h window, a whole day of a week', () => {
    expect(projectedPct(300, null, NOW)).toBeNull();
    expect(projectedPct(300, { pct: 40, resetsAt: null }, NOW)).toBeNull();
    expect(projectedPct(300, { pct: 40, resetsAt: NOW }, NOW)).toBeNull();
    // 29 minutes of 5 h gone: too early, a turn or two would take it far past 100 %; 30 minutes: a tenth.
    expect(projectedPct(300, { pct: 5, resetsAt: NOW + 271 * 60_000 }, NOW)).toBeNull();
    expect(projectedPct(300, { pct: 5, resetsAt: NOW + 270 * 60_000 }, NOW)).toBeCloseTo(50);
    // A week's: a morning's work (15 % in 8 h) would say « → 315 % » as if it went on day and night; nothing before a
    // whole day, then the pace of that day (20 % in 24 h: 140 %).
    expect(projectedPct(10080, { pct: 15, resetsAt: NOW + 160 * H }, NOW)).toBeNull();
    expect(projectedPct(10080, { pct: 20, resetsAt: NOW + 144 * H }, NOW)).toBeCloseTo(140);
  });

  it('takes a reset further than the window as a window just started', () => {
    expect(projectedPct(300, { pct: 5, resetsAt: NOW + 6 * H }, NOW)).toBeNull();
  });

  it('is green up to 80 %, amber up to 100 %, red past it, as it is written', () => {
    expect(projectionLevel(80)).toBe('ok');
    expect(projectionLevel(80.4)).toBe('ok');
    expect(projectionLevel(81)).toBe('warn');
    expect(projectionLevel(100.4)).toBe('warn');
    expect(projectionLevel(100.6)).toBe('over');
  });

  it('writes the percentage, « 999+ » past it', () => {
    setLang('fr');
    expect(fProjection(112.3)).toBe('112 %');
    expect(fProjection(999.4)).toBe('999 %');
    expect(fProjection(4000)).toBe('999+ %');
  });
});

describe('the projection of the day’s cost at midnight', () => {
  /** A time of day today, in the time zone of the machine. */
  const at = (h: number, m = 0) => new Date(2026, 9, 20, h, m).getTime();

  it('spreads the cost since midnight over the whole day', () => {
    expect(projectedDayCost(1, at(12))).toBeCloseTo(2);
    expect(projectedDayCost(3, at(18))).toBeCloseTo(4);
  });

  it('says nothing in the first tenth of the day (2 h 24)', () => {
    expect(projectedDayCost(0.2, at(2, 20))).toBeNull();
    expect(projectedDayCost(0.2, at(2, 30))).not.toBeNull();
  });
});

describe('« Projection des quotas »', () => {
  beforeEach(() => resetApp());

  it('shows past 80 % by default, always, or never, and reads the switch of before 2.0.2', () => {
    const mode = (v: unknown) => {
      (app.settings as { quotaProjection: unknown }).quotaProjection = v;
      return projectionMode();
    };
    expect(mode('alert')).toBe('alert');
    expect(mode('always')).toBe('always');
    expect(mode('off')).toBe('off');
    expect(mode(true)).toBe('alert');
    expect(mode(false)).toBe('off');
    expect(mode(undefined)).toBe('alert');
  });

  it('puts in the status bar the projections past 80 %, or every one when always', () => {
    app.settings.quotaProjection = 'alert';
    expect([projectionInBar('ok'), projectionInBar('warn'), projectionInBar('over')]).toEqual([false, true, true]);
    app.settings.quotaProjection = 'always';
    expect(projectionInBar('ok')).toBe(true);
  });

  it('says the projection after a tooltip, unless never or unknown', () => {
    setLang('fr');
    const H = 3600_000;
    const w = { pct: 20, resetsAt: 10 * H };
    expect(projectionTip(300, w, 8 * H)).toBe(' · projection à la remise à zéro : 33 %');
    expect(projectionTip(300, { pct: 20, resetsAt: null }, 8 * H)).toBe('');
    app.settings.quotaProjection = 'off';
    expect(projectionTip(300, w, 8 * H)).toBe('');
  });
});
