// The accounts of every harness as the window shows them: each one runs a single harness, and its agents never go to
// another harness's account (going on another harness is a handover, ESC-88).

import { quotaUntil } from './board';
import { capsOf, engineName } from './engines';
import { fDayMonth, fList, fPct, fTime } from './format';
import { estimatedBy } from './spend';
import { t, tIn } from './i18n';
import { app } from './state.svelte';
import type { SettingsTab } from './settings.svelte';
import type { Account, AccountUsage, Agent, AutopilotPause, EngineId, Project, QuotaMeter, QuotaProjection, RateWindow } from './types';

/** The id of the user's own Claude Code account. */
export const PRINCIPAL = 'principal';

/** The account's harness: Claude Code for one saved before 1.8. */
export function engineOfAccount(a: Pick<Account, 'engine'>): EngineId {
  return a.engine ?? 'claude';
}

/** The ids of the Principals that are not `principal-<harness>`: Claude Code's, the only one before 1.8. */
const PRINCIPAL_IDS: Partial<Record<EngineId, string>> = { claude: PRINCIPAL };

/** The id of the user's own account of `engine` (`accounts::principal_id`). */
export function principalId(engine: EngineId): string {
  return PRINCIPAL_IDS[engine] ?? `${PRINCIPAL}-${engine}`;
}

/** Whether the account is the user's own of its harness, launched without a folder of Escouade's. */
export function isPrincipal(a: Pick<Account, 'id' | 'engine'>): boolean {
  return a.id === principalId(engineOfAccount(a));
}

/** The harness of what says none (the backend lists it first). */
export function defaultEngine(): EngineId {
  return app.engines[0]?.id ?? 'claude';
}

/**
 * The harnesses Escouade knows (declared, or with an account) in « Ordre des harness » (`Settings.engineOrder`), those it
 * does not list after: the order of the groups of « Comptes ».
 */
export function engineOrder(): EngineId[] {
  const known = new Set<EngineId>([...app.engines.map((e) => e.id), ...(app.settings.accounts ?? []).map(engineOfAccount)]);
  const out: EngineId[] = [];
  const add = (e: EngineId) => {
    if (known.has(e) && !out.includes(e)) out.push(e);
  };
  (app.settings.engineOrder ?? []).forEach(add);
  known.forEach(add);
  return out;
}

/** The harnesses that have accounts, in their order: one group each in the panel of the quotas. */
export function accountEngines(): EngineId[] {
  const have = new Set((app.settings.accounts ?? []).map(engineOfAccount));
  return engineOrder().filter((e) => have.has(e));
}

/**
 * The groups of « Comptes »: every harness installed (its program found), with « Aucun compte » while it has none, and
 * those with accounts, in their order (ESC-128).
 */
export function accountsTabEngines(): EngineId[] {
  const have = new Set((app.settings.accounts ?? []).map(engineOfAccount));
  return engineOrder().filter((e) => have.has(e) || app.enginesFound.includes(e));
}

/** Whether the harness has an account at all: without one, nothing starts on it (ESC-128). */
export function hasAccount(engine: EngineId): boolean {
  return (app.settings.accounts ?? []).some((a) => engineOfAccount(a) === engine);
}

/** Claude Code's model of new agents, until an account says another (`accounts::CLAUDE_DEFAULT_MODEL`). */
export const CLAUDE_DEFAULT_MODEL = 'sonnet';

/** What a new agent goes with unless it says its own (`accounts::defaults`). */
export interface AgentDefaults {
  model: string;
  effort: string;
  mode: string;
}

/**
 * The account's model, effort and permission mode for its new agents, else its harness's (`engine`, without an account):
 * Claude Code's Sonnet, effort `medium` and mode `auto` (« Demander » for a harness without it, as the backend has it); no model
 * for another harness, which runs its own default.
 */
