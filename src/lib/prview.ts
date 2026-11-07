// The side panel's « PR et CI » tab (ESC-305): what it says of a pull request's checks, of the host's errors, and which
// agent a pull request of the project is. The component reads; these say.

import { fDur, fTime } from './format';
import { t } from './i18n';
import type { IconName } from './icons';
import { ownBranch } from './prs';
import type { Agent, PrCheck, PrFailure, PullRequest } from './types';

/** How often the checks are read again while one runs and the tab is shown: the rhythm of the agents' pull requests. */
export const POLL_MS = 20_000;

/** A check in one of the words the tab says. */
export type CheckPhase = 'queued' | 'running' | 'success' | 'failure' | 'cancelled' | 'skipped';

export function checkPhase(c: PrCheck): CheckPhase {
  if (c.cancelled) return 'cancelled';
  if (c.state === 'pending') return c.started ? 'running' : 'queued';
  return c.state;
}

/** Its glyph, as the card's numbers draw a pull request's CI. */
export const phaseIcon: Record<CheckPhase, IconName> = {
  queued: 'clock',
  running: 'clock',
  success: 'check',
  failure: 'close',
  cancelled: 'stop',
  skipped: 'circle',
};

/** The order of the list: what to look at first. */
const RANK: Record<CheckPhase, number> = { failure: 0, running: 1, queued: 2, cancelled: 3, success: 4, skipped: 5 };

export const sortChecks = (checks: PrCheck[]) => [...checks].sort((a, b) => RANK[checkPhase(a)] - RANK[checkPhase(b)]);

/** How long it ran (till `now` ms while it runs); none when the host does not say when it started. */
export function checkDuration(c: PrCheck, now: number): string | null {
  if (!c.started) return null;
  const end = c.finished ?? (c.state === 'pending' ? Math.floor(now / 1000) : null);
  return end === null ? null : fDur((end - c.started) * 1000);
}

/** A check still runs or waits: the tab reads them again. */
export const stillRunning = (checks: PrCheck[]) => checks.some((c) => c.state === 'pending' && !c.cancelled);

/** A check failed or was cancelled: « Relancer les échecs » has something to run again (the host runs both). */
export const hasFailed = (checks: PrCheck[]) => checks.some((c) => c.state === 'failure');

/** A check failed, not only cancelled: the summary says it in the colour of a failure. */
export const reallyFailed = (checks: PrCheck[]) => checks.some((c) => checkPhase(c) === 'failure');

/** The summary at the head of the CI: « 2 échecs sur 14 », « 3 en cours sur 14 », « 14 vérifications réussies ». */
export function ciSummary(checks: PrCheck[]): string {
  const total = checks.length;
  const failed = checks.filter((c) => checkPhase(c) === 'failure').length;
  if (failed) return t('prs.view.ci.failed', { count: failed, total });
  const running = checks.filter((c) => c.state === 'pending' && !c.cancelled).length;
  if (running) return t('prs.view.ci.running', { count: running, total });
  const passed = t('prs.view.ci.passed', { count: checks.filter((c) => c.state === 'success').length });
  const cancelled = checks.filter((c) => c.cancelled).length;
  return cancelled ? `${passed} · ${t('prs.view.ci.cancelled', { count: cancelled })}` : passed;
}

/** What the host's error says to the user: its kind in words, the host's own words for the others. */
export function failureTitle(f: PrFailure): string {
  if (f.kind === 'refused') return t('prs.view.failure.refused');
  if (f.kind === 'rateLimit')
    return f.reset ? t('prs.view.failure.rateLimitUntil', { time: fTime(f.reset * 1000) }) : t('prs.view.failure.rateLimit');
  if (f.kind === 'unreachable') return t('prs.view.failure.unreachable');
  return f.message;
}

/** The agent of the project whose pull request it is: one that read it, else one whose branch it comes from. */
export function agentOfPr(agents: Agent[], p: PullRequest): Agent | null {
  const live = agents.filter((a) => !a.archived);
  return (
    live.find((a) => a.prs.some((q) => q.number === p.number && !q.hostRepo)) ??
    live.find((a) => !p.fork && ownBranch(a) === p.head) ??
    null
  );
}

/** The CI of the agent's first pull request failed while it is open: the tab's dot. */
export const ciFailed = (a: Agent | null) => {
  const p = a?.prs[0];
  return !!p && p.state === 'open' && p.checks === 'failure';
};

/** The comments people wrote, the open threads first. */
export const sortComments = <T extends { resolved: boolean }>(comments: T[]) =>
  [...comments].sort((a, b) => Number(a.resolved) - Number(b.resolved));

/** The states a host tells of a deployment (GitHub's), in words; another one as the host says it. */
const DEPLOY_STATES = ['success', 'failure', 'error', 'pending', 'queued', 'in_progress', 'inactive'] as const;

export function deployState(state: string): string {
  const known = DEPLOY_STATES.find((s) => s === state);
  return known ? t(`prs.view.deployments.state.${known === 'in_progress' ? 'inProgress' : known}`) : state;
}
