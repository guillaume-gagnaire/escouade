import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import {
  keptNotes,
  mergeStops,
  noteOrder,
  notePlace,
  onItsLine,
  placeNotes,
  reviewMark,
  severityCounts,
  shownBy,
  sortNotes,
  suggestionDiff,
} from './prreview';
import type { AgentReview, PrContentsFile, ReviewNote } from './types';

// The reviewer agent's remarks as the window places, counts, sorts and publishes them (ESC-490).

const FILE = (path: string, lines: PrContentsFile['lines']): PrContentsFile => ({
  path,
  status: 'modified',
  additions: 1,
  deletions: 0,
  lines,
  binary: false,
  generated: false,
  cut: false,
});
const A = FILE('a.ts', [
  { kind: '@', text: '@@' },
  { kind: '+', new: 1, text: 'un' },
  { kind: ' ', old: 1, new: 2, text: 'deux' },
  { kind: '-', old: 2, text: 'trois' },
]);
const B = FILE('b.ts', [{ kind: '+', new: 1, text: 'b' }]);

const n = (over: Partial<ReviewNote>): ReviewNote => ({
  id: 1,
  path: 'a.ts',
  line: 1,
  severity: 'detail',
  body: 'x',
  proposed: 'x',
  choice: 'open',
  excerpt: [],
  outside: false,
  code: [],
  stale: false,
  ...over,
});

beforeEach(() => setLang('fr'));

describe('the agent’s remarks (ESC-490)', () => {
  it('are placed under their line while their code is there, else apart; on a file, on the pull request; never once published', () => {
    const notes = [
      n({ id: 1, line: 2, code: ['deux'] }),
      n({ id: 2, line: 1, code: ['autre'] }),
      n({ id: 3, line: 0 }),
      n({ id: 4, path: '' }),
      n({ id: 5, path: 'gone.ts' }),
      n({ id: 6, line: 2, stale: true }),
      n({ id: 7, line: 2, choice: 'published' }),
      n({ id: 8, line: 9 }),
    ];
    const p = placeNotes([A, B], notes);
    expect(p.under['a.ts'][2].map((x) => x.id)).toEqual([1]);
    expect(p.stale['a.ts'].map((x) => x.id)).toEqual([2, 6, 8]);
    expect(p.onFile['a.ts'].map((x) => x.id)).toEqual([3]);
    expect(p.top.map((x) => x.id)).toEqual([4, 5]);
    expect(onItsLine([A], notes[0])).toBe(true);
    expect(onItsLine([A], notes[1])).toBe(false);
    expect(onItsLine([A], notes[3])).toBe(false);
  });

  it('are counted by severity while they wait, filtered, sorted', () => {
    const notes = [
      n({ id: 1, severity: 'detail', line: 2 }),
      n({ id: 2, severity: 'security', line: 3 }),
      n({ id: 3, severity: 'security', choice: 'dropped' }),
      n({ id: 4, severity: 'spec', path: '' }),
      n({ id: 5, severity: 'bug', path: 'b.ts' }),
    ];
    expect(severityCounts(notes)).toEqual({ security: 1, bug: 1, spec: 1, detail: 1 });
    expect(notes.filter(shownBy({})).length).toBe(5);
    expect(notes.filter(shownBy({ security: true, spec: true })).map((x) => x.id)).toEqual([2, 3, 4]);
    expect(sortNotes(notes, 'severity', ['a.ts', 'b.ts']).map((x) => x.id)).toEqual([3, 2, 5, 4, 1]);
    expect(sortNotes(notes, 'place', ['a.ts', 'b.ts']).map((x) => x.id)).toEqual([4, 3, 1, 2, 5]);
    expect(
      keptNotes([n({ id: 9, choice: 'kept', path: 'b.ts' }), n({ id: 8, choice: 'kept' }), n({ id: 7 })], ['a.ts', 'b.ts']).map(
        (x) => x.id,
      ),
    ).toEqual([8, 9]);
  });

  it('say their place, and their suggestion as a diff', () => {
    expect(notePlace(n({ line: 12, startLine: 10 }), 'PR')).toBe('a.ts:10-12');
    expect(notePlace(n({ line: 0 }), 'PR')).toBe('a.ts');
    expect(notePlace(n({ path: '' }), 'MR')).toBe('Sur la MR');
    expect(suggestionDiff(n({ code: ['a', 'b'], suggestion: 'c' }))).toEqual([
      { kind: '-', text: 'a' },
      { kind: '-', text: 'b' },
      { kind: '+', text: 'c' },
    ]);
    expect(suggestionDiff(n({}))).toEqual([]);
  });

  it('« Suivante » goes through the host’s threads and the agent’s, in the order of the tab, as chosen', () => {
    const placed = placeNotes(
      [A, B],
      [n({ id: 1, line: 2 }), n({ id: 2, path: '' }), n({ id: 3, path: 'b.ts' }), n({ id: 4, choice: 'dropped', line: 1 })],
    );
    const agent = noteOrder([A, B], placed);
    expect(agent.map((s) => s.key)).toEqual(['agent:2', 'agent:1', 'agent:3']);
    const host = [
      { key: 'T0', path: null, index: -1 },
      { key: 'T1', path: 'a.ts', index: 1 },
      { key: 'T2', path: 'b.ts', index: 0 },
    ];
    expect(mergeStops([A, B], host, agent, 'both').map((s) => s.key)).toEqual(['T0', 'agent:2', 'T1', 'agent:1', 'T2', 'agent:3']);
    expect(mergeStops([A, B], host, agent, 'host').map((s) => s.key)).toEqual(['T0', 'T1', 'T2']);
    expect(mergeStops([A, B], host, agent, 'agent').map((s) => s.key)).toEqual(['agent:2', 'agent:1', 'agent:3']);
  });

  it('mark the row of their pull request', () => {
    const r = (over: Partial<AgentReview>) => ({ phase: 'ready', notes: [], ...over }) as AgentReview;
    expect(reviewMark(r({ phase: 'reading' })).text).toBe('Relecture en cours');
    expect(reviewMark(r({ phase: 'failed' })).tone).toBe('fail');
    expect(reviewMark(r({ phase: 'failed', notes: [n({})] })).text).toBe('Relecture : 1 remarque ouverte');
    expect(reviewMark(r({ notes: [n({ choice: 'published' })] })).text).toBe('Relecture : rien d’ouvert');
  });
});
