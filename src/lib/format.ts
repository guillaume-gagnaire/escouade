// Numbers, amounts, durations, dates and sizes as the design writes them, in the language of the interface: the
// units are in the `format` zone of the catalogs, the rest comes from `Intl`. They read `locale.ui` (through `t`
// and `intlLocale`), so a template that calls them follows a change of language.
import { t } from './i18n';
import { intlLocale, locale, type Lang } from './i18n/locale.svelte';
import { IS_MAC } from './platform';

const separators = new Map<string, string>();

/** A number written by `toFixed` with the decimal separator of the language: « 1,2 » in French, « 1.2 » in English. */
function decimal(s: string): string {
  const tag = intlLocale();
  let sep = separators.get(tag);
  if (sep === undefined) {
    sep = new Intl.NumberFormat(tag).formatToParts(1.5).find((p) => p.type === 'decimal')?.value ?? '.';
    separators.set(tag, sep);
  }
  return s.replace('.', sep);
}

const pad = (n: number) => String(n).padStart(2, '0');

export function fTok(n: number): string {
  if (n >= 1e9) return t('format.tokens.billion', { n: decimal((n / 1e9).toFixed(2)) });
  if (n >= 1e6) return t('format.tokens.million', { n: decimal((n / 1e6).toFixed(2)) });
  if (n >= 1e3) return t('format.tokens.thousand', { n: decimal((n / 1e3).toFixed(1)) });
  return String(Math.round(n));
}

/** « 2,84 $ » in French (« $ » kept on the line of its amount by a non-breaking space), « $2.84 » in English. */
export function fUsd(x: number): string {
  const digits = x > 0 && x < 0.1 ? 3 : 2;
  return t('format.usd', { amount: x.toLocaleString(intlLocale(), { minimumFractionDigits: digits, maximumFractionDigits: digits }) });
}

export function fDur(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return h ? t('format.duration.hours', { h, m: pad(m) }) : t('format.duration.minutes', { m, s: pad(s % 60) });
}

/** How long ago `seconds` (Unix time) was, relative to `now` (ms). */
export function fAgo(seconds: number, now: number): string {
  const s = Math.max(0, now / 1000 - seconds);
  if (s < 60) return t('format.ago.now');
  if (s < 3600) return t('format.ago.minutes', { n: Math.floor(s / 60) });
  if (s < 86400) return t('format.ago.hours', { n: Math.floor(s / 3600) });
  const d = Math.floor(s / 86400);
  if (d === 1) return t('format.ago.yesterday');
  if (d < 30) return t('format.ago.days', { n: d });
  return new Date(seconds * 1000).toLocaleDateString(intlLocale());
}

/** For how long since `since` (ms), relative to `now` (ms), to follow « depuis »: "< 1 min", "3 min", "2 h", "3 j". */
export function fSince(since: number, now: number): string {
  const s = Math.max(0, (now - since) / 1000);
  if (s < 60) return t('format.since.underMinute');
  if (s < 3600) return t('format.since.minutes', { n: Math.floor(s / 60) });
  if (s < 86400) return t('format.since.hours', { n: Math.floor(s / 3600) });
  return t('format.since.days', { count: Math.floor(s / 86400) });
}

/** "1h48" style countdown until a timestamp. */
export function fCountdown(target: number | null, now: number): string {
  if (!target) return '—';
  const s = Math.max(0, Math.floor((target - now) / 1000));
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return t('format.countdown.days', { d, h });
  return t('format.countdown.hours', { h, m: pad(m) });
}

export function fPct(p: number): string {
  return t('format.percent', { n: Math.round(p) });
}

export function fInt(n: number): string {
  return Math.round(n).toLocaleString(intlLocale());
}

/** A number with at most `maxDecimals` decimals, grouped: « 1,3 », « 1 235 » in French, « 1.3 », « 1,235 » in English. */
export function fNum(n: number, maxDecimals = 0): string {
  return n.toLocaleString(intlLocale(), { maximumFractionDigits: maxDecimals });
}

export function fDate(ts: number): string {
  return new Date(ts).toLocaleDateString(intlLocale(), { day: 'numeric', month: 'long', year: 'numeric' });
}