export function accountDefaults(a: Account | undefined, engine: EngineId = a ? engineOfAccount(a) : defaultEngine()): AgentDefaults {
  const own = (v: string | undefined) => v?.trim() || undefined;
  const of = a ? engineOfAccount(a) : engine;
  return {
    model: own(a?.defaultModel) ?? (of === 'claude' ? CLAUDE_DEFAULT_MODEL : ''),
    effort: own(a?.defaultEffort) ?? 'medium',
    mode: own(a?.defaultMode) ?? (capsOf(of)?.modes.some((m) => m.id === 'auto') === false ? 'default' : 'auto'),
  };
}

/** The defaults of the account a new agent of the project on `engine` would go to (`newAgentAccount`), else of its harness. */
export function newAgentDefaults(project: ProjectLike | undefined, engine?: EngineId): AgentDefaults {
  const e = engine ?? engineOfProject(project);
  return accountDefaults(newAgentAccount(project, e), e);
}

/** « Ajouter un compte… » on `engine` (Réglages › Comptes, back there when it closes). */
export function addAccount(engine: EngineId) {
  app.modal = { kind: 'account', engine };
}

/** The harness whose accounts the status bar shows: the one of the agent shown, else the project's default one. */
export function statusEngine(): EngineId {
  return app.agent?.engine ?? app.project?.engine ?? defaultEngine();
}

/**
 * The files a new account may share of its Principal's folder, Claude Code's, Codex's then Copilot CLI's (`accounts::SHARED_FILES`,
 * `CODEX_SHARED_FILES`, `COPILOT_SHARED_FILES`): the other items are folders.
 */
export const SHARED_FILES = ['settings.json', 'CLAUDE.md', 'config.toml', 'AGENTS.md', 'mcp-config.json', 'copilot-instructions.md'];

/**
 * The account's name as shown. Principal is named in the language of its first save: while the user has not renamed it,
 * it is shown in the interface's.
 */
export function accountName(a: Pick<Account, 'id' | 'name' | 'engine'>): string {
  const name = a.name.trim();
  const defaults = [tIn('fr', 'accounts.principal'), tIn('en', 'accounts.principal')];
  return isPrincipal(a) && (!name || defaults.includes(name)) ? t('accounts.principal') : a.name;
}

/** The agents not archived that run on the account: while there are some, it is not removed. */
export function agentsOn(id: string): number {
  return Object.values(app.agents).filter((a) => a.account === id && !a.archived).length;
}

/** Whether a new agent of the project (none: of no project) may go to the account: it is not reserved to other projects. */
export function serves(a: Pick<Account, 'projects'>, project: string | undefined): boolean {
  const reserved = a.projects ?? [];
  return !reserved.length || (!!project && reserved.includes(project));
}

/** The accounts of `engine` (by default the default harness's), in the order of the settings. */
export function accountsOf(engine: EngineId = defaultEngine()): Account[] {
  return (app.settings.accounts ?? []).filter((a) => engineOfAccount(a) === engine);
}

/** The active accounts of `engine` that serve new agents of the project (none: of no project), in order (`accounts::for_project`). */
function servingFor(project: string | undefined, engine: EngineId): Account[] {
  return (app.settings.accounts ?? []).filter((a) => a.active && engineOfAccount(a) === engine && serves(a, project));
}

/** Whether the account is reserved to the project alone: the only ones a private project shares with no other (`accounts::dedicated`). */
export function dedicated(a: Pick<Account, 'projects'>, project: string): boolean {
  const reserved = a.projects ?? [];
  return reserved.length === 1 && reserved[0] === project;
}

/** A project as `accountsFor` reads it: a draft's fields will do. */
export type PrivateLike = Pick<Project, 'id' | 'account'> & { private?: boolean; sharedOnLimit?: boolean };

/**
 * The active accounts of `engine` a new agent of the project may go to, in order (`accounts::for_project_of`): a private
 * project that did not allow shared ones at the limit (« Comptes partagés à la limite ») and has accounts of its own goes
 * to those alone, and to the one it prefers.
 */
