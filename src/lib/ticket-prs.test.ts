import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { ciLine, joinNames, prColumnShown, prErrorLine, prFacts, prStateKey, prStateWord, splitNames } from './ticket-prs';
import { ticket, ticketPr } from '../test/ipc';

describe('the pull requests of the tickets (ESC-59)', () => {
  beforeEach(() => setLang('fr'));

  it('shows the column « En revue » when the board opens pull requests or holds a ticket there', () => {
    expect(prColumnShown({ action: 'pr' }, [])).toBe(true);
    expect(prColumnShown({ action: 'merge' }, [ticket({ column: 'done' })])).toBe(false);
    expect(prColumnShown({ action: 'merge' }, [ticket({ column: 'pr' })])).toBe(true);
  });

  it('tells its state in a word: draft, open, approved, changes requested, closed', () => {
    expect(prStateWord(ticketPr())).toBe('ouverte');
    expect(prStateWord(ticketPr({ draft: true, approvals: 1 }))).toBe('brouillon');
    expect(prStateWord(ticketPr({ approvals: 2 }))).toBe('approuvée');
    expect(prStateWord(ticketPr({ approvals: 1, changesRequested: true }))).toBe('changements demandés');
    expect(prStateKey(ticketPr({ closed: true }))).toBe('closed');
    setLang('en');
    expect(prStateWord(ticketPr({ approvals: 1 }))).toBe('approved');
  });

  it('tells its CI with the count of its jobs, and nothing without CI', () => {
    expect(ciLine(ticketPr())).toBeNull();
    expect(ciLine(ticketPr({ checks: 'success', checksTotal: 3 }))).toBe('CI verte · 3 jobs');
    expect(ciLine(ticketPr({ checks: 'pending', checksTotal: 1 }))).toBe('CI en cours · 1 job');
    expect(ciLine(ticketPr({ checks: 'failure', checksTotal: 4, checksFailed: 1 }))).toBe('1/4 jobs en échec');
    // Its jobs not counted yet: its CI in a word.
    expect(ciLine(ticketPr({ checks: 'failure' }))).toBe('CI en échec');
    setLang('en');
    expect(ciLine(ticketPr({ checks: 'success', checksTotal: 1 }))).toBe('CI passed · 1 job');
  });

  it('tells its approvals, its open threads and a conflict with its target', () => {
    expect(prFacts(ticketPr())).toEqual([]);
    expect(prFacts(ticketPr({ approvals: 1, openThreads: 2, mergeable: false }))).toEqual([
      '1 approbation',
      '2 fils ouverts',
      'conflit avec main',
    ]);
  });

  it('says an error of the host and since when, and nothing once it reads well', () => {
    const now = 1_000_000_000;
    expect(prErrorLine(ticketPr(), now)).toBeNull();
    expect(prErrorLine(ticketPr({ error: 'GitHub injoignable : délai dépassé', errorSince: now - 10 * 60_000 }), now)).toBe(
      'GitHub injoignable : délai dépassé · depuis 10 min',
    );
  });

  it('reads the reviewers and the labels typed by commas', () => {
    expect(splitNames(' @ada, grace ,, à relire ')).toEqual(['ada', 'grace', 'à relire']);
    expect(joinNames(['ada', 'grace'])).toBe('ada, grace');
    expect(joinNames(undefined)).toBe('');
  });
});
