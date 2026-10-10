import { describe, expect, it } from 'vitest';
import { BEAT_MS, CYCLE, SNAPSHOT, fDur, fTok, fUsd, squadAt, type Agent, type Answer, type Squad } from '../app/data/squad';

/** Times in beats from `from` to `to`, every `step`. */
const times = (from: number, to: number, step = 0.25) =>
  Array.from({ length: Math.round((to - from) / step) + 1 }, (_, i) => from + i * step);
const asking = (s: Squad) => s.agents.find((a) => a.question);
const ticketAgent = (s: Squad) => s.agents.find((a) => a.ticket)!;
/** When the first question of the run shows. */
const firstAsk = times(0, CYCLE).find((t) => asking(squadAt(t)))!;

describe('squad demo', () => {
  it('shows five agents of demo-api, each with its model', () => {
    const s = squadAt(0);
    expect(s.project.name).toBe('demo-api');
    expect(s.agents).toHaveLength(5);
    for (const a of s.agents) expect(['Fable', 'Opus', 'Sonnet', 'Haiku']).toContain(a.model);
    expect(new Set(s.agents.map((a) => a.name)).size).toBe(5);
  });

  it('counts running, waiting and finished agents as their cards show them', () => {
    for (const t of times(0, 2 * CYCLE, 0.5)) {
      const s = squadAt(t);
      const n = (...st: Agent['status'][]) => s.agents.filter((a) => st.includes(a.status)).length;
      expect(s.counts, `t=${t}`).toEqual({ running: n('running'), waiting: n('question'), done: n('done', 'totest') });
    }
    expect(squadAt(SNAPSHOT).counts).toEqual({ running: 3, waiting: 1, done: 1 });
  });

  it('keeps a running agent on a tool call, an asking one on its question, a finished one on its result', () => {
    for (const t of times(0, CYCLE)) {
      for (const a of squadAt(t).agents) {
        expect(a.activity !== null, `${a.name} t=${t}`).toBe(a.status === 'running');
        expect(a.question !== null, `${a.name} t=${t}`).toBe(a.status === 'question');
      }
    }
  });

  it('has an agent ask a question a few seconds in, then resume with the default answer', () => {
    expect(firstAsk * BEAT_MS).toBeGreaterThanOrEqual(1500);
    expect(firstAsk * BEAT_MS).toBeLessThanOrEqual(5000);
    const who = asking(squadAt(firstAsk))!;
    expect(who.question!.options.length).toBeGreaterThanOrEqual(2);
    const resumed = times(firstAsk, CYCLE).find((t) => squadAt(t).agents[who.slot].status === 'running');
    expect(resumed).toBeDefined();
  });

  it('resumes the asking agent as soon as the visitor answers, with the chosen option', () => {
    const at = firstAsk + 0.5;
    const slot = asking(squadAt(firstAsk))!.slot;
    expect(squadAt(at + 0.1).agents[slot].status).toBe('question');
    const yes = squadAt(at + 0.1, [{ at, option: 0 }]).agents[slot];
    const other = squadAt(at + 0.1, [{ at, option: 1 }]).agents[slot];
    expect(yes.status).toBe('running');
    expect(other.status).toBe('running');
    expect(other.activity).not.toEqual(yes.activity);
  });

  it('ignores an answer given when no question is pending', () => {
    const before = firstAsk - 0.5;
    expect(squadAt(firstAsk + 0.1, [{ at: before, option: 1 }])).toEqual(squadAt(firstAsk + 0.1));
    // Once the default answer has come, a late click changes nothing either.
    const slot = asking(squadAt(firstAsk))!.slot;
    const resumed = times(firstAsk, CYCLE).find((t) => squadAt(t).agents[slot].status === 'running')!;
    expect(squadAt(resumed + 1, [{ at: resumed, option: 1 }])).toEqual(squadAt(resumed + 1));
  });

  it('keeps an answer to its own question: the next cycle asks again', () => {
    const answers: Answer[] = [{ at: firstAsk + 0.5, option: 1 }];
    expect(squadAt(CYCLE + firstAsk + 0.1, answers)).toMatchObject({
      agents: squadAt(CYCLE + firstAsk + 0.1).agents.map(() => expect.anything()),
    });
    const slot = asking(squadAt(firstAsk))!.slot;
    expect(squadAt(CYCLE + firstAsk + 0.1, answers).agents[slot].status).toBe('question');
    const later = squadAt(CYCLE + 12, answers).agents[slot];
    expect(later.activity).toEqual(squadAt(CYCLE + 12).agents[slot].activity);
  });

  it('loops a ticket agent until its criteria are met, then hands it over for testing', () => {
    const seen = times(0, CYCLE).map((t) => ticketAgent(squadAt(t)));
    const first = seen.filter((a) => a.ticket!.key === 'DEM-4');
    expect(first[0].name.startsWith('dem-4-')).toBe(true);
    expect(new Set(first.map((a) => a.ticket!.loop))).toEqual(new Set([1, 2]));
    for (const a of first) expect(a.ticket!.loop).toBeLessThanOrEqual(a.ticket!.max);
    const ready = first.find((a) => a.status === 'totest');
    expect(ready?.ticket!.met.every(Boolean)).toBe(true);
    for (const a of first.filter((x) => x.status !== 'totest')) expect(a.ticket!.met.every(Boolean)).toBe(false);
  });

  it('starts the next ticket once one is ready to test', () => {
    const next = times(0, 2 * CYCLE)
      .map((t) => ticketAgent(squadAt(t)))
      .find((a) => a.ticket!.key === 'DEM-5');
    expect(next).toBeDefined();
    expect(next!.name.startsWith('dem-5-')).toBe(true);
    expect(next!.ticket).toMatchObject({ loop: 1, met: [false, false] });
  });

  it('never takes back tokens, cost, time or spend', () => {
    const answers: Answer[] = [{ at: firstAsk + 1, option: 1 }];
    let prev = squadAt(0, answers);
    for (const t of times(0.25, 3 * CYCLE)) {
      const s = squadAt(t, answers);
      expect(s.today, `t=${t}`).toBeGreaterThanOrEqual(prev.today);
      s.agents.forEach((a, i) => {
        if (a.name !== prev.agents[i].name) return;
        expect(a.tokens, `${a.name} t=${t}`).toBeGreaterThanOrEqual(prev.agents[i].tokens);
        expect(a.cost, `${a.name} t=${t}`).toBeGreaterThanOrEqual(prev.agents[i].cost);
        expect(a.ms, `${a.name} t=${t}`).toBeGreaterThanOrEqual(prev.agents[i].ms);
      });
      prev = s;
    }
    expect(prev.today).toBeGreaterThan(squadAt(0).today);
  });

  it('repeats its statuses every cycle', () => {
    for (const t of times(0, CYCLE - 0.25)) {
      expect(
        squadAt(t + CYCLE).agents.map((a) => a.status),
        `t=${t}`,
      ).toEqual(squadAt(t).agents.map((a) => a.status));
    }
  });

  it('stands still on a telling moment: work under way, a question, a finished task, a ticket mid-loop', () => {
    const s = squadAt(SNAPSHOT);
    const statuses = s.agents.map((a) => a.status);
    expect(statuses).toContain('running');
    expect(statuses).toContain('question');
    expect(statuses).toContain('done');
    const met = ticketAgent(s).ticket!.met;
    expect(met.some(Boolean) && !met.every(Boolean)).toBe(true);
  });

  it('shows the projects behind with their agents, one of them waiting at some point', () => {
    const s = squadAt(0);
    expect(s.projects.map((p) => p.name)).toEqual(['studio-web', 'mobile-app']);
    expect(times(0, CYCLE).some((t) => squadAt(t).projects.some((p) => p.agents.includes('question')))).toBe(true);
  });
});

