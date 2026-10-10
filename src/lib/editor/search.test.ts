import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeBackend } from '../../test/ipc';
import { setLang } from '../i18n';
import type { SearchQuery, SearchResult } from '../types';
import { FileSearch, fileSearches, findInFiles, pieces, setFindInFiles } from './search.svelte';

const found = (path = 'a.ts'): SearchResult => ({
  matches: [{ path, line: 1, col: 1, text: 'foo', offset: 0, ranges: [[0, 3]] }],
  truncated: false,
  timedOut: false,
});

/** A value to hand out later, to make an answer come after what happens in between. */
const deferred = <T>() => {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => ((resolve = res), (reject = rej)));
  return { promise, resolve, reject };
};

const query = (pattern: string, o: Partial<SearchQuery> = {}): SearchQuery => ({
  pattern,
  regex: false,
  caseSensitive: false,
  wholeWord: false,
  maxResults: 2000,
  ...o,
});

describe('FileSearch', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('searches 250 ms after the last change, once', async () => {
    const run = vi.fn(async () => found());
    const s = new FileSearch(run);
    s.type('f');
    await vi.advanceTimersByTimeAsync(200);
    s.type('fo');
    s.type('foo');
    expect(s.pending).toBe(true);
    await vi.advanceTimersByTimeAsync(249);
    expect(run).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(run).toHaveBeenCalledExactlyOnceWith('project', query('foo'));
    expect(s.result).toEqual(found());
    expect(s.pending).toBe(false);
  });

  it('sends the options as they are set, in the source shown', async () => {
    const run = vi.fn(async () => found());
    const s = new FileSearch(run);
    s.setSource('a1');
    s.type('x');
    s.toggle('caseSensitive');
    s.toggle('wholeWord');
    s.toggle('regex');
    await vi.advanceTimersByTimeAsync(250);
    expect(run).toHaveBeenCalledExactlyOnceWith('a1', query('x', { regex: true, caseSensitive: true, wholeWord: true }));
  });

  it('keeps only the answer of the last search, whichever comes first', async () => {
    const answers = [deferred<SearchResult>(), deferred<SearchResult>(), deferred<SearchResult>(), deferred<SearchResult>()];
    let n = 0;
    const s = new FileSearch(() => answers[n++].promise);
    s.type('a');
    await vi.advanceTimersByTimeAsync(250);
    s.type('ab');
    await vi.advanceTimersByTimeAsync(250);
    // The second answers first, then the first, which was overtaken.
    answers[1].resolve(found('ab.ts'));
    answers[0].resolve(found('a.ts'));
    await vi.advanceTimersByTimeAsync(0);
    expect(s.result).toEqual(found('ab.ts'));
    // A failure overtaken is not told either.
    s.type('abc');
    await vi.advanceTimersByTimeAsync(250);
    s.type('abcd');
    await vi.advanceTimersByTimeAsync(250);
    answers[2].reject('boom');
    answers[3].resolve(found('abcd.ts'));
    await vi.advanceTimersByTimeAsync(0);
    expect(s.result).toEqual(found('abcd.ts'));
    expect(s.error).toBeNull();
  });

  it('says why in English when the language of the interface is', async () => {
    setLang('en');
    const s = new FileSearch(vi.fn(async () => found()));
    s.toggle('regex');
    s.type('foo(');
    await vi.advanceTimersByTimeAsync(250);
    expect(s.error).toBe('Invalid regular expression.');
  });

  it('sends no invalid expression, and says why', async () => {
    const run = vi.fn(async () => found());
    const s = new FileSearch(run);
    s.toggle('regex');
    s.type('foo(');
    await vi.advanceTimersByTimeAsync(250);
    expect(run).not.toHaveBeenCalled();
    expect(s.error).toBe('Expression régulière invalide.');
    expect(s.result).toBeNull();
    expect(s.pending).toBe(false);
    // As plain text, it is searched as written.
    s.toggle('regex');
    await vi.advanceTimersByTimeAsync(250);
    expect(run).toHaveBeenCalledExactlyOnceWith('project', query('foo('));
    expect(s.error).toBeNull();
  });

  it('tells what git refused, in place of the results', async () => {
    const refusal = 'Cette expression n’est pas prise en charge par git sur ce poste : \\b';
    let refuse = false;
    const s = new FileSearch(async () => (refuse ? Promise.reject(refusal) : found()));
    s.ask({ text: 'foo' });
    await vi.advanceTimersByTimeAsync(0);
    expect(s.result).toEqual(found());
    refuse = true;
    s.ask({ text: '\\bfoo', regex: true });
    await vi.advanceTimersByTimeAsync(0);
    expect(s.error).toBe(refusal);
    expect(s.result).toBeNull();
  });

  it('starts no search while the same one is under way', async () => {
    const answer = deferred<SearchResult>();
    const run = vi.fn(() => answer.promise);
    const s = new FileSearch(run);
    s.ask({ text: 'foo' });
    s.now();
    s.type('foo');
    await vi.advanceTimersByTimeAsync(250);
    expect(run).toHaveBeenCalledTimes(1);
    answer.resolve(found());
    await vi.advanceTimersByTimeAsync(0);
    // Asked again once it answered: searched again, the files may have changed.
    s.now();
    expect(run).toHaveBeenCalledTimes(2);
  });

  it('sends nothing for a change undone within the pause', async () => {
    const run = vi.fn(async () => found());
    const s = new FileSearch(run);
    s.ask({ text: 'foo' });
    await vi.advanceTimersByTimeAsync(0);
    s.toggle('wholeWord');
    s.toggle('wholeWord');
    s.type('fooo');
    s.type('foo');
    await vi.advanceTimersByTimeAsync(250);
    expect(run).toHaveBeenCalledTimes(1);
    expect(s.pending).toBe(false);
  });

  it('drops an answer that comes for a search given up meanwhile', async () => {
    const answer = deferred<SearchResult>();
    const s = new FileSearch(() => answer.promise);
    s.ask({ text: 'foo' });
    s.toggle('regex');
    s.type('foo(');
    await vi.advanceTimersByTimeAsync(250);
    answer.resolve(found());
    await vi.advanceTimersByTimeAsync(0);
    expect(s.result).toBeNull();
    expect(s.error).toBe('Expression régulière invalide.');
  });

  it('clears the results as soon as the field is emptied, and drops the answer under way', async () => {
    const answers = [deferred<SearchResult>(), deferred<SearchResult>()];
    let n = 0;
    const run = vi.fn(() => answers[n++].promise);
    const s = new FileSearch(run);
    s.ask({ text: 'foo' });
    answers[0].resolve(found());
    await vi.advanceTimersByTimeAsync(0);
    s.type('foox');
    await vi.advanceTimersByTimeAsync(250);
    s.type('');
    expect(s.result).toBeNull();
    expect(s.pending).toBe(false);
    answers[1].resolve(found('late.ts'));
    await vi.advanceTimersByTimeAsync(300);
    expect(s.result).toBeNull();
    expect(run).toHaveBeenCalledTimes(2);
  });

  it('searches again in another source, the results of the previous one gone', async () => {
    const answer = deferred<SearchResult>();
    const run = vi.fn((source: string) => (source === 'project' ? Promise.resolve(found()) : answer.promise));
    const s = new FileSearch(run);
    s.ask({ text: 'foo' });
    await vi.advanceTimersByTimeAsync(0);
    s.setSource('a1');
    expect(s.result).toBeNull();
    expect(s.pending).toBe(true);
    expect(run).toHaveBeenLastCalledWith('a1', query('foo'));
    s.setSource('a1');
    expect(run).toHaveBeenCalledTimes(2);
    answer.resolve(found('wt.ts'));
    await vi.advanceTimersByTimeAsync(0);
    expect(s.result).toEqual(found('wt.ts'));
  });

  it('takes a selection as the text to find, escaped when it is read as an expression', async () => {
    const run = vi.fn(async () => found());
    const s = new FileSearch(run);
    s.seed('a.b(c)');
    expect(s.text).toBe('a.b(c)');
    expect(run).toHaveBeenLastCalledWith('project', query('a.b(c)'));
    s.toggle('regex');
    s.seed('a.b(c)');
    expect(s.text).toBe('a\\.b\\(c\\)');
    expect(run).toHaveBeenLastCalledWith('project', query('a\\.b\\(c\\)', { regex: true }));
  });

  it('stops for good once disposed', async () => {
    const answer = deferred<SearchResult>();
    const run = vi.fn(() => answer.promise);
    const s = new FileSearch(run);
    s.ask({ text: 'foo' });
    s.type('bar');
    s.dispose();
    answer.resolve(found());
    await vi.advanceTimersByTimeAsync(300);
    expect(run).toHaveBeenCalledTimes(1);
    expect(s.result).toBeNull();
  });
});

