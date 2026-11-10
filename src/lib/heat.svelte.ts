// The temperature of an agent's cache (ESC-398, docs/FEATURES.md §3): 100 °C when its cache just served, down to 0 °C
// when it expires, in a straight line over its life (the vendor drops it whole at the end, it does not fade: what the
// figure says is the share of its life left, which a line says best). 0 °C too when it broke, « — °C » when its harness
// says nothing of it. One clock for every figure, which runs only while a cache is warm.

import { engineName } from './engines';
import { fAgo, fTok, fUsd } from './format';
import { t } from './i18n';
import type { Agent, CacheHeat, HeatBroken } from './types';

/** How warm it is: hot (≥ 67 °C), mid (≥ 34 °C), cool (above 0), cold (0 °C), none (« — °C »). */
export type HeatTone = 'hot' | 'mid' | 'cool' | 'cold' | 'none';

/** When it expires (ms), if its life is told and it did not break. */
export function expiresAt(h: CacheHeat | null | undefined): number | null {
  if (!h || h.ttl == null || h.broken) return null;
  return h.at + h.ttl * 1000;
}

/** Its degrees at `now`, 100 to 0; null when its life is not told (« — °C »). */
export function temperature(h: CacheHeat | null | undefined, now: number): number | null {
  if (!h || h.ttl == null) return null;
  const end = expiresAt(h);
  if (end === null || h.ttl <= 0) return 0;
  const left = end - now;
  if (left <= 0) return 0;
  // Rounded up: 0 °C only once expired.
  return Math.min(100, Math.ceil((left / (h.ttl * 1000)) * 100));
}

export function heatTone(deg: number | null): HeatTone {
  if (deg === null) return 'none';
  if (deg >= 67) return 'hot';
  if (deg >= 34) return 'mid';
  if (deg > 0) return 'cool';
  return 'cold';
}

/** « 3 min 40 », « 40 s », « 1 h 05 ». */
export function fLeft(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s >= 3600) return t('heat.left.hours', { h: Math.floor(s / 3600), m: String(Math.floor((s % 3600) / 60)).padStart(2, '0') });
  if (s >= 60) return t('heat.left.minutes', { m: Math.floor(s / 60), s: String(s % 60).padStart(2, '0') });
  return t('heat.left.seconds', { s });
}

/** « 72 °C », « — °C ». */
export function fHeat(deg: number | null): string {
  return deg === null ? t('heat.unknown') : t('heat.degrees', { n: deg });
}

function why(b: string): string {
  const known: HeatBroken[] = ['model', 'compaction', 'prompt', 'account', 'conversation'];
  return known.includes(b as HeatBroken) ? t(`heat.why.${b as HeatBroken}`) : b;
}

/** What a screen reader says of it, and the card's tooltip: « Cache : 72 °C, expire dans 3 min 40 ». */
export function heatLabel(a: Pick<Agent, 'heat' | 'engine'>, now: number): string {
  const h = a.heat;
  const deg = temperature(h, now);
  if (!h) return t('heat.aria.none', { engine: engineName(a.engine) });
  if (deg === null) return t('heat.aria.unknown', { engine: engineName(a.engine) });
  if (h.broken) return t('heat.aria.broken', { why: why(h.broken) });
  if (deg === 0) return t('heat.aria.expired');
  return t('heat.aria.warm', { n: deg, left: fLeft((expiresAt(h) ?? now) - now) });
}

function fLife(ttl: number): string {
  return ttl >= 3600 && ttl % 3600 === 0
    ? t('heat.detail.life.hours', { count: ttl / 3600 })
    : t('heat.detail.life.minutes', { n: Math.round(ttl / 60) });
}

export interface HeatRow {
  label: string;
  value: string;
}