describe('formatting, as in the app', () => {
  it('writes tokens, dollars and durations the French way', () => {
    expect(fTok(184_210)).toBe('184,2 k');
    expect(fTok(999)).toBe('999');
    expect(fTok(1_250_000)).toBe('1,25 M');
    expect(fUsd(2.414)).toBe('2,41 $');
    expect(fUsd(0.02)).toBe('0,020 $');
    expect(fDur(760_000)).toBe('12m 40s');
    expect(fDur(3_900_000)).toBe('1h 05m');
  });
});

describe('formatting in English', () => {
  it('writes tokens and dollars the English way, and durations as ever', () => {
    expect(fTok(184_210, 'en')).toBe('184.2k');
    expect(fTok(999, 'en')).toBe('999');
    expect(fTok(1_250_000, 'en')).toBe('1.25M');
    expect(fTok(2_500_000_000, 'en')).toBe('2.50B');
    expect(fUsd(2.414, 'en')).toBe('$2.41');
    expect(fUsd(0.02, 'en')).toBe('$0.020');
    expect(fUsd(1234.5, 'en')).toBe('$1,234.50');
    // Durations read the same in both languages.
    expect(fDur(760_000)).toBe('12m 40s');
    expect(fDur(3_900_000)).toBe('1h 05m');
  });

  it('keeps writing French by default', () => {
    expect(fTok(184_210, 'fr')).toBe(fTok(184_210));
    expect(fUsd(2.414, 'fr')).toBe(fUsd(2.414));
  });
});

