import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import {
  canFixCi,
  canFixReviews,
  canUpdateBranch,
  canWaitGreen,
  ciLine,
  fixesSpent,
  fixingLine,
  joinNames,
  mergeActions,
  mergeHowText,
  mergeReady,
  mergeStrategy,
  mergeWaitLine,
  noLogLine,
  prColumnShown,
  prErrorLine,
  prFacts,
  prStateKey,
  prStateWord,
  splitNames,
} from './ticket-prs';
import { busy } from './board';
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

  it('offers « L’agent corrige » on a failed CI of an open one with an agent, none under way (ESC-60)', () => {
    const failing = ticketPr({ checks: 'failure' });
    expect(canFixCi(ticket({ column: 'pr', agentId: 'a1', pr: failing }))).toBe(true);
    expect(canFixCi(ticket({ column: 'pr', agentId: 'a1', pr: ticketPr({ checks: 'success' }) }))).toBe(false);
    expect(canFixCi(ticket({ column: 'pr', pr: failing }))).toBe(false);
    expect(canFixCi(ticket({ column: 'pr', agentId: 'a1', pr: { ...failing, fixing: 5 } }))).toBe(false);
    expect(canFixCi(ticket({ column: 'pr', agentId: 'a1', pr: { ...failing, closed: true } }))).toBe(false);
    expect(canFixCi(ticket({ column: 'review', agentId: 'a1', pr: failing }))).toBe(false);
  });

  it('says the failed checks of a CI that keeps no log, and the loops the autopilot spent', () => {
    expect(noLogLine(ticketPr())).toBeNull();
    const failed = [
      { name: 'test', url: null, log: true },
      { name: 'ci/jenkins', url: null, log: false },
      { name: 'circleci', url: null, log: false },
    ];
    expect(noLogLine(ticketPr({ failed }))).toBe('ci/jenkins, circleci : CI tierces, sans journal');
    setLang('en');
    expect(noLogLine(ticketPr({ failed: failed.slice(0, 2) }))).toBe('ci/jenkins: third-party CI, no log');
    const x = ticket({ column: 'pr', maxLoops: 3, pr: ticketPr({ fixes: 3 }) });
    expect(fixesSpent(x, true)).toBe(true);
    expect(fixesSpent(x, false)).toBe(false);
    expect(fixesSpent({ ...x, maxLoops: 5 }, true)).toBe(false);
  });

  it('offers « L’agent corrige les retours » on new feedback and « L’agent met à jour la branche » on a conflict, none under way (ESC-61)', () => {
    const fed = ticketPr({ feedback: 2 });
    expect(canFixReviews(ticket({ column: 'pr', agentId: 'a1', pr: fed }))).toBe(true);
    expect(canFixReviews(ticket({ column: 'pr', agentId: 'a1', pr: ticketPr() }))).toBe(false);
    expect(canFixReviews(ticket({ column: 'pr', pr: fed }))).toBe(false);
    expect(canFixReviews(ticket({ column: 'pr', agentId: 'a1', pr: { ...fed, fixing: 3 } }))).toBe(false);
    expect(canFixReviews(ticket({ column: 'pr', agentId: 'a1', pr: { ...fed, state: 'merged' } }))).toBe(false);
    const clash = ticketPr({ mergeable: false });
    expect(canUpdateBranch(ticket({ column: 'pr', agentId: 'a1', pr: clash }))).toBe(true);
    expect(canUpdateBranch(ticket({ column: 'pr', agentId: 'a1', pr: ticketPr({ mergeable: null }) }))).toBe(false);
    expect(canUpdateBranch(ticket({ column: 'pr', agentId: 'a1', pr: { ...clash, closed: true } }))).toBe(false);
    expect(canUpdateBranch(ticket({ column: 'pr', agentId: 'a1', pr: { ...clash, fixing: 3 } }))).toBe(false);
  });

  it('says what its agent does on its pull request, in both languages (ESC-61)', () => {
    expect(fixingLine(ticketPr())).toBe("L'agent corrige la CI…");
    expect(fixingLine(ticketPr({ fixKind: 'reviews' }))).toBe("L'agent corrige les retours…");
    expect(fixingLine(ticketPr({ fixKind: 'merge' }))).toBe("L'agent met à jour la branche…");
    expect(fixingLine(ticketPr({ fixKind: 'rebase' }))).toBe("L'agent rebase la branche sur main…");
    setLang('en');
    expect(fixingLine(ticketPr({ fixKind: 'reviews' }))).toBe('The agent is addressing the feedback…');
    expect(fixingLine(ticketPr({ fixKind: 'rebase' }))).toBe('The agent is rebasing the branch onto main…');
  });

  it('counts a ticket in review whose agent fixes its CI among the places taken', () => {
    expect(busy([ticket({ column: 'pr', pr: ticketPr() })])).toBe(0);
    expect(busy([ticket({ column: 'pr', pr: ticketPr({ fixing: 1 }) }), ticket({ column: 'doing' })])).toBe(2);
  });
});

