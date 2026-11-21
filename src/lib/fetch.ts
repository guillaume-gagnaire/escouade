// The background fetch (ESC-561): its bounds and proposed values, how its durations are written, what a project's
// own values make of the general setting (as the backend's `ProjectFetch::plan`), and what the Git panel says of it.

import { fAgo, fWhen } from './format';
import { t } from './i18n';
import type { FetchFailure, GitFetchSettings, ProjectFetch } from './types';

export const EVERY_MIN = 1;
export const EVERY_MAX = 24 * 60;
export const TIMEOUT_MIN = 30;
export const TIMEOUT_MAX = 30 * 60;

/** The intervals offered, in minutes. */
export const EVERY_PRESETS = [2, 5, 15, 30, 60];
/** The time limits offered, in seconds. */
export const TIMEOUT_PRESETS = [60, 120, 300, 600, 1800];

export const DEFAULT_FETCH: GitFetchSettings = { auto: true, everyMin: 5, timeoutSecs: 300 };
export const DEFAULT_PROJECT_FETCH: ProjectFetch = { mode: 'default', everyMin: 0, timeoutSecs: 0 };

/** Minutes as the design writes them: « 5 min », « 1 h », « 1 h 30 ». */
export function fMinutes(n: number): string {
  if (n < 60) return t('fetch.minutes', { n });
  const h = Math.floor(n / 60);
  const m = n % 60;
  return m ? t('fetch.hoursMinutes', { h, m: String(m).padStart(2, '0') }) : t('fetch.hours', { n: h });
}

/** Seconds: « 30 s », « 5 min », « 2 min 30 s ». */
export function fSeconds(n: number): string {
  if (n < 60) return t('fetch.seconds', { n });
  const m = Math.floor(n / 60);
  const s = n % 60;
  return s ? t('fetch.minutesSeconds', { m, s }) : fMinutes(m);
}

/**
 * A number typed in a field of whole numbers between `min` and `max`: the value it is saved as, brought back to its
 * bound when out of them (`clamped` says so). Empty: `empty` when the field may be (a project's « the general one »),
 * else null (left as it was).
 */
export function boundedInput(
  raw: string,
  min: number,
  max: number,
  empty: number | null = null,
): { value: number; clamped: boolean } | null {
  const text = raw.trim();
  if (!text) return empty === null ? null : { value: empty, clamped: false };
  const n = Number(text.replace(',', '.'));
  if (!Number.isFinite(n)) return null;
  const whole = Math.round(n);
  const value = Math.min(max, Math.max(min, whole));
  return { value, clamped: value !== n };
}

/** Whether a project is fetched in the background, how often and within what (as the backend's `plan`). */
export function fetchPlan(general: GitFetchSettings, own: ProjectFetch | undefined) {
  const p = own ?? DEFAULT_PROJECT_FETCH;
  const on = p.mode === 'default' ? general.auto : p.mode === 'always';
  const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
  return {
    on,
    everyMin: clamp(p.everyMin || general.everyMin, EVERY_MIN, EVERY_MAX),
    timeoutSecs: clamp(p.timeoutSecs || general.timeoutSecs, TIMEOUT_MIN, TIMEOUT_MAX),
  };
}

/** What the Git panel says beside « Fetch »: when it was last fetched (the time on hover). */
export function fetchedLine(lastFetch: number | null | undefined, now: number): { text: string; title: string } {
  if (!lastFetch) return { text: t('fetch.panel.never'), title: '' };
  return {
    text: t('fetch.panel.fetched', { ago: fAgo(lastFetch / 1000, now) }),
    title: t('fetch.panel.at', { when: fWhen(lastFetch, now) }),
  };
}

/** Why the last background fetch failed, in plain words that guide, and git's own on hover. */
export function failureLine(f: FetchFailure): { text: string; title: string } {
  const reason = t(`fetch.panel.reasons.${f.kind}`, { timeout: fSeconds(f.timeoutSecs) });
  return { text: t('fetch.panel.failed', { reason }), title: f.message };
}
