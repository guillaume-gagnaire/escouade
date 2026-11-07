import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import {
  ageOf,
  asksMe,
  chipsOf,
  ciOf,
  filterCounts,
  filterPrs,
  forkBranch,
  forkOwner,
  isMine,
  moveIn,
  needOf,
  overviewPrs,
  ownerOf,
  rowTone,
  saveFilter,
  savedFilter,
} from './prlist';
import { agent, pullRequest as pr, ticket } from '../test/ipc';
import type { TicketPr } from './types';

const NOW = 1_793_260_800_000;

beforeEach(() => {
  setLang('fr');
  localStorage.clear();
});

describe('the filters of the project’s pull requests (ESC-65)', () => {
  const prs = [
    pr({ number: 1, author: 'ada' }),
    pr({ number: 2, author: 'Bob', reviewers: ['ADA', 'eve'] }),
    pr({ number: 3, author: 'eve', reviewers: ['bob'] }),
    pr({ number: 4 }),
  ];

  it('keep mine, those whose review is asked of me, or all, whatever the case of the login', () => {
    expect(filterPrs(prs, 'mine', 'ada').map((p) => p.number)).toEqual([1]);
    expect(filterPrs(prs, 'review', 'ada').map((p) => p.number)).toEqual([2]);
    expect(filterPrs(prs, 'all', 'ada')).toHaveLength(4);
    expect(filterCounts(prs, 'ada')).toEqual({ mine: 1, review: 1, all: 4 });
    expect(isMine(prs[1], 'bob')).toBe(true);
    expect(asksMe(prs[2], 'Bob')).toBe(true);
  });

  it('keep none of mine without the login of the account', () => {
    expect(filterCounts(prs, '')).toEqual({ mine: 0, review: 0, all: 4 });
  });

  it('remember the one chosen last, « Toutes » at first', () => {
    expect(savedFilter()).toBe('all');
    saveFilter('review');
    expect(savedFilter()).toBe('review');
  });
});

describe('a row of the list', () => {
  it('says its CI in words, and its tone', () => {
    expect(ciOf(pr({ checks: 'success' }))).toMatchObject({ text: 'CI verte', tone: 'ok', icon: 'check' });
    expect(ciOf(pr({ checks: 'failure' }))).toMatchObject({ text: 'CI en échec', tone: 'fail' });
    expect(ciOf(pr({ checks: 'pending' }))).toMatchObject({ text: 'CI en cours', tone: 'wait' });
    expect(ciOf(pr({ checks: null }))).toMatchObject({ text: 'Pas de CI', tone: 'dim' });
    expect(rowTone(pr({ checks: 'success', mergeable: false }))).toBe('fail');
    expect(rowTone(pr({ checks: 'success', changesRequested: true }))).toBe('wait');
    expect(rowTone(pr({ checks: 'failure', draft: true }))).toBe('dim');
  });

  it('says its conflict and its reviews, the most pressing first', () => {
    const p = pr({ mergeable: false, changesRequested: true, approvals: 2, openThreads: 1, reviewers: ['ada'] });
    expect(chipsOf(p, 'ada').map((c) => c.text)).toEqual([
      'Conflit',
      'Changements demandés',
      '2 approbations',
      '1 fil ouvert',
      'Ta relecture',
    ]);
    expect(chipsOf(pr({ mergeable: null }), 'ada')).toEqual([]);
  });

  it('says its age, short', () => {
    expect(ageOf(pr({ created: NOW / 1000 - 3 * 3600 }), NOW)).toBe('3 h');
    expect(ageOf(pr({ created: undefined }), NOW)).toBe('');
  });

  it('marks a fork’s with its owner, and names its local branch with the host’s word', () => {
    expect(forkOwner(pr({ fork: true, headRepo: 'marta/demo' }))).toBe('marta');
    expect(forkOwner(pr({ fork: false }))).toBeNull();
    expect(forkBranch('PR', 127)).toBe('pr-127');
    expect(forkBranch('MR', 3)).toBe('mr-3');
  });

  it('knows the agent already on it: by its pull requests, its branch, or a fork’s local branch', () => {
    const wt = (branch: string) => ({ path: 'C:\\wt', branch, baseBranch: 'main', existing: true });
    const onHead = agent({ id: 'a1', worktree: wt('feat/login') });
    const onFork = agent({ id: 'a2', worktree: wt('pr-9') });
    const archived = agent({ id: 'a3', worktree: wt('feat/x'), archived: true });
    const agents = [onHead, onFork, archived];
    expect(ownerOf(agents, pr({ head: 'feat/login' }), 'PR')?.id).toBe('a1');
    expect(ownerOf(agents, pr({ number: 9, head: 'whatever', fork: true }), 'PR')?.id).toBe('a2');
    // A fork's branch of the same name as an agent's is not that agent's.
    expect(ownerOf(agents, pr({ number: 5, head: 'feat/login', fork: true }), 'PR')).toBeNull();
    expect(ownerOf(agents, pr({ head: 'feat/x' }), 'PR')).toBeNull();
  });
});