/** The detail shown over the header: its title, what it says of its life, its rows, and the next message cold. */
export function heatDetail(
  a: Pick<Agent, 'heat' | 'engine'>,
  now: number,
): { title: string; line: string; rows: HeatRow[]; next: string | null } {
  const h = a.heat;
  const deg = temperature(h, now);
  const engine = engineName(a.engine);
  const title = deg === null ? t('heat.detail.titleUnknown') : t('heat.detail.title', { n: deg });
  if (!h) return { title, line: t('heat.aria.none', { engine }), rows: [], next: null };
  const ago = fAgo(h.at / 1000, now);
  const line = h.broken
    ? t('heat.detail.broken', { ago, why: why(h.broken) })
    : h.ttl == null
      ? t('heat.detail.unknownLife', { ago, engine })
      : deg === 0
        ? t('heat.detail.expired', { ago, engine, life: fLife(h.ttl) })
        : t('heat.detail.warm', { ago, engine, life: fLife(h.ttl), left: fLeft((expiresAt(h) ?? now) - now) });
  const total = h.cached + h.fresh;
  const pct = (n: number) => (total > 0 ? ` · ${Math.round((n / total) * 100)} %` : '');
  const rows: HeatRow[] = [];
  if (h.read != null && h.written != null) {
    rows.push({ label: t('heat.detail.read'), value: fTok(h.read) + pct(h.read) });
    rows.push({ label: t('heat.detail.written'), value: fTok(h.written) });
  } else {
    rows.push({ label: t('heat.detail.cached'), value: fTok(h.cached) + pct(h.cached) });
  }
  rows.push({ label: t('heat.detail.fresh'), value: fTok(h.fresh) });
  if (h.saved != null) rows.push({ label: t('heat.detail.saved'), value: `≈ ${fUsd(h.saved)}` });
  const next =
    h.context > 0
      ? h.cold != null && h.warm != null
        ? t('heat.detail.next', { tokens: fTok(h.context), cold: fUsd(h.cold), warm: fUsd(h.warm) })
        : t('heat.detail.nextTokens', { tokens: fTok(h.context) })
      : null;
  return { title, line, rows, next };
}

/** The local day of `ms`, as the backend writes it (`YYYY-MM-DD`). */
export function localDay(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

/** What the cache of `agents` saved today (USD). */
export function savedToday(agents: Iterable<Pick<Agent, 'heat'>>, now: number): number {
  const day = localDay(now);
  let sum = 0;
  for (const a of agents) if (a.heat?.day === day) sum += a.heat.savedToday || 0;
  return sum;
}

/** When the last warm cache of `agents` expires (ms), or null when none is warm at `now`. */
export function lastExpiry(agents: Iterable<Pick<Agent, 'heat'>>, now: number): number | null {
  let last: number | null = null;
  for (const a of agents) {
    const end = expiresAt(a.heat);
    if (end !== null && end > now && (last === null || end > last)) last = end;
  }
  return last;
}

type Timers = {
  every: (fn: () => void, ms: number) => unknown;
  stop: (id: unknown) => void;
  now: () => number;
};

const realTimers: Timers = {
  every: (fn, ms) => setInterval(fn, ms),
  stop: (id) => clearInterval(id as ReturnType<typeof setInterval>),
  now: () => Date.now(),
};

/**
 * The one clock of every figure: it ticks each second while a cache is warm (`watch`), and stops once the last one
 * expired; nothing runs while every agent is cold or says nothing.
 */
export class HeatClock {
  now = $state(Date.now());
  #timer: unknown = null;
  #until: number | null = null;
  #timers: Timers;

  constructor(timers: Timers = realTimers) {
    this.#timers = timers;
    this.now = timers.now();
  }

  /** Whether it ticks. */
  get running(): boolean {
    return this.#timer !== null;
  }

  /** The agents changed: it runs until the last of their warm caches expires. */
  watch(agents: Iterable<Pick<Agent, 'heat'>>): void {
    const now = this.#timers.now();
    this.now = now;
    this.#until = lastExpiry(agents, now);
    if (this.#until === null) this.#halt();
    else if (this.#timer === null) this.#timer = this.#timers.every(() => this.#tick(), 1000);
  }

  #tick(): void {
    this.now = this.#timers.now();
    if (this.#until === null || this.now >= this.#until) this.#halt();
  }

  #halt(): void {
    if (this.#timer !== null) this.#timers.stop(this.#timer);
    this.#timer = null;
  }
}

export const heatClock = new HeatClock();
