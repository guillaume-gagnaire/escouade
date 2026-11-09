import { afterEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { fHeat, fLeft, heatDetail, heatLabel, heatTone, HeatClock, lastExpiry, savedToday, temperature } from './heat.svelte';
import { AT, heat } from '../test/heat';

const MIN = 60_000;

afterEach(() => setLang('fr'));

describe('temperature', () => {
  it('is 100 °C when the cache just served and goes down in a line to 0 °C at its expiry', () => {
    const h = heat();
    expect(temperature(h, AT)).toBe(100);
    expect(temperature(h, AT + 2.5 * MIN)).toBe(50);
    expect(temperature(h, AT + 3 * MIN + 40_000)).toBe(27);
    // 0 °C only once expired.
    expect(temperature(h, AT + 5 * MIN - 1000)).toBe(1);
    expect(temperature(h, AT + 5 * MIN)).toBe(0);
    expect(temperature(h, AT + 60 * MIN)).toBe(0);
  });

  it('follows the life of an hour of Claude Code', () => {
    expect(temperature(heat({ ttl: 3600 }), AT + 18 * MIN)).toBe(70);
  });

  it('is 0 °C when the cache broke, however recent', () => {
    expect(temperature(heat({ broken: 'model' }), AT)).toBe(0);
    expect(temperature(heat({ broken: 'compaction' }), AT + 1000)).toBe(0);
  });

  it('is unknown (« — °C ») when the harness tells nothing, or not how long its cache lives', () => {
    expect(temperature(null, AT)).toBeNull();
    expect(temperature(undefined, AT)).toBeNull();
    expect(temperature(heat({ ttl: null }), AT)).toBeNull();
    expect(fHeat(null)).toBe('— °C');
    expect(fHeat(72)).toBe('72 °C');
  });

  it('gives its tone: hot, mid way, cooling, cold, none', () => {
    expect([100, 67, 66, 34, 33, 1, 0, null].map(heatTone)).toEqual(['hot', 'hot', 'mid', 'mid', 'cool', 'cool', 'cold', 'none']);
  });
});

describe('what it says', () => {
  it('says the figure and what it means, in words, in both languages', () => {
    const a = { heat: heat(), engine: 'codex' as const };
    expect(heatLabel(a, AT + 2 * MIN)).toBe('Cache : 60 °C, expire dans 3 min 00');
    expect(heatLabel({ ...a, heat: heat({ broken: 'model' }) }, AT)).toBe(
      'Cache : 0 °C, cassé (modèle changé) : le prochain message relira tout au plein tarif',
    );
    expect(heatLabel(a, AT + 6 * MIN)).toBe('Cache : 0 °C, expiré : le prochain message relira tout au plein tarif');
    expect(heatLabel({ heat: null, engine: 'cursor' }, AT)).toBe('Cache : — °C, Cursor CLI n’a encore rien dit de son cache');
    setLang('en');
    expect(heatLabel(a, AT + 2 * MIN)).toBe('Cache: 60 °C, expires in 3 min 00');
  });

  it('writes the time left', () => {
    expect(fLeft(40_000)).toBe('40 s');
    expect(fLeft(80_000)).toBe('1 min 20');
    expect(fLeft(3_900_000)).toBe('1 h 05');
  });

  it('details the last turn: its life, the tokens read, written and at full price, the savings and the next message cold', () => {
    const d = heatDetail({ heat: heat(), engine: 'codex' }, AT + 3 * MIN + 40_000);
    expect(d.title).toBe('Cache : 27 °C');
    expect(d.line).toBe('Dernier tour il y a 3 min · expire dans 1 min 20 (le cache de Codex vit 5 min sans activité).');
    expect(d.rows.map((r) => [r.label, r.value])).toEqual([
      ['Lus depuis le cache (chauds)', '129,0 k · 91 %'],
      ['Mis en cache (froids)', '9,0 k'],
      ['Nouveaux, plein tarif', '4,0 k'],
      ['Économisé sur ce tour', '≈ 0,36 $'],
    ]);
    expect(d.next).toBe('Si le prochain message part à froid : ≈ 142,0 k tokens relus, ≈ 0,47 $ au lieu de ≈ 0,050 $.');
  });

  it('tells read and written in one row when its harness tells them together, and invents no saving', () => {
    const d = heatDetail(
      { heat: heat({ read: null, written: null, saved: null, warm: null, cold: null, ttl: 300 }), engine: 'copilot' },
      AT,
    );
    expect(d.rows.map((r) => r.label)).toEqual(['Lus ou mis en cache', 'Nouveaux, plein tarif']);
    expect(d.next).toBe('Si le prochain message part à froid : ≈ 142,0 k tokens relus au plein tarif.');
  });

  it('says a life of an hour, and why a cache broke', () => {
    expect(heatDetail({ heat: heat({ ttl: 3600 }), engine: 'claude' }, AT).line).toContain('vit 1 heure sans activité');
    expect(heatDetail({ heat: heat({ broken: 'compaction' }), engine: 'claude' }, AT).line).toBe(
      'Dernier tour à l’instant · cassé : contexte compacté.',
    );
    expect(heatDetail({ heat: heat({ ttl: null }), engine: 'gemini' }, AT).line).toBe(
      'Dernier tour à l’instant · Gemini CLI ne dit pas combien de temps vit son cache.',
    );
  });
});

describe('the day', () => {
  it('adds up what the cache saved today, not on another day', () => {
    const agents = [
      { heat: heat({ savedToday: 1.5 }) },
      { heat: heat({ savedToday: 2 }) },
      { heat: heat({ day: '2026-11-08', savedToday: 9 }) },
      { heat: null },
    ];
    expect(savedToday(agents, AT)).toBeCloseTo(3.5);
    expect(savedToday(agents, AT + 86_400_000)).toBe(0);
  });
});

describe('the clock', () => {
  function fakeTimers(start: number) {
    const timers = {
      t: start,
      live: new Map<number, () => void>(),
      next: 1,
      started: 0,
      every(fn: () => void) {
        timers.started += 1;
        const id = timers.next++;
        timers.live.set(id, fn);
        return id;
      },
      stop(id: unknown) {
        timers.live.delete(id as number);
      },
      now: () => timers.t,
      tick(ms: number) {
        for (let i = 0; i < ms / 1000; i++) {
          timers.t += 1000;
          for (const fn of [...timers.live.values()]) fn();
        }
      },
    };
    return timers;
  }

  it('runs one timer while a cache is warm, and stops with the last one', () => {
    const timers = fakeTimers(AT);
    const clock = new HeatClock(timers);
    clock.watch([{ heat: heat() }, { heat: heat({ ttl: 600 }) }, { heat: null }]);
    expect(clock.running).toBe(true);
    // Watched again (an agent changed): still one timer.
    clock.watch([{ heat: heat() }, { heat: heat({ ttl: 600 }) }]);
    expect(timers.started).toBe(1);
    expect(timers.live.size).toBe(1);
    timers.tick(5 * MIN);
    expect(clock.running).toBe(true);
    expect(clock.now).toBe(AT + 5 * MIN);
    timers.tick(5 * MIN);
    expect(clock.running).toBe(false);
    expect(timers.live.size).toBe(0);
  });

  it('runs nothing while every cache is cold, broken or untold', () => {
    const timers = fakeTimers(AT + 10 * MIN);
    const clock = new HeatClock(timers);
    clock.watch([{ heat: heat() }, { heat: heat({ broken: 'model', at: AT + 10 * MIN }) }, { heat: heat({ ttl: null }) }, { heat: null }]);
    expect(clock.running).toBe(false);
    expect(timers.started).toBe(0);
    expect(lastExpiry([{ heat: heat() }], AT)).toBe(AT + 5 * MIN);
  });
});