export function accountsFor(project: PrivateLike, engine: EngineId = defaultEngine()): Account[] {
  const all = servingFor(project.id, engine);
  if (!project.private || project.sharedOnLimit || !all.some((a) => dedicated(a, project.id))) return all;
  return all.filter((a) => dedicated(a, project.id) || a.id === project.account);
}

/**
 * The accounts of `engine` a private project's agents may go to that it shares with other projects: there, sessions,
 * memory and history of the harness are theirs too. None for a project that is not private.
 */
export function sharedBy(project: PrivateLike, engine: EngineId = defaultEngine()): Account[] {
  return project.private ? accountsFor(project, engine).filter((a) => !dedicated(a, project.id)) : [];
}

/** The private projects whose agents may go to the account although it is not theirs alone (`sharedBy`). */
export function privateSharing(a: Account): Project[] {
  return app.projects.filter((p) => p.private && sharedBy(p, engineOfAccount(a)).some((x) => x.id === a.id));
}

/** The active accounts of `engine` a new agent of the project may go to, in order (`accountsFor`; none: of no project). */
export function usableFor(project: string | undefined, engine: EngineId = defaultEngine()): Account[] {
  const p = project ? app.projects.find((x) => x.id === project) : undefined;
  return p ? accountsFor(p, engine) : servingFor(project, engine);
}

/** The harness a project's new agents and tickets run on unless they say theirs. */
function engineOfProject(project: { engine?: EngineId | null } | undefined): EngineId {
  return project?.engine ?? defaultEngine();
}

/**
 * The projects no active account of their default harness serves (each reserved to others, its Principal too, or switched
 * off): nothing starts for them there.
 */
export function projectsWithoutAccount(): Project[] {
  return app.projects.filter((p) => !usableFor(p.id, engineOfProject(p)).length);
}

type ProjectLike = Pick<Project, 'account'> & { id?: string; engine?: EngineId | null };

/**
 * The active account the project prefers (« Compte préféré »), if it names one of `engine`: a switched off, reserved,
 * unknown one, or one of another harness, is none.
 */
function preferred(project: ProjectLike | undefined, engine: EngineId = engineOfProject(project)): Account | undefined {
  const id = project?.account;
  return id ? usableFor(project?.id, engine).find((a) => a.id === id) : undefined;
}

/** The project's tickets wait for accounts of their own: the one it prefers, or the set that reserved accounts make its own. */
function ownAccounts(project: Pick<Project, 'id' | 'account'> & { engine?: EngineId | null }): boolean {
  const engine = engineOfProject(project);
  const ids = (p: string | undefined, e: EngineId) =>
    usableFor(p, e)
      .map((a) => a.id)
      .join(' ');
  return !!preferred(project) || ids(project.id, engine) !== ids(undefined, defaultEngine());
}

/**
 * The account a new agent of the project on `engine` (by default the project's) would go to now (« Automatique »): the one
 * it prefers, else the current account of that harness (the first active one under the pause threshold, as the backend
 * says), else its first active one.
 */
export function newAgentAccount(project: ProjectLike | undefined, engine: EngineId = engineOfProject(project)): Account | undefined {
  const usable = usableFor(project?.id, engine);
  return preferred(project, engine) ?? usable.find((a) => a.id === app.usage.current[engine]) ?? usable[0];
}

/** Why no ticket of the project starts: the pause of its preferred account alone, else of the accounts new agents may go to. */
export function pauseOf(project: Pick<Project, 'id' | 'account'>): AutopilotPause | null {
  return ownAccounts(project) ? (app.projectPauses[project.id] ?? null) : app.autopilotPause;
}

/** The agent's account (one the window does not know any more is its own still: it gets another at its next start). */
function accountOf(a: Pick<Agent, 'account'>): string {
  return a.account;
}

