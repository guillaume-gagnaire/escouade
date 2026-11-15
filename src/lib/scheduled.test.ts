import { afterEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { cronOf, dateLabel, dayRank, nextLabel, parseTime, rhythmLabel, runningTicket, timeValue } from './scheduled';
import type { Rhythm, ScheduledTask } from './types';
import { ticket } from '../test/ipc';

const r = (over: Partial<Rhythm>): Rhythm => ({ kind: 'daily', days: [], day: 1, hour: 7, minute: 0, cron: '', ...over });

describe('scheduled', () => {
  afterEach(() => setLang('fr'));

  it('says a rhythm as the mock-ups do', () => {
    expect(rhythmLabel(r({ kind: 'weekly', days: [1] }))).toBe('Chaque lundi 7:00');
    expect(rhythmLabel(r({ hour: 23, minute: 30 }))).toBe('Chaque jour 23:30');
    expect(rhythmLabel(r({ kind: 'monthly', day: 1, hour: 9 }))).toBe('Le 1er du mois 9:00');
    expect(rhythmLabel(r({ kind: 'monthly', day: 15, hour: 9 }))).toBe('Le 15 du mois 9:00');
    expect(rhythmLabel(r({ kind: 'weekly', days: [4, 1] }))).toBe('Chaque lun., jeu. 7:00');
    expect(rhythmLabel(r({ kind: 'weekly', days: [1, 2, 3, 4, 5] }))).toBe('Du lundi au vendredi 7:00');
    expect(rhythmLabel(r({ kind: 'weekly', days: [1, 2, 3, 4, 5, 6, 7] }))).toBe('Chaque jour 7:00');
    expect(rhythmLabel(r({ kind: 'cron', cron: ' 0  7 * * 1 ' }))).toBe('Cron 0 7 * * 1');
    setLang('en');
    expect(rhythmLabel(r({ kind: 'weekly', days: [1] }))).toBe('Every Monday 7:00');
    expect(rhythmLabel(r({ kind: 'monthly', day: 2, hour: 9 }))).toBe('On the 2nd of the month 9:00');
    expect([1, 3, 11, 22].map(dayRank)).toEqual(['1st', '3rd', '11th', '22nd']);
  });

  it('writes the cron expression a rhythm runs by, as the backend does', () => {
    expect(cronOf(r({}))).toBe('0 7 * * *');
    expect(cronOf(r({ kind: 'weekly', days: [7, 1, 5, 1], minute: 30 }))).toBe('30 7 * * 1,5,0');
    expect(cronOf(r({ kind: 'monthly', day: 1, hour: 9 }))).toBe('0 9 1 * *');
  });

  it('says the next due time from now', () => {
    const now = new Date(2026, 10, 2, 10, 0).getTime();
    expect(nextLabel(new Date(2026, 10, 2, 23, 30).getTime(), now)).toBe('ce soir');
    expect(nextLabel(new Date(2026, 10, 2, 14, 0).getTime(), now)).toBe('aujourd’hui 14:00');
    expect(nextLabel(new Date(2026, 10, 3, 7, 0).getTime(), now)).toBe('demain 7:00');
    const later = new Date(2026, 10, 9, 7, 0).getTime();
    expect(nextLabel(later, now)).toBe(dateLabel(later));
    expect(dateLabel(later)).toBe('lun. 9 nov. 7:00');
  });

  it('reads and writes the time field', () => {
    expect(parseTime('07:30')).toEqual({ hour: 7, minute: 30 });
    expect(parseTime('24:00')).toBeNull();
    expect(parseTime('7h')).toBeNull();
    expect(timeValue({ hour: 7, minute: 5 })).toBe('07:05');
  });

  it('holds the next run back while the ticket before is to do or under way only', () => {
    const task = { lastTicket: 't1' } as ScheduledTask;
    for (const [column, held] of [
      ['todo', true],
      ['doing', true],
      ['review', false],
      ['done', false],
    ] as const) {
      expect(!!runningTicket(task, { t1: ticket({ column }) }), column).toBe(held);
    }
    expect(runningTicket(task, {})).toBeUndefined();
  });
});