/** The hour of a time of day: French writes « 09:30 », English « 9:30 AM ». */
const HOUR: Record<Lang, '2-digit' | 'numeric'> = { fr: '2-digit', en: 'numeric' };

export function fTime(ts: number): string {
  return new Date(ts).toLocaleTimeString(intlLocale(), { hour: HOUR[locale.ui], minute: '2-digit' });
}

/** A day and its time: « 2 octobre 2026 à 09:30 », « October 2, 2026 at 9:30 AM ». */
export function fDateTime(ts: number): string {
  return t('format.dateTime', { date: fDate(ts), time: fTime(ts) });
}

/** `s` as an expression of JavaScript that matches it as written. */
export const escapeRegExp = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, (c) => `\\${c}`);

/**
 * « 3 fichiers », for the texts not extracted to the catalogs yet: French only.
 * @deprecated A plural leaf `{ one, other }` read by `t(key, { count })`, which follows the rules of each language.
 */
export function plural(n: number, one: string, many: string): string {
  return `${n} ${n > 1 ? many : one}`;
}

export function basename(p: string): string {
  const s = p.replace(/\\/g, '/');
  return s.slice(s.lastIndexOf('/') + 1);
}

export function dirname(p: string): string {
  const s = p.replace(/\\/g, '/');
  const i = s.lastIndexOf('/');
  return i > 0 ? s.slice(0, i) : '.';
}

/** Path relative to `base` when inside it (case-insensitive, Windows friendly). */
export function relPath(base: string, p: string): string {
  const b = base.replace(/\\/g, '/').replace(/\/$/, '');
  const s = p.replace(/\\/g, '/');
  return s.toLowerCase().startsWith(b.toLowerCase() + '/') ? s.slice(b.length + 1) : s;
}

/** `rel` (forward slashes) under the folder `root`, written as its system does: with `\` for a Windows folder. */
export function joinPath(root: string, rel: string): string {
  const sep = /^[A-Za-z]:|\\/.test(root) ? '\\' : '/';
  return (root.replace(/[\\/]+$/, '') + '/' + rel).replace(/[\\/]/g, sep);
}

/** A path from the root of a drive or of the filesystem (`C:\x`, `/x`, `\\server\x`), not relative to a folder. */
export function isAbsPath(p: string): boolean {
  return /^([A-Za-z]:[\\/]|[\\/])/.test(p);
}

/** Shortens the user's home folder to "~" for display: `C:\Users\<name>` on Windows, `/Users/<name>` on macOS (`mac`). */
export function tildify(p: string, mac = IS_MAC): string {
  const home = mac ? /^\/Users\/[^/]+(\/.*)?$/ : /^[A-Za-z]:\/Users\/[^/]+(\/.*)?$/;
  const m = p.replace(/\\/g, '/').match(home);
  return m ? '~' + (m[1] ?? '') : p;
}

/**
 * A size: "312 Mo", "1,3 Go" (a memory), or with `decimals` for the megabytes, "1,3 Mo" (a file). The megabytes
 * are not grouped: "1023 Mo".
 */
export function fBytes(n: number, decimals = 0): string {
  const MB = 1024 * 1024;
  const mb = (n / MB).toLocaleString(intlLocale(), { maximumFractionDigits: decimals, useGrouping: false });
  if (n < 1024 * MB) return t('format.bytes.megabytes', { n: mb });
  return t('format.bytes.gigabytes', { n: fNum(n / (1024 * MB), Math.max(1, decimals)) });
}

const lists = new Map<string, Intl.ListFormat>();

/** Items joined as a sentence does: « a, b et c », « a, b, and c ». */
export function fList(items: string[]): string {
  const tag = intlLocale();
  let list = lists.get(tag);
  if (!list) lists.set(tag, (list = new Intl.ListFormat(tag, { style: 'long', type: 'conjunction' })));
  return list.format(items);
}

/** When something happens: "à 15:00" today, "le vendredi 2 octobre à 09:30" another day. */
export function fWhen(ts: number, now: number): string {
  const d = new Date(ts);
  if (d.toDateString() === new Date(now).toDateString()) return t('format.when.today', { time: fTime(ts) });
  const date = d.toLocaleDateString(intlLocale(), { weekday: 'long', day: 'numeric', month: 'long' });
  return t('format.when.day', { date, time: fTime(ts) });
}
