import { createHmac, randomBytes } from 'node:crypto';

// The store's rate limits (docs/STORE.md §8.1, review of 2026-10-29): the app's API, the downloads, the submissions,
// and `POST /api/v1/installs`, so that nobody swells a counter or turns the store into a downloader. Counted in this
// process's memory, by a keyed hash of the address (the key is drawn at start and never written): no address is
// kept, in the base or anywhere, and a restart forgets everything. Generous on purpose: a company's proxy puts many
// users behind one address (Zscaler, docs/STORE.md §8.1).

export interface Rule {
  name: string;
  /** How many requests a window lets through. */
  max: number;
  windowMs: number;
}

export const RULES = {
  /** The catalogue, `version.json`, the revocation list: an app reads them at start and every 6 hours. */
  api: { name: 'api', max: 300, windowMs: 60_000 },
  /** Archives and binaries: a few per installation. */
  downloads: { name: 'downloads', max: 300, windowMs: 3_600_000 },
  /** The installations counted: one plugin installed or updated is one; a whole setup, a few dozen. */
  installs: { name: 'installs', max: 60, windowMs: 3_600_000 },
  /** Submissions, retries and publications from one address (on top of 20 submissions an hour by account). */
  submissions: { name: 'submissions', max: 60, windowMs: 3_600_000 },
} as const satisfies Record<string, Rule>;

/** Past this many keys, the windows gone by are swept; if that is not enough, everything is forgotten (fail open). */
const MAX_KEYS = 100_000;

export class Limiter {
  private windows = new Map<string, { start: number; n: number }>();
  /** Each (address, plugin, version, event) counted once a day: the day it was, by key. */
  private seen = new Map<string, string>();
  private readonly secret = randomBytes(32);

  /** The address as this process knows it: a keyed hash, never the address. */
  who(address: string): string {
    return createHmac('sha256', this.secret).update(address).digest('base64url').slice(0, 22);
  }

  /** Takes one request of `rule` for `address`: allowed, or the seconds to wait. */
  take(rule: Rule, address: string, now = Date.now()): { ok: true } | { ok: false; retryAfter: number } {
    const key = `${rule.name}:${this.who(address)}`;
    let w = this.windows.get(key);
    if (!w || now - w.start >= rule.windowMs) {
      this.room(now);
      w = { start: now, n: 0 };
      this.windows.set(key, w);
    }
    if (w.n >= rule.max) return { ok: false, retryAfter: Math.max(1, Math.ceil((w.start + rule.windowMs - now) / 1000)) };
    w.n++;
    return { ok: true };
  }

  /** Whether `what` from `address` is new today (an installation counted once a day per address); notes it if so. */
  firstToday(address: string, what: string, now = new Date()): boolean {
    const today = now.toISOString().slice(0, 10);
    const key = `${this.who(address)}:${what}`;
    if (this.seen.get(key) === today) return false;
    if (this.seen.size >= MAX_KEYS) for (const [k, d] of this.seen) if (d !== today) this.seen.delete(k);
    if (this.seen.size >= MAX_KEYS) this.seen.clear();
    this.seen.set(key, today);
    return true;
  }

  private room(now: number) {
    if (this.windows.size < MAX_KEYS) return;
    for (const [k, w] of this.windows) if (now - w.start >= 3_600_000) this.windows.delete(k);
    if (this.windows.size >= MAX_KEYS) this.windows.clear();
  }
}

/** The process's limiter. */
export const limiter = new Limiter();

/**
 * The rule a request falls under, if any. The app's API by its path; the downloads apart; the counter; and what
 * starts or moves a submission.
 */
export function ruleFor(method: string, path: string): Rule | null {
  if (path === '/api/v1/installs') return method === 'POST' ? RULES.installs : null;
  if (path.startsWith('/api/v1/')) {
    if (/\/(archive\.tar\.gz|binaries\/[^/]+)$/.test(path)) return RULES.downloads;
    return RULES.api;
  }
  if (method === 'POST' && (path === '/api/submissions' || /^\/api\/submissions\/[^/]+\/(retry|publish)$/.test(path)))
    return RULES.submissions;
  return null;
}

/**
 * The client's address. Behind Coolify's proxy (Traefik), the last address of `X-Forwarded-For` is the one the proxy
 * saw (it does not trust what a client sends before it); without the header, the socket's.
 */
export function clientAddress(forwardedFor: string | undefined, socket: string | undefined): string {
  const last = (forwardedFor ?? '')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
    .pop();
  return last || socket || 'unknown';
}