/**
 * When the first agent waiting for its quota resumes, if every account a ticket of the project could start on has one:
 * nothing starts before. While one has none, a ticket starts on it.
 */
export function quotaOf(project: ProjectLike): number | null {
  const pick = preferred(project);
  const ids = pick ? [pick.id] : usableFor(project?.id, engineOfProject(project)).map((a) => a.id);
  const agents = Object.values(app.agents);
  const times = ids.map((id) => quotaUntil(agents.filter((a) => accountOf(a) === id)));
  return ids.length && times.every((x) => x !== null) ? Math.min(...(times as number[])) : null;
}

/** The use past which an account takes no new agent while another can: the lower of « Pause au-delà du quota » and « Changer de compte à » (ESC-148), as the backend's `board::pick_threshold`. */
export function pickThreshold(): number {
  return Math.min(app.settings.quotaPause, app.settings.switchAt ?? 100);
}

/**
 * The accounts an agent stopped by the usage limit can go on on (« Reprendre sur <compte> »), in the order of the settings:
 * the active ones of its harness other than its own that are signed in and that an agent would not leave at the threshold
 * (`pickThreshold`, `judge`). One at it is left out: the agent would meet the limit again there. Another harness's never is (ESC-88).
 */
export function resumeTargets(agent: Pick<Agent, 'account'> & { projectId?: string; engine?: EngineId }): Account[] {
  const own = accountOf(agent);
  const engine = agent.engine ?? 'claude';
  const threshold = pickThreshold();
  return app.settings.accounts.filter((a) => {
    if (!a.active || a.id === own || engineOfAccount(a) !== engine || !serves(a, agent.projectId)) return false;
    const read = app.usage.accounts.find((u) => u.id === a.id);
    // An account not read yet counts as signed in, as the backend has it.
    if (read && !read.connected) return false;
    return !judge(read?.meters ?? [], threshold, app.now).leave;
  });
}

const DAY = 24 * 60;

/** A quota meter's label, by its length: « 5h », « 7j » (« 7d »), « 30j »; « crédits » for the month's credits. */
export function meterLabel(minutes: number | null): string {
  if (minutes === null) return t('accounts.quota.credits.label');
  if (minutes === 7 * DAY) return t('accounts.quota.week.label');
  if (minutes % DAY === 0) return t('accounts.quota.days.label', { n: minutes / DAY });
  return t('accounts.quota.hours.label', { n: Math.round(minutes / 60) });
}

/** The name of a quota bar, for the screen readers: « Quota sur 5 heures », « Crédits du mois ». */
export function meterName(minutes: number | null): string {
  if (minutes === null) return t('accounts.quota.credits.name');
  if (minutes % DAY === 0) return t('accounts.quota.days.name', { count: minutes / DAY });
  return t('accounts.quota.hours.name', { count: Math.round(minutes / 60) });
}

/** One bar of an account's quota: its length (null: the month's credits) and its reading, null until it is read. */
export interface MeterShown {
  minutes: number | null;
  usage: RateWindow | null;
}

/**
 * The bars of an account of `engine`, as its harness has them: those read, else, before a reading, the windows it declares
 * (`EngineCaps.quotaWindows`) or its credits, empty; none for a harness without a quota.
 */
export function metersShown(usage: AccountUsage | null, engine: EngineId): MeterShown[] {
  const caps = app.engines.find((e) => e.id === engine)?.caps;
  if (caps?.quota === 'none') return [];
  const read = usage?.meters ?? [];
  if (read.length) return read.map((m) => ({ minutes: m.minutes, usage: m }));
  if (caps?.quota === 'credits') return [{ minutes: null, usage: null }];
  return (caps?.quotaWindows ?? []).map((minutes) => ({ minutes, usage: null }));
}