describe('merging a ticket’s pull request from Escouade (ESC-62)', () => {
  beforeEach(() => setLang('fr'));
  const green = { approvals: 1, checks: 'success' as const, mergeable: true };

  it('is ready approved and green, without a conflict, a draft or changes asked for', () => {
    expect(mergeReady(ticketPr(green))).toBe(true);
    expect(mergeReady(ticketPr({ ...green, checks: null }))).toBe(true);
    expect(mergeReady(ticketPr({ ...green, approvals: 0 }))).toBe(false);
    expect(mergeReady(ticketPr({ ...green, checks: 'pending' }))).toBe(false);
    expect(mergeReady(ticketPr({ ...green, checks: 'failure' }))).toBe(false);
    expect(mergeReady(ticketPr({ ...green, mergeable: false }))).toBe(false);
    expect(mergeReady(ticketPr({ ...green, draft: true }))).toBe(false);
    expect(mergeReady(ticketPr({ ...green, changesRequested: true }))).toBe(false);
    expect(mergeReady(ticketPr({ ...green, state: 'merged' }))).toBe(false);
    // Its agent fixes its CI (ESC-60): nothing merges before the fix is pushed.
    expect(mergeReady(ticketPr({ ...green, fixing: 1 }))).toBe(false);
    expect(canWaitGreen(ticketPr({ checks: 'pending', fixing: 1 }))).toBe(false);
  });

  it('waits for green while the CI runs, once', () => {
    expect(canWaitGreen(ticketPr({ checks: 'pending' }))).toBe(true);
    expect(canWaitGreen(ticketPr({ checks: 'success' }))).toBe(false);
    expect(canWaitGreen(ticketPr({ checks: 'pending', autoMerge: true }))).toBe(false);
    expect(canWaitGreen(ticketPr({ checks: 'pending', mergeWhenGreen: 'merge' }))).toBe(false);
  });

  it('takes the Kanban’s strategy when the repository allows it, else the first it allows', () => {
    const opts = (strategies: ('merge' | 'squash' | 'rebase')[]) => ticketPr({ mergeOptions: { strategies, autoMerge: false } });
    expect(mergeStrategy(opts(['merge', 'squash']), 'squash')).toBe('squash');
    expect(mergeStrategy(opts(['rebase', 'merge']), 'squash')).toBe('rebase');
    expect(mergeStrategy(opts([]), 'squash')).toBeNull();
    expect(mergeStrategy(ticketPr(), 'squash')).toBeNull();
    expect(mergeActions(ticketPr({ ...green, mergeOptions: { strategies: [], autoMerge: false } }), 'squash')).toEqual({
      merge: null,
      whenGreen: false,
      none: true,
    });
  });

  it('says what merges it by itself, in French and in English', () => {
    expect(mergeWaitLine(ticketPr({ autoMerge: true, kind: 'gitlab', word: 'MR' }))).toBe(
      'Auto-merge de GitLab activé : mergée dès que la CI est verte',
    );
    expect(mergeWaitLine(ticketPr({ mergeWhenGreen: 'rebase' }))).toContain('Escouade la merge (rebase)');
    expect(mergeWaitLine(ticketPr())).toBeNull();
    expect(mergeHowText('escouade', ticketPr({ kind: 'bitbucket' }))).toContain('Bitbucket');
    setLang('en');
    expect(mergeWaitLine(ticketPr({ autoMerge: true }))).toBe('GitHub auto-merge on: merged as soon as the CI is green');
    expect(mergeHowText('merged', ticketPr())).toBe('PR #12 merged');
    expect(mergeHowText('escouade', ticketPr({ kind: 'bitbucket' }))).toBe(
      'No auto-merge on Bitbucket for this repository: Escouade waits for the end of the CI and merges itself, while it runs',
    );
  });
});