describe('fileSearches', () => {
  beforeEach(() => fileSearches.reset());

  it('keeps one search per project, asked of code_search in the source shown', async () => {
    const be = fakeBackend({ code_search: () => found() });
    const s = fileSearches.of('p1');
    expect(fileSearches.of('p1')).toBe(s);
    expect(fileSearches.of('p2')).not.toBe(s);
    s.ask({ text: 'foo' });
    s.setSource('a1');
    await vi.waitFor(() => expect(s.result).toEqual(found()));
    expect(be.called('code_search').map((c) => c.args)).toEqual([
      { projectId: 'p1', agentId: null, query: query('foo') },
      { projectId: 'p1', agentId: 'a1', query: query('foo') },
    ]);
    fileSearches.closeProject('p1');
    expect(fileSearches.of('p1')).not.toBe(s);
  });
});

describe('findInFiles', () => {
  it('goes to the editor that set it, and to none once it is gone', () => {
    const open = vi.fn();
    const unset = setFindInFiles(open);
    expect(findInFiles()).toBe(true);
    expect(open).toHaveBeenCalledTimes(1);
    // Another editor took over: the first one going does not take it away.
    const other = vi.fn();
    const unsetOther = setFindInFiles(other);
    unset();
    expect(findInFiles()).toBe(true);
    expect(other).toHaveBeenCalledTimes(1);
    unsetOther();
    expect(findInFiles()).toBe(false);
  });
});

