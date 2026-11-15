// The scheduled tasks (ESC-448): how their rhythm and their next due time read, in the section « Programmées » and in
// their form. The due times themselves are the backend's (`scheduled.rs`, with the machine's summer time): the window
// only says them.
import { intlLocale, locale, t } from './i18n';
import type { Rhythm, ScheduledTask, Ticket } from './types';

/** A new task's rhythm, as the form opens: every Monday at 7:00. */
export function defaultRhythm(): Rhythm {
  return { kind: 'weekly', days: [1], day: 1, hour: 7, minute: 0, cron: '' };
}

/** `7:00`, `23:30`: the hour without its leading zero, as the mock-ups write it. */
export function clock(hour: number, minute: number): string {
  return `${hour}:${String(minute).padStart(2, '0')}`;
}

/** `07:00`, the value of a time field. */
export function timeValue(r: Pick<Rhythm, 'hour' | 'minute'>): string {
  return `${String(r.hour).padStart(2, '0')}:${String(r.minute).padStart(2, '0')}`;
}

/** The hour and minute a time field holds (`07:30`), or null. */
export function parseTime(value: string): { hour: number; minute: number } | null {
  const m = /^(\d{1,2}):(\d{2})$/.exec(value.trim());
  if (!m) return null;
  const hour = Number(m[1]);
  const minute = Number(m[2]);
  return hour < 24 && minute < 60 ? { hour, minute } : null;
}

/** A day of the week, 1 (Monday) to 7 (Sunday), by its name in the language of the window. */
export function weekday(day: number, style: 'long' | 'short' | 'narrow' = 'long'): string {
  // 5 January 2026 is a Monday.
  const date = new Date(2026, 0, 4 + day);
  return new Intl.DateTimeFormat(intlLocale(), { weekday: style }).format(date);
}

/** The day of the month as a rank: « 1er », « 2 » ; “1st”, “2nd”. */
export function dayRank(day: number): string {
  if (locale.ui === 'fr') return day === 1 ? '1er' : String(day);
  const rule = new Intl.PluralRules('en-US', { type: 'ordinal' }).select(day);
  const suffix = { one: 'st', two: 'nd', few: 'rd' }[rule as 'one' | 'two' | 'few'] ?? 'th';
  return `${day}${suffix}`;
}

/** The cron expression a rhythm runs by, as the backend writes it (Sunday is 0). */
export function cronOf(r: Rhythm): string {
  const m = r.minute;
  const h = r.hour;
  switch (r.kind) {
    case 'daily':
      return `${m} ${h} * * *`;
    case 'weekly': {
      const days = [...new Set(r.days.filter((d) => d >= 1 && d <= 7))].sort((a, b) => a - b);
      return `${m} ${h} * * ${days.map((d) => d % 7).join(',')}`;
    }
    case 'monthly':
      return `${m} ${h} ${r.day} * *`;
    case 'cron':
      return r.cron.trim().split(/\s+/).join(' ');
  }
}

/** The rhythm in words: « Chaque lundi 7:00 », « Chaque jour 23:30 », « Le 1er du mois 9:00 », « Cron 0 7 * * 1 ». */
export function rhythmLabel(r: Rhythm): string {
  const time = clock(r.hour, r.minute);
  switch (r.kind) {
    case 'daily':
      return t('scheduled.rhythm.daily', { time });
    case 'weekly': {
      const days = [...new Set(r.days)].sort((a, b) => a - b);
      if (days.length === 7) return t('scheduled.rhythm.daily', { time });
      if (days.join() === '1,2,3,4,5') return t('scheduled.rhythm.weekdays', { time });
      if (days.length === 1) return t('scheduled.rhythm.weekly', { day: weekday(days[0]), time });
      return t('scheduled.rhythm.weeklyMany', { days: days.map((d) => weekday(d, 'short')).join(', '), time });
    }
    case 'monthly':
      return t('scheduled.rhythm.monthly', { day: dayRank(r.day), time });
    case 'cron':
      return t('scheduled.rhythm.cron', { cron: cronOf(r) });
  }
}

/** Midnight of the day of `ms`, in the machine's time. */
function dayStart(ms: number): number {
  const d = new Date(ms);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

/** A due time in words, from `now`: « ce soir », « aujourd’hui 14:00 », « demain 7:00 », « lun. 3 nov. 7:00 ». */
export function nextLabel(ms: number, now = Date.now()): string {
  const d = new Date(ms);
  const time = clock(d.getHours(), d.getMinutes());
  const days = Math.round((dayStart(ms) - dayStart(now)) / 86_400_000);
  if (days === 0) return d.getHours() >= 18 ? t('scheduled.next.tonight') : t('scheduled.next.today', { time });
  if (days === 1) return t('scheduled.next.tomorrow', { time });
  return dateLabel(ms);
}

/** « lun. 3 nov. 7:00 », a due time of the form's « Prochaines ». */
export function dateLabel(ms: number): string {
  const d = new Date(ms);
  const date = new Intl.DateTimeFormat(intlLocale(), { weekday: 'short', day: 'numeric', month: 'short' }).format(d);
  return t('scheduled.next.on', { date, time: clock(d.getHours(), d.getMinutes()) });
}

/** The ticket of the task's last run while it is « À faire » or « En cours »: the next due time passes its turn then. */
export function runningTicket(task: ScheduledTask, tickets: Record<string, Ticket>): Ticket | undefined {
  const tk = task.lastTicket ? tickets[task.lastTicket] : undefined;
  return tk && (tk.column === 'todo' || tk.column === 'doing') ? tk : undefined;
}

/** A project's tasks, in the order they were made. */
export function tasksOf(projectId: string, all: ScheduledTask[]): ScheduledTask[] {
  return all.filter((x) => x.projectId === projectId);
}
