// The filter of the box « À toi » by type (ESC-578): the types and how they gather, their numbers, the rows kept and
// hidden, the choice kept on this machine.
import { describe, expect, it } from 'vitest';
import {
  FILTER_KEY,
  hiddenCount,
  INBOX_TYPES,
  keep,
  keepAsks,
  keepViews,
  readFilter,
  toggle,
  typeCounts,
  typeOf,
  writeFilter,
  type FilterRow,
} from './filter';
import type { ViewSource } from './views';

const ask = (kind: 'choice' | 'confirm' | 'answer' | 'task', gate = false): FilterRow => ({ kind: 'ask', ask: { kind, gate } });
const view = (source: ViewSource): FilterRow => ({ kind: 'view', view: { source } });
const stopped = (r: 'permission' | 'question' | 'plan' | 'delete'): FilterRow => ({
  kind: 'stopped',
  agent:
    r === 'delete'
      ? { requests: [], deleteAsk: { by: 'a', at: 0, worktree: false } as never }
      : {
          requests: [{ kind: r === 'question' ? 'question' : 'permission', tool: r === 'plan' ? 'ExitPlanMode' : 'Bash' } as never],
          deleteAsk: null as never,
        },
});

class Mem {
  data = new Map<string, string>();
  getItem = (k: string) => this.data.get(k) ?? null;
  setItem = (k: string, v: string) => void this.data.set(k, v);
  removeItem = (k: string) => void this.data.delete(k);
}

describe('the types of the box', () => {
  it('types every row, and gathers what the user would not tell apart', () => {
    expect(typeOf(ask('choice'))).toBe('choice');
    expect(typeOf(ask('task'))).toBe('task');
    // A go-ahead stays apart; a plain yes or no and a free answer are questions.
    expect(typeOf(ask('confirm', true))).toBe('gate');
    expect(typeOf(ask('confirm'))).toBe('question');
    expect(typeOf(ask('answer'))).toBe('question');
    expect(typeOf(view('permission'))).toBe('permission');
    expect(typeOf(view('question'))).toBe('question');
    expect(typeOf(view('plan'))).toBe('plan');
    // The tickets to approve and the blocked ones: « Tickets ».
    expect(typeOf(view('ticketReview'))).toBe('tickets');
    expect(typeOf(view('ticketBlocked'))).toBe('tickets');
    expect(typeOf(view('decision'))).toBe('decision');
    expect(typeOf(view('docs'))).toBe('docs');
    expect(typeOf(view('review'))).toBe('review');
    // An agent stopped with no view: its request; a deletion it asks for is a permission.
    expect(typeOf(stopped('permission'))).toBe('permission');
    expect(typeOf(stopped('question'))).toBe('question');
    expect(typeOf(stopped('plan'))).toBe('plan');
    expect(typeOf(stopped('delete'))).toBe('permission');
  });

  it('counts each type present, in the filter’s order, leaves out a type with none unless it is chosen', () => {
    const rows = [
      view('decision'),
      ask('confirm', true),
      view('permission'),
      view('ticketBlocked'),
      view('ticketReview'),
      stopped('delete'),
    ];
    expect(typeCounts(rows, [])).toEqual([
      { type: 'permission', count: 2, on: false },
      { type: 'gate', count: 1, on: false },
      { type: 'tickets', count: 2, on: false },
      { type: 'decision', count: 1, on: false },
    ]);
    // Chosen, a type stays offered at 0 (its last request answered), so that it can be taken off.
    expect(typeCounts(rows, ['gate', 'docs']).filter((c) => c.on)).toEqual([
      { type: 'gate', count: 1, on: true },
      { type: 'docs', count: 0, on: true },
    ]);
    expect(typeCounts([], [])).toEqual([]);
  });
});

describe('the filter', () => {
  const rows = [view('permission'), ask('choice'), ask('confirm', true), view('decision'), ask('answer')];

  it('keeps the rows of the types chosen, in their order; none chosen, all', () => {
    expect(keep(rows, [])).toEqual(rows);
    expect(keep(rows, ['question', 'permission'])).toEqual([rows[0], rows[4]]);
    expect(
      keepAsks(
        [
          { kind: 'choice', gate: false, id: 1 },
          { kind: 'task', gate: false, id: 2 },
        ],
        ['task'],
      ),
    ).toEqual([{ kind: 'task', gate: false, id: 2 }]);
    expect(keepViews([{ source: 'ticketReview' as const }, { source: 'docs' as const }], ['tickets'])).toEqual([
      { source: 'ticketReview' },
    ]);
  });

  it('counts what it hides, nothing without a filter', () => {
    expect(hiddenCount(rows, [])).toBe(0);
    expect(hiddenCount(rows, ['gate'])).toBe(4);
    expect(hiddenCount(rows, INBOX_TYPES)).toBe(0);
  });

  it('chooses several types at once, in a fixed order, and takes one off again', () => {
    let f = toggle([], 'decision');
    f = toggle(f, 'permission');
    expect(f).toEqual(['permission', 'decision']);
    expect(toggle(f, 'permission')).toEqual(['decision']);
  });
});

describe('the choice kept on this machine', () => {
  it('is read back as it was written, « Tout » leaving nothing behind', () => {
    const m = new Mem();
    writeFilter(m, ['gate', 'tickets']);
    expect(m.getItem(FILTER_KEY)).toBe('["gate","tickets"]');
    expect(readFilter(m)).toEqual(['gate', 'tickets']);
    writeFilter(m, []);
    expect(m.data.has(FILTER_KEY)).toBe(false);
    expect(readFilter(m)).toEqual([]);
  });

  it('drops what it does not know, and falls back to « Tout » when unreadable or absent', () => {
    const m = new Mem();
    m.setItem(FILTER_KEY, '["gate","invitation",3]');
    expect(readFilter(m)).toEqual(['gate']);
    m.setItem(FILTER_KEY, '{oops');
    expect(readFilter(m)).toEqual([]);
    m.setItem(FILTER_KEY, '{"a":1}');
    expect(readFilter(m)).toEqual([]);
    expect(readFilter(null)).toEqual([]);
    const broken = {
      getItem: () => {
        throw new Error('denied');
      },
      setItem: () => {
        throw new Error('denied');
      },
      removeItem: () => {},
    };
    expect(readFilter(broken)).toEqual([]);
    expect(() => writeFilter(broken, ['gate'])).not.toThrow();
  });
});