describe('pieces', () => {
  const at = (text: string, ranges: [number, number][], offset = 0) =>
    pieces({ text, offset, ranges })
      .map((p) => (p.hit ? `[${p.text}]` : p.text))
      .join('');

  it('marks each match, the indentation left out', () => {
    expect(
      at('    foo bar foo', [
        [4, 7],
        [12, 15],
      ]),
    ).toBe('[foo] bar [foo]');
    expect(at('\tx = 1', [[1, 2]])).toBe('[x] = 1');
  });

  it('counts the ranges in characters: an emoji is one, and two units of a JavaScript string', () => {
    expect(at('😀 é foo', [[4, 7]])).toBe('😀 é [foo]');
    expect(at('a😀b', [[1, 2]])).toBe('a[😀]b');
  });

  it('shows the line without marks when git gave none, all of it from its indentation', () => {
    expect(at('  foo()', [])).toBe('foo()');
    // A coloured line, or an expression matching nothing but an empty string (`^`, `x*`): no match to bring into view.
    expect(at('    const veryLongIdentifierName = computeSomething(argument);', [])).toBe(
      'const veryLongIdentifierName = computeSomething(argument);',
    );
    expect(at('    const veryLongIdentifierName = computeSomething(argument);', [[4, 4]])).toBe(
      'const veryLongIdentifierName = computeSomething(argument);',
    );
    // A stretch of a long line: it goes on before.
    expect(at('  veryLongIdentifierName = computeSomething(argument);', [], 360)).toBe(
      '…veryLongIdentifierName = computeSomething(argument);',
    );
  });

  it('joins the ranges that overlap, in order, inside the text', () => {
    expect(
      at('abcdef', [
        [3, 5],
        [0, 2],
        [1, 3],
        [4, 99],
        [-2, 0],
        [2, 2],
      ]),
    ).toBe('[abcdef]');
    expect(
      at('abcdef', [
        [4, 5],
        [0, 1],
      ]),
    ).toBe('[a]bcd[e]f');
  });

  it('never leaves out a match made of spaces', () => {
    expect(at('    x', [[0, 2]])).toBe('[  ]  x');
  });

  it('starts with … when the line goes on before what is shown', () => {
    expect(at('ab foo', [[3, 6]], 120)).toBe('…ab [foo]');
    // A match far in a long line: the characters just before it are kept.
    expect(at(`${'x'.repeat(40)}0123456789abcdeffoo`, [[56, 59]])).toBe('…0123456789abcdef[foo]');
    expect(at(`${'é'.repeat(16)}foo`, [[16, 19]])).toBe(`${'é'.repeat(16)}[foo]`);
  });
});