describe('squad demo in each language', () => {
  const LANGS = ['fr', 'en'] as const;
  const story = (lang: 'fr' | 'en', t: number, answers: Answer[] = []) => squadAt(t, answers, lang);

  it('tells the same story whatever the language: same statuses, same numbers, same questions', () => {
    const answers: Answer[] = [{ at: firstAsk + 0.5, option: 1 }];
    for (const t of times(0, 2 * CYCLE, 0.5)) {
      const fr = story('fr', t, answers);
      const en = story('en', t, answers);
      expect(
        en.agents.map((a) => [a.status, a.model, a.tokens, a.cost, a.ms, a.ticket?.loop]),
        `t=${t}`,
      ).toEqual(fr.agents.map((a) => [a.status, a.model, a.tokens, a.cost, a.ms, a.ticket?.loop]));
      expect(en.counts).toEqual(fr.counts);
      expect(en.today).toBe(fr.today);
      expect(en.projects).toEqual(fr.projects);
    }
  });

  it('speaks French when the language is not given', () => {
    for (const t of times(0, CYCLE, 1)) expect(squadAt(t)).toEqual(story('fr', t));
  });

  it('names the agents, the tickets and the results in the language of the page', () => {
    const fr = story('fr', SNAPSHOT);
    const en = story('en', SNAPSHOT);
    expect(fr.agents.map((a) => a.name)).toEqual(['refacto-auth', 'tests-e2e', 'dem-4-paginer-les-users', 'docs-api', 'fix-login']);
    expect(en.agents.map((a) => a.name)).toEqual(['refactor-auth', 'e2e-tests', 'dem-4-paginate-users', 'docs-api', 'fix-login']);
    expect(fr.agents[3].result).toBe('2 fichiers modifiés');
    expect(en.agents[3].result).toBe('2 files changed');
    expect(asking(fr)!.question).toEqual({ text: 'Lancer toute la suite e2e (38 tests) ?', options: ['Oui', 'Seulement auth'] });
    expect(asking(en)!.question).toEqual({ text: 'Run the whole e2e suite (38 tests)?', options: ['Yes', 'Auth only'] });
  });

  it('names the finished runs and the tickets ready for review in English too', () => {
    const results = (lang: 'fr' | 'en', answers: Answer[]) =>
      new Set(times(0, CYCLE, 1).flatMap((t) => story(lang, t, answers).agents.map((a) => a.result)));
    expect(results('en', [])).toEqual(new Set([null, '2 files changed', '38 tests passed', 'Criteria met · ready for review']));
    expect(results('en', [{ at: firstAsk + 0.5, option: 1 }])).toContain('12 tests passed');
    expect(results('fr', [])).toEqual(new Set([null, '2 fichiers modifiés', '38 tests passés', 'Critères atteints · à tester']));
  });

  it('takes the next tickets in the language of the page', () => {
    const names = (lang: 'fr' | 'en') => [...new Set(times(0, 3 * CYCLE, 1).map((t) => ticketAgent(story(lang, t)).name))];
    expect(names('en')).toEqual(['dem-4-paginate-users', 'dem-5-filter-by-role', 'dem-6-export-to-csv', 'dem-7-paginate-users']);
    expect(names('fr')).toEqual(['dem-4-paginer-les-users', 'dem-5-filtrer-par-role', 'dem-6-exporter-en-csv', 'dem-7-paginer-les-users']);
  });

  it('keeps tokens and cost growing in English as well', () => {
    for (const lang of LANGS) {
      const before = story(lang, 0);
      const after = story(lang, CYCLE / 2);
      expect(after.today).toBeGreaterThan(before.today);
    }
  });
});
