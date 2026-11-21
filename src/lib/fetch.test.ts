import { afterEach, describe, expect, it } from 'vitest';
import { boundedInput, fMinutes, fSeconds, fetchPlan } from './fetch';
import { setLang } from './i18n';

// ESC-561: the durations as written, a number typed within its bounds, a project's own values over the general setting.
describe('the background fetch', () => {
  afterEach(() => setLang('fr'));

  it('writes its durations as the design does', () => {
    expect([2, 5, 59, 60, 90, 1440].map(fMinutes)).toEqual(['2 min', '5 min', '59 min', '1 h', '1 h 30', '24 h']);
    expect([30, 60, 150, 300, 1800].map(fSeconds)).toEqual(['30 s', '1 min', '2 min 30 s', '5 min', '30 min']);
    setLang('en');
    expect(fMinutes(90)).toBe('1 h 30 min');
  });

  it('brings a number typed back to its bounds, says it, and keeps an empty or wrong one as it was', () => {
    expect(boundedInput('15', 1, 1440)).toEqual({ value: 15, clamped: false });
    expect(boundedInput('0', 1, 1440)).toEqual({ value: 1, clamped: true });
    expect(boundedInput('99999', 1, 1440)).toEqual({ value: 1440, clamped: true });
    expect(boundedInput('2,5', 1, 1440)).toEqual({ value: 3, clamped: true });
    expect(boundedInput('', 1, 1440)).toBeNull();
    expect(boundedInput('abc', 1, 1440)).toBeNull();
    // A project's field: empty is the general one's.
    expect(boundedInput(' ', 30, 1800, 0)).toEqual({ value: 0, clamped: false });
  });

  it('takes a project’s own values over the general setting, as the backend does', () => {
    const general = { auto: true, everyMin: 5, timeoutSecs: 300 };
    expect(fetchPlan(general, undefined)).toEqual({ on: true, everyMin: 5, timeoutSecs: 300 });
    expect(fetchPlan({ ...general, auto: false }, { mode: 'always', everyMin: 0, timeoutSecs: 0 }).on).toBe(true);
    expect(fetchPlan(general, { mode: 'never', everyMin: 0, timeoutSecs: 0 }).on).toBe(false);
    expect(fetchPlan(general, { mode: 'default', everyMin: 60, timeoutSecs: 5000 })).toEqual({
      on: true,
      everyMin: 60,
      timeoutSecs: 1800,
    });
  });
});
