import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { dimensions, fCostOf, grainLine, lines, noticeLine, periodBounds, planMark, total, within } from './team-costs';
import { API, BOB, costsView, ME, record, ticketRecord, WEB } from '../test/team-costs';

// The table of « Équipe › Coûts » (TS-16): the periods, the sums by member, project, ticket, harness and model, the
// equivalent in API of a plan marked, and what each member is told of the team's choice (C15).

describe('the costs of a team (TS-16)', () => {
  beforeEach(() => setLang('fr'));

  it('bounds the week from Monday, the month from the 1st, and a chosen period both ends in', () => {
    // Thursday 20 November 2026.
    const today = new Date(2026, 10, 20);
    expect(periodBounds('week', today)).toEqual(['2026-11-16', '2026-11-20']);
    expect(periodBounds('month', today)).toEqual(['2026-11-01', '2026-11-20']);
    expect(periodBounds('custom', today, '2026-10-01', '2026-10-31')).toEqual(['2026-10-01', '2026-10-31']);
    // Given the wrong way round, put back.
    expect(periodBounds('custom', today, '2026-10-31', '2026-10-01')).toEqual(['2026-10-01', '2026-10-31']);
    // A Sunday's week starts the Monday before.
    expect(periodBounds('week', new Date(2026, 10, 22))[0]).toBe('2026-11-16');
    expect(within('2026-11-16', ['2026-11-16', '2026-11-20'])).toBe(true);
    expect(within('2026-11-21', ['2026-11-16', '2026-11-20'])).toBe(false);
  });

  it('sums by project, by harness and model, by ticket, within the period, the most costly first', () => {
    const v = costsView({
      records: [
        record({ cost: 1 }),
        record({ cost: 2, model: 'gpt-5.5', harness: 'codex' }),
        record({ project: WEB, cost: 4 }),
        record({ day: '2026-10-02', cost: 100 }),
      ],
      tickets: [ticketRecord({ cost: 0.5 }), ticketRecord({ ticket: 'API-13', cost: 2 })],
    });
    const nov: [string, string] = ['2026-11-01', '2026-11-30'];
    const byProject = lines(v, 'project', nov);
    expect(byProject.map((l) => [l.label, l.cost])).toEqual([
      ['web', 4],
      ['api', 3],
    ]);
    expect(total(byProject)).toEqual({ tokens: 6000, cost: 7, plan: false });
    expect(lines(v, 'model', nov).map((l) => l.label)).toEqual(['claude · claude-opus-4', 'codex · gpt-5.5']);
    expect(lines(v, 'ticket', nov).map((l) => [l.label, l.cost])).toEqual([
      ['API-13 · api', 2],
      ['API-12 · api', 0.5],
    ]);
    // October's 100 $ are out of November.
    expect(lines(v, 'project', ['2026-10-01', '2026-10-31'])[0].cost).toBe(100);
  });

  it('marks an equivalent in API for what ran on a plan, whole or in part', () => {
    const v = costsView({
      records: [record({ plan: true, cost: 3 }), record({ project: WEB, cost: 1 }), record({ project: WEB, plan: true, cost: 1 })],
    });
    const nov: [string, string] = ['2026-11-01', '2026-11-30'];
    const [api, web] = lines(v, 'project', nov);
    expect(api.label).toBe('api');
    expect(planMark(api)).toBe('équivalent API');
    expect(planMark(web)).toBe('en partie un équivalent API');
    expect(fCostOf(api.cost, api.plan)).toBe('≈ 3,00 $');
    expect(fCostOf(1, false)).toBe('1,00 $');
    expect(planMark({ plan: false, planOnly: false })).toBeNull();
  });

  it('offers « par membre » only to an administrator of a team that chose it, and names each member', () => {
    expect(dimensions(costsView())).toEqual(['project', 'ticket', 'model']);
    const chosen = costsView({ rules: { ...costsView().rules, byMember: true } });
    expect(dimensions(chosen)).toEqual(['member', 'project', 'ticket', 'model']);
    expect(dimensions({ ...chosen, admin: false })).toEqual(['project', 'ticket', 'model']);
    const v = costsView({
      rules: chosen.rules,
      records: [record({ member: ME, mine: true, cost: 1 }), record({ member: BOB, cost: 2 }), record({ member: null, cost: 0.5 })],
    });
    expect(lines(v, 'member', ['2026-11-01', '2026-11-30']).map((l) => [l.label, l.cost, l.mine])).toEqual([
      [`Membre ${BOB.slice(0, 8)}`, 2, false],
      ['Toi', 1, true],
      ['Sans nom (par projet)', 0.5, false],
    ]);
  });

  it('tells each member the team’s choice', () => {
    const v = costsView();
    expect(grainLine(v)).toBe('L’équipe ACME voit les totaux par projet : les tiens partent sans ton nom.');
    const member = { ...v, rules: { ...v.rules, byMember: true } };
    expect(grainLine(member)).toContain('tes totaux partent sans ton nom tant que tu n’as pas lu l’écran');
    expect(grainLine({ ...member, told: true })).toBe(
      'L’équipe ACME voit les totaux par membre : ses administrateurs voient tes totaux par jour, à ton nom.',
    );
  });

  it('asks before a start in a project near or beyond its budget', () => {
    expect(noticeLine(null)).toBeNull();
    const text = 'Le projet api a dépensé 92 % de son budget du mois';
    expect(noticeLine({ text, pct: 92, over: false, stops: false, team: 'ACME' })).toBe(`${text}. Le lancer quand même ?`);
    expect(noticeLine({ text, pct: 104, over: true, stops: true, team: 'ACME' })).toBe(
      `${text} : le pilote auto de ce projet est arrêté par l’équipe ACME. Le lancer quand même ?`,
    );
  });

  it('keeps the projects without a name known by the start of their journal', () => {
    const v = costsView({ projects: {}, records: [record({ project: API })] });
    expect(lines(v, 'project', ['2026-11-01', '2026-11-30'])[0].label).toBe(API.slice(0, 8));
  });
});
