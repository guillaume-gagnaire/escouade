// The accounts of every harness (ESC-30): grouped by harness, each harness's agents on its accounts alone, and a quota made
// of windows labelled by their length, of the month's credits, or of nothing.

import { beforeEach, describe, expect, it } from 'vitest';
import { agent, BARE_ENGINE, CLAUDE_ENGINE, project, resetApp, usageOf } from '../test/ipc';
import {
  accountEngines,
  engineOrder,
  isPrincipal,
  judge,
  meterLabel,
  meterName,
  metersShown,
  newAgentAccount,
  principalId,
  projectsWithoutAccount,
  quotaGroups,
  resumeTargets,
  statusEngine,
  usableFor,
} from './accounts';
import { pauseLabel } from './board';
import { setLang } from './i18n';
import { app } from './state.svelte';
import type { Account, EngineDecl } from './types';

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true };
const CODEX_MAIN: Account = { id: 'principal-codex', name: 'Principal', engine: 'codex', configDir: '', claudePath: '', active: true };
const WORK: Account = { id: 'work', name: 'Travail', engine: 'codex', configDir: 'C:\\codex\\work', claudePath: '', active: true };

/** The bare harness with a quota of windows: one of 5 hours, as Codex's free plan has. */
const WINDOWED: EngineDecl = { ...BARE_ENGINE, caps: { ...BARE_ENGINE.caps, quota: 'windows', quotaWindows: [300] } };
/** The bare harness with a quota of credits, as Copilot's. */
const CREDITS: EngineDecl = { ...BARE_ENGINE, caps: { ...BARE_ENGINE.caps, quota: 'credits', quotaWindows: [] } };

const NOW = 1_000_000;
const HOUR = 3_600_000;