describe('the keyboard in the list', () => {
  it('moves within the rows', () => {
    expect(moveIn(3, -1, 'ArrowDown')).toBe(0);
    expect(moveIn(3, -1, 'ArrowUp')).toBe(2);
    expect(moveIn(3, 2, 'ArrowDown')).toBe(2);
    expect(moveIn(3, 0, 'ArrowUp')).toBe(0);
    expect(moveIn(3, 1, 'End')).toBe(2);
    expect(moveIn(3, 1, 'Home')).toBe(0);
    expect(moveIn(0, 0, 'ArrowDown')).toBe(-1);
  });
});

describe('in English', () => {
  it('says the same', () => {
    setLang('en');
    expect(ciOf(pr({ checks: null })).text).toBe('No CI');
    expect(chipsOf(pr({ mergeable: false, reviewers: ['ada'] }), 'ada').map((c) => c.text)).toEqual(['Conflict', 'Your review']);
  });
});

describe('the group « Pull requests » of the overview (ESC-68)', () => {
  const list = (prs: ReturnType<typeof pr>[], over = {}) => ({ projectId: 'p1', word: 'PR', me: 'ada', prs, ...over });
  const inReview = (number: number, over: Partial<TicketPr> = {}) =>
    ticket({
      column: 'pr',
      agentId: null,
      pr: {
        number,
        word: 'PR',
        state: 'open',
        draft: false,
        approvals: 0,
        changesRequested: false,
        openThreads: 0,
        checks: null,
        closed: false,
        ...over,
      } as TicketPr,
    });

  it('tells what a pull request waits for', () => {
    expect(needOf(pr({ checks: 'failure' }))).toBe('fix');
    expect(needOf(pr({ changesRequested: true, approvals: 1 }))).toBe('fix');
    expect(needOf(pr({ mergeable: false }))).toBe('fix');
    expect(needOf(pr({ approvals: 1, checks: 'success' }))).toBe('merge');
    expect(needOf(pr({ approvals: 1, checks: null }))).toBe('merge');
    expect(needOf(pr({ approvals: 1, checks: 'pending' }))).toBeNull();
    expect(needOf(pr({ approvals: 0, checks: 'success' }))).toBeNull();
    expect(needOf(pr({ draft: true, checks: 'failure' }))).toBeNull();
    expect(needOf(pr({ state: 'merged', approvals: 1 }))).toBeNull();
  });

  it('gathers the tickets, the agents and the lists, once each, the most pressing first', () => {
    const a = agent({ id: 'a1', projectId: 'p1', prs: [pr({ number: 7, checks: 'failure' })] });
    const t = inReview(12, { approvals: 1, checks: 'success' });
    const lines = overviewPrs(
      [
        list([
          pr({ number: 7, author: 'ada', checks: 'failure' }),
          pr({ number: 12, author: 'ada' }),
          pr({ number: 20, author: 'bob', reviewers: ['Ada'] }),
          pr({ number: 21, author: 'ada', approvals: 1 }),
          pr({ number: 22, author: 'bob', approvals: 1 }),
          pr({ number: 23, author: 'ada', reviewers: ['ada'] }),
        ]),
      ],
      [a],
      [t],
      ['p1'],
    );
    expect(lines.map((o) => [o.need, o.pr.number])).toEqual([
      ['fix', 7],
      ['merge', 21],
      ['merge', 12],
      ['review', 20],
    ]);
    expect(lines[0].agent?.id).toBe('a1');
    expect(lines[2].ticket?.id).toBe(t.id);
    expect(lines[2].pr.title).toBe(t.title);
    expect(lines[3].agent).toBeNull();
  });

  it('leaves out an archived agent, a closed pull request and a project that is not open', () => {
    const gone = agent({ id: 'a1', projectId: 'p1', archived: true, prs: [pr({ number: 7, checks: 'failure' })] });
    const closed = inReview(12, { checks: 'failure', closed: true });
    expect(overviewPrs([], [gone], [closed], ['p1'])).toEqual([]);
    const elsewhere = agent({ id: 'a2', projectId: 'p9', prs: [pr({ number: 8, checks: 'failure' })] });
    expect(overviewPrs([list([pr({ author: 'ada', checks: 'failure' })], { projectId: 'p9' })], [elsewhere], [], ['p1'])).toEqual([]);
  });

  it('keeps apart two pull requests of the same number in two repositories', () => {
    const a = agent({
      projectId: 'p1',
      prs: [pr({ number: 7, checks: 'failure' }), pr({ number: 7, checks: 'failure', hostRepo: 'acme/up' })],
    });
    expect(overviewPrs([], [a], [], ['p1'])).toHaveLength(2);
  });
});
