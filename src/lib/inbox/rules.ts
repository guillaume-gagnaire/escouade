// The rules of a request of the box « À toi » that need no state (ESC-571): when it is « Aujourd'hui », open or
// closed, its renders and their letters, the times « Plus tard » offers. The window's box (`inbox.svelte.ts`) and the
// phone's thread (apps/mobile) read them alike; nothing here imports the window's state.

import { fDateShort, fTime } from '../format';
import { intlLocale, t } from '../i18n';
import type { AskFile, WindowAsk } from '../types';

/** A closed request stays in « Répondu récemment » that long, and so many at most (§ 3.4). */
export const RECENT_MS = 7 * 24 * 3600 * 1000;
export const RECENT_MAX = 50;

/** Midnight of the day of `now`. */
export const startOfDay = (now: number) => {
  const d = new Date(now);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
};

/** A request that waits in the queue now (not put off for later). */
export const isOpen = (a: WindowAsk) => a.state === 'open';

/** It goes in « Aujourd'hui »: urgent today, back from « Plus tard », a ticket waits for it, or left today. */
export function isToday(a: WindowAsk, now = Date.now()) {
  return a.urgency === 'today' || a.woke || a.tickets.length > 0 || a.createdAt >= startOfDay(now);
}

/** Answered, withdrawn, expired or taken out: closed, or answered but not read yet. */
export const isClosed = (a: WindowAsk) => !['open', 'snoozed'].includes(a.state);

/** When it left the queue: its answer, else when it closed. */
export const closedAt = (a: WindowAsk) => a.answer?.at ?? a.closedAt ?? a.updatedAt;

/** The images of an option (or of the request itself, `option` null), in their order. */
export const imagesOf = (a: WindowAsk, option: string | null) => a.files.filter((f) => f.kind === 'image' && f.option === option);

/** Whether an image is the light render of its option, by its name (« -clair », « -light »). */
export const isLight = (f: AskFile) => /(^|[-_. ])(clair|light)([-_. ]|$)/i.test(f.name.replace(/\.[^.]+$/, ''));

/** The letter of an option, as the box writes it: « B ». */
export const letter = (id: string) => id.toUpperCase();

/** The times « Plus tard » offers (§ 8.4), from `now`: in an hour, this evening at 18:00 (before 17:00), tomorrow at 9:00, Monday at 9:00 (unless tomorrow is Monday). */
export function laterTimes(now = Date.now()): { key: 'hour' | 'evening' | 'tomorrow' | 'monday'; at: number }[] {
  const d = new Date(now);
  const at = (days: number, h: number) => {
    const x = new Date(d);
    x.setDate(x.getDate() + days);
    x.setHours(h, 0, 0, 0);
    return x.getTime();
  };
  const out: { key: 'hour' | 'evening' | 'tomorrow' | 'monday'; at: number }[] = [{ key: 'hour', at: now + 3600_000 }];
  if (d.getHours() < 17) out.push({ key: 'evening', at: at(0, 18) });
  out.push({ key: 'tomorrow', at: at(1, 9) });
  const toMonday = (8 - d.getDay()) % 7 || 7;
  if (toMonday > 1) out.push({ key: 'monday', at: at(toMonday, 9) });
  return out;
}

/**
 * A time near `now`, as « Plus tard » and the trace say it: « 17:50 » today, « demain 9:00 », « lundi 9:00 » within the
 * week, « 2 décembre 9:00 » beyond.
 */
export function fLater(ts: number, now = Date.now()): string {
  const day = (x: number) => {
    const d = new Date(x);
    d.setHours(0, 0, 0, 0);
    return d.getTime();
  };
  const days = Math.round((day(ts) - day(now)) / 86_400_000);
  const time = fTime(ts);
  if (days === 0) return time;
  if (days === 1) return t('inbox.later.tomorrowAt', { time });
  if (days === -1) return t('inbox.later.yesterdayAt', { time });
  if (days > 1 && days < 7)
    return t('inbox.later.dayAt', { day: new Date(ts).toLocaleDateString(intlLocale(), { weekday: 'long' }), time });
  return t('inbox.later.dayAt', { day: fDateShort(ts, now), time });
}

/** The option the answer starts on: the agent's recommendation, else the first (never chosen without the user). */
export const startOption = (a: WindowAsk) => a.options.find((o) => o.id === a.recommend?.option)?.id ?? a.options[0]?.id ?? '';