describe('the accounts of each harness', () => {
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.settings.accounts = [PRINCIPAL, PRO, CODEX_MAIN, WORK];
  });

  it('are grouped by harness in the order of the harnesses, Claude Code’s first by default', () => {
    expect(accountEngines()).toEqual(['claude', 'codex']);
    app.settings.engineOrder = ['codex', 'claude'];
    expect(accountEngines()).toEqual(['codex', 'claude']);
    expect(quotaGroups(NOW).map((g) => [g.name, g.rows.map((r) => r.account.id)])).toEqual([
      ['Harness nu', ['principal-codex', 'work']],
      ['Claude Code', ['principal', 'pro']],
    ]);
    // A harness the order does not list yet comes after it; one with no account has no group.
    app.settings.engineOrder = [];
    app.settings.accounts = [PRINCIPAL, PRO];
    expect(engineOrder()).toEqual(['claude', 'codex']);
    expect(accountEngines()).toEqual(['claude']);
  });

  it('serve the agents of their harness alone: a new agent never goes to another harness’s account', () => {
    app.usage.current = { claude: 'pro', codex: 'work' };
    expect(usableFor(undefined).map((a) => a.id)).toEqual(['principal', 'pro']);
    expect(usableFor(undefined, 'codex').map((a) => a.id)).toEqual(['principal-codex', 'work']);
    expect(newAgentAccount(project())?.id).toBe('pro');
    expect(newAgentAccount(project(), 'codex')?.id).toBe('work');
    // A project on Codex goes to Codex's; the account it prefers counts only if it is one of Codex's.
    expect(newAgentAccount(project({ engine: 'codex' }))?.id).toBe('work');
    expect(newAgentAccount(project({ engine: 'codex', account: 'pro' }))?.id).toBe('work');
    expect(newAgentAccount(project({ engine: 'codex', account: 'principal-codex' }))?.id).toBe('principal-codex');
  });

  it('are reserved to projects as the Claude accounts are, Principal included', () => {
    app.settings.accounts = [PRINCIPAL, PRO, { ...CODEX_MAIN, projects: ['client'] }, { ...WORK, projects: ['client'] }];
    app.projects = [project({ id: 'client', name: 'client', engine: 'codex' }), project({ id: 'web', name: 'web', engine: 'codex' })];
    expect(usableFor('client', 'codex').map((a) => a.id)).toEqual(['principal-codex', 'work']);
    expect(usableFor('web', 'codex')).toEqual([]);
    // A project on Codex that no account of Codex serves starts nothing, though Claude Code's serve it.
    expect(projectsWithoutAccount().map((p) => p.id)).toEqual(['web']);
    expect(usableFor('web').map((a) => a.id)).toEqual(['principal', 'pro']);
  });

  it('go on, after a usage limit, on another account of the same harness only', () => {
    app.usage.accounts = [usageOf('pro'), usageOf('work', { engine: 'codex' })];
    expect(resumeTargets({ account: 'principal', projectId: 'p1' }).map((a) => a.id)).toEqual(['pro']);
    expect(resumeTargets({ account: 'principal-codex', projectId: 'p1', engine: 'codex' }).map((a) => a.id)).toEqual(['work']);
  });

  it('have their Principal by harness', () => {
    expect(principalId('claude')).toBe('principal');
    expect(principalId('codex')).toBe('principal-codex');
    expect(isPrincipal(CODEX_MAIN)).toBe(true);
    expect(isPrincipal({ id: 'principal', engine: 'codex' })).toBe(false);
    expect(isPrincipal(PRINCIPAL)).toBe(true);
  });

  it('show in the status bar the harness of the agent shown, else the default one of its project', () => {
    resetApp({ agents: [agent({ id: 'a1', engine: 'codex' })] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    expect(statusEngine()).toBe('codex');
    resetApp({ agents: [] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    expect(statusEngine()).toBe('claude');
    app.projects = [project({ engine: 'codex' })];
    expect(statusEngine()).toBe('codex');
  });
});

describe('the quota of an account', () => {
  beforeEach(() => {
    resetApp();
    app.now = NOW;
  });

  it('labels each window by its length, and the month’s credits', () => {
    expect([300, 10080, 43200, 60].map(meterLabel)).toEqual(['5 h', '7 j', '30 j', '1 h']);
    expect(meterLabel(null)).toBe('crédits');
    expect(meterName(300)).toBe('Quota sur 5 heures');
    expect(meterName(1440)).toBe('Quota sur 1 jour');
    expect(meterName(null)).toBe('Crédits du mois');
    setLang('en');
    expect([300, 10080, 43200].map(meterLabel)).toEqual(['5h', 'W', '30d']);
    expect(meterName(10080)).toBe('7-day quota');
    expect(meterLabel(null)).toBe('credits');
  });

  it('shows the windows read, else those the harness declares, its credits, or nothing without a quota', () => {
    app.engines = [CLAUDE_ENGINE, WINDOWED];
    // Not read yet: Claude Code's two windows, empty.
    expect(metersShown(null, 'claude')).toEqual([
      { minutes: 300, usage: null },
      { minutes: 10080, usage: null },
    ]);
    // Read: those the reading gave, a single one on a plan that has one.
    const one = usageOf('work', { engine: 'codex', meters: [{ minutes: 300, pct: 40, resetsAt: NOW + HOUR }] });
    expect(metersShown(one, 'codex').map((m) => m.minutes)).toEqual([300]);
    app.engines = [CLAUDE_ENGINE, CREDITS];
    expect(metersShown(null, 'codex')).toEqual([{ minutes: null, usage: null }]);
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    expect(metersShown(null, 'codex')).toEqual([]);
  });

  it('is out by any of its meters, credits included; never without one', () => {
    const credits = usageOf('x', { meters: [{ minutes: null, pct: 100, resetsAt: NOW + HOUR }] });
    expect(judge(credits.meters, 95, NOW)).toMatchObject({ standing: 'out', leave: true });
    // The month's credits are a long window judged by their pace: 94 % an hour before the reset is on pace, past the threshold or not.
    const early = usageOf('x', { meters: [{ minutes: null, pct: 94, resetsAt: NOW + HOUR }] });
    expect(judge(early.meters, 90, NOW)).toMatchObject({ standing: 'good', leave: false });
    // 96 %, on pace too: near its limit (NEAR_LIMIT), behind the others; an agent at the threshold leaves.
    const near = usageOf('x', { meters: [{ minutes: null, pct: 96, resetsAt: NOW + HOUR }] });
    expect(judge(near.meters, 95, NOW)).toEqual({ standing: 'behind', leave: true, near: true });
    expect(judge(usageOf('none').meters, 0, NOW)).toMatchObject({ standing: 'good', leave: false });
  });

  it('pauses the autopilot by a window named by its length, or by the credits', () => {
    const until = new Date(2026, 9, 10, 14, 0).getTime();
    const now = new Date(2026, 9, 10, 12, 0).getTime();
    expect(pauseLabel({ reason: 'window', minutes: 43200, pct: 96, until }, now)).toBe(
      'Pilote auto en pause : quota de 30 jours à 96 % (reprise à 14:00)',
    );
    expect(pauseLabel({ reason: 'credits', pct: 100, until }, now)).toBe(
      'Pilote auto en pause : crédits du mois à 100 % (reprise à 14:00)',
    );
    setLang('en');
    expect(pauseLabel({ reason: 'credits', pct: 100, until }, now)).toBe('Autopilot paused: monthly credits at 100% (resumes at 2:00 PM)');
  });
});