/** A quota window as its tooltip says it: « 42 % · remise à zéro le 10/10 à 18:00 ». Also the value of its bar for a screen reader. */
export function quotaTip(w: RateWindow | null): string {
  if (!w) return t('accounts.quota.unavailable');
  if (!w.resetsAt) return fPct(w.pct);
  return t('accounts.quota.tip', { pct: fPct(w.pct), date: fDayMonth(w.resetsAt), time: fTime(w.resetsAt) });
}

/** « Les comptes Principal et Pro ont passé le seuil. », under the pause: nothing after a usage limit, nor with a single account. */
export function pausedAccounts(p: AutopilotPause): string {
  if (p.reason === 'limit' || !p.accounts?.length) return '';
  const names = p.accounts.map((id) => {
    const a = app.settings.accounts.find((x) => x.id === id);
    return a ? accountName(a) : id;
  });
  return t('accounts.pause.over', { count: names.length, accounts: fList(names) });
}

/**
 * A window used `threshold` percent or more, until an end still to come: it holds the tickets back (`board::over_threshold`).
 * One whose end is unknown holds nothing back: a reading that no longer comes would hold them forever.
 */
export function overThreshold(w: RateWindow | null, threshold: number, now: number): boolean {
  // Claude Code gives a fraction, times 100 there: 0.9 may come as 89.99999….
  return !!w && w.pct + 1e-6 >= threshold && w.resetsAt !== null && w.resetsAt > now;
}

/**
 * Where a window would end if it went on at the pace used so far: its percentage spread over the time gone, to its whole
 * length. The month's credits (`minutes` null, Copilot CLI) are a window of the month that ends at their reset. None for a
 * window whose end is unknown or past, nor while it is too early (`tooEarly`).
 */
export function projectedPct(minutes: number | null, w: RateWindow | null, now: number): number | null {
  if (!w?.resetsAt) return null;
  const length = minutes === null ? w.resetsAt - monthBefore(w.resetsAt) : minutes * 60_000;
  const left = w.resetsAt - now;
  if (left <= 0) return null;
  const gone = Math.min(length, length - left);
  if (tooEarly(gone, length)) return null;
  return (w.pct * length) / gone;
}

/** How an account stands for the choice of the account (PR #20), as the backend's `pace::Standing`: good, behind (only its pace is bad) or out. */
export type Standing = 'good' | 'behind' | 'out';

/** The use past which a long window is never good, on pace or not, as the backend's `pace::NEAR_LIMIT`. */
export const NEAR_LIMIT = 95;

/** What an account's meters say of it, as the backend's `pace::Verdict`. */
export interface Verdict {
  /** The worst of its meters (good with none: a harness without a quota keeps the order alone). */
  standing: Standing;
  /** An agent running on the account leaves it: a short window at the threshold, or a long one at the threshold whose pace is not good. */
  leave: boolean;
  /** Behind for being near its limit (`NEAR_LIMIT`), not for its pace. */
  near: boolean;
}

/** Whether a window is a short pause (a day or less: the 5 hours); the month's credits (no length) and the longer windows are judged by their pace. */
export function isShortWindow(minutes: number | null): boolean {
  return minutes !== null && minutes <= DAY;
}

const WORST: Record<Standing, number> = { good: 0, behind: 1, out: 2 };

/**
 * The verdict on an account's meters at `now`, `threshold` being the percentage at which a short window (or a long one without
 * a projection) is out, as the backend's `pace::judge` (the same figures, tests/fixtures/quota-pace.json). A short window at the
 * threshold is out until it starts again; a long one is out at 100 %, good when projected at 100 % or less even past the
 * threshold, behind when projected past it even under the threshold, and goes by the threshold without a projection.
 */
export function judge(meters: QuotaMeter[], threshold: number, now: number): Verdict {
  let standing: Standing = 'good';
  let leave = false;
  let near = false;
  for (const m of meters) {
    const at = overThreshold(m, threshold, now);
    let s: Standing = 'good';
    if (isShortWindow(m.minutes)) {
      if (at) s = 'out';
      leave ||= at;
    } else if (overThreshold(m, 100, now)) {
      s = 'out';
      leave = true;
    } else {
      const p = projectedPct(m.minutes, m, now);
      const byPace = p !== null && p > 100;
      const nearLimit = !byPace && !(p === null && at) && overThreshold(m, NEAR_LIMIT, now);
      if (p === null && at) s = 'out';
      else if (byPace || nearLimit) s = 'behind';
      near ||= nearLimit;
      // At the threshold with a pace that is not good (past 100 % or not known), or near its limit.
      leave ||= at && (p === null || byPace || nearLimit);
    }
    if (WORST[s] > WORST[standing]) standing = s;
  }
  return { standing, leave, near };
}

const DAY_MS = 24 * 3_600_000;

/**
 * Too early to project, `gone` into a window of `length`. One of a day or so waits for a tenth of itself (30 minutes of a
 * 5 h window, a turn or two would take it far past 100 %; 2 h 24 of a day); a longer one (a week's, a month's) for a whole
 * day, for the nights to count: a morning's work spread over a week as if it went on day and night would make the start
 * of every week red.
 */
function tooEarly(gone: number, length: number): boolean {
  return gone < (length < 2 * DAY_MS ? length * 0.1 : DAY_MS);
}

/**
 * The same day and time a month before `ts`, in UTC (the credits come back on the 1st, 00:00 UTC); the last day of that
 * month when it is shorter (March 31st → February 28th, not March 3rd).
 */
function monthBefore(ts: number): number {
  const d = new Date(ts);
  const day = d.getUTCDate();
  d.setUTCDate(1);
  d.setUTCMonth(d.getUTCMonth() - 1);
  const last = new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth() + 1, 0)).getUTCDate();
  d.setUTCDate(Math.min(day, last));
  return d.getTime();
}

/**
 * Where the day's cost would end at midnight at the pace since midnight, for a harness without a quota (OpenCode): null
 * while it is too early (`tooEarly`: until 2 h 24), as for a window.
 */
export function projectedDayCost(cost: number, now: number): number | null {
  const midnight = new Date(now);
  midnight.setHours(0, 0, 0, 0);
  const next = new Date(midnight);
  next.setDate(next.getDate() + 1);
  const length = next.getTime() - midnight.getTime();
  const gone = now - midnight.getTime();
  if (tooEarly(gone, length)) return null;
  return (cost * length) / gone;
}

/**
 * The colour of a projection: past 100 % the window runs out before its end, past 80 % it comes close. Rounded as it is
 * written, so that « 100 % » is never red.
 */
export function projectionLevel(pct: number): 'ok' | 'warn' | 'over' {
  const p = Math.round(pct);
  if (p > 100) return 'over';
  if (p > 80) return 'warn';
  return 'ok';
}

/** « Projection des quotas » as the settings have it; the switch of before 2.0.2 too (on: `alert`, off: `off`). */
export function projectionMode(): QuotaProjection {
  const v: unknown = app.settings.quotaProjection;
  if (v === false || v === 'off') return 'off';
  return v === 'always' ? 'always' : 'alert';
}

/**
 * A projection of this level is written in the status bar itself: past 80 % (choice 21 of the 2.0 design), or every one
 * when « Toujours » is chosen. The panel and the tooltips write every one; under 1240 pixels the bar writes none, the
 * tooltips still do.
 */
export function projectionInBar(level: 'ok' | 'warn' | 'over'): boolean {
  return projectionMode() === 'always' || level !== 'ok';
}

/** « · projection à la remise à zéro : 64 % », after a window's tooltip: nothing when none is known or « Jamais ». */
export function projectionTip(minutes: number | null, w: RateWindow | null, now: number): string {
  const p = projectionMode() === 'off' ? null : projectedPct(minutes, w, now);
  return p === null ? '' : ` · ${t('accounts.quota.projectedTip', { pct: fProjection(p) })}`;
}

/** A projection as it is written: « 112 % », « 999+ % » past that (early in a window, a few turns go far). */
export function fProjection(pct: number): string {
  return Math.round(pct) > 999 ? t('format.percent', { n: '999+' }) : fPct(pct);
}

/** One account in the panel of the quotas. */
export interface QuotaRow {
  account: Account;
  /** What its last reading gave; null before it was read. */
  usage: AccountUsage | null;
  /** The one new agents go to, and whose quota the status bar shows. */
  current: boolean;
  /** Active, and out for the choice of the account: a window at the threshold of « Pause au-delà du quota » or « Changer de compte à » (the 5 h), or at its limit (`judge`). */
  over: boolean;
  /** Active, and behind the others only by its pace: a long window that would not last to its reset (`judge`). */
  behind: boolean;
  /** Behind the others for a long window near its limit (`NEAR_LIMIT`), not for its pace. */
  nearLimit: boolean;
  /** Why it is dimmed: switched off, not signed in, sign-in expired. */
  reasons: string[];
  /** What its turns cost today, the ones running now included (an estimate): the status bar's total, account by account. */
  today: { cost: number; estimated: boolean; byHarness?: boolean };
}

/** What the turns running on the account's agents cost so far: archived agents too, as the status bar counts them. */
function liveCostOf(id: string): number {
  return Object.values(app.agents)
    .filter((a) => accountOf(a) === id)
    .reduce((sum, a) => sum + (a.liveCost ?? 0), 0);
}

/** The accounts of one harness in the panel of the quotas and in « Comptes ». */
export interface QuotaGroup {
  engine: EngineId;
  /** Its name, as the interface says it. */
  name: string;
  rows: QuotaRow[];
}

/** Every account for the panel of the quotas, by harness in their order: in each, the current one first, the others in the order of the settings. */
export function quotaGroups(now: number): QuotaGroup[] {
  const rows = quotaRows(now);
  return accountEngines().map((engine) => ({
    engine,
    name: engineName(engine),
    rows: rows.filter((r) => engineOfAccount(r.account) === engine),
  }));
}

/** Every account for the panel of the quotas: the current ones (one per harness) first, the others in the order of the settings. */
export function quotaRows(now: number): QuotaRow[] {
  const rows = app.settings.accounts.map((account): QuotaRow => {
    const usage = app.usage.accounts.find((u) => u.id === account.id) ?? null;
    const reasons: string[] = [];
    if (!account.active) reasons.push(t('accounts.tab.state.inactive'));
    if (usage && !usage.connected) {
      // Main with no claude.ai sign-in is most likely run with an API key: the tab of the accounts says so.
      reasons.push(
        account.id === PRINCIPAL ? t('accounts.tab.state.principalNotConnected') : (usage.reason ?? t('accounts.tab.state.notConnected')),
      );
    } else if (usage?.reason) {
      reasons.push(usage.reason);
    }
    const threshold = pickThreshold();
    const verdict = account.active ? judge(usage?.meters ?? [], threshold, now) : null;
    const over = verdict?.standing === 'out';
    const behind = verdict?.standing === 'behind';
    const nearLimit = behind && !!verdict?.near;
    const live = liveCostOf(account.id);
    const byHarness = estimatedBy(engineOfAccount(account));
    const today = { cost: (usage?.todayCost ?? 0) + live, estimated: byHarness || live > 0, ...(byHarness ? { byHarness } : {}) };
    const current = account.id === app.usage.current[engineOfAccount(account)];
    return { account, usage, current, over, behind, nearLimit, reasons, today };
  });
  return [...rows.filter((r) => r.current), ...rows.filter((r) => !r.current)];
}

/** Back to the settings' tab `tab` (« Comptes » by default), their draft as it was. */
export function backToAccounts(tab: SettingsTab = 'accounts') {
  app.modal = { kind: 'settings', tab, resume: true };
}
