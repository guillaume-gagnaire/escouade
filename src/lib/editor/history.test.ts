import { beforeEach, describe, expect, it } from 'vitest';
import { navHistory, type NavEntry } from './history';

const at = (path: string, line = 1, col = 1, source = 'project'): NavEntry => ({ projectId: 'p1', source, path, line, col });
const all = () => true;

describe('navHistory', () => {
  beforeEach(() => navHistory.reset());

  it('goes back to where each jump left from, then forward again', () => {
    navHistory.push(at('a.ts', 3, 2));
    navHistory.push(at('b.ts', 10));
    expect(navHistory.back(at('c.ts', 5), all)).toEqual(at('b.ts', 10));
    expect(navHistory.back(at('b.ts', 10), all)).toEqual(at('a.ts', 3, 2));
    expect(navHistory.back(at('a.ts', 3, 2), all)).toBeNull();
    expect(navHistory.forward(at('a.ts', 3, 2), all)).toEqual(at('b.ts', 10));
    expect(navHistory.forward(at('b.ts', 10), all)).toEqual(at('c.ts', 5));
    expect(navHistory.forward(at('c.ts', 5), all)).toBeNull();
  });

  it('forgets the way forward once a new jump is made', () => {
    navHistory.push(at('a.ts'));
    navHistory.back(at('b.ts'), all);
    navHistory.push(at('a.ts', 4));
    expect(navHistory.forward(at('d.ts'), all)).toBeNull();
    expect(navHistory.back(at('d.ts'), all)).toEqual(at('a.ts', 4));
  });

  it('keeps the last 50 places', () => {
    for (let i = 1; i <= 60; i++) navHistory.push(at('a.ts', i));
    const lines: number[] = [];
    let here = at('b.ts');
    for (let e = navHistory.back(here, all); e; e = navHistory.back(here, all)) {
      lines.push(e.line);
      here = e;
    }
    expect(lines).toHaveLength(50);
    expect(lines[0]).toBe(60);
    expect(lines.at(-1)).toBe(11);
  });

  it('skips a place whose file is gone', () => {
    navHistory.push(at('a.ts'));
    navHistory.push(at('gone.ts'));
    const exists = (e: NavEntry) => e.path !== 'gone.ts';
    expect(navHistory.back(at('c.ts'), exists)).toEqual(at('a.ts'));
    expect(navHistory.back(at('a.ts'), exists)).toBeNull();
  });

  it('keeps a history per project and source', () => {
    navHistory.push(at('a.ts'));
    navHistory.push(at('w.ts', 1, 1, 'a2'));
    expect(navHistory.back(at('b.ts'), all)).toEqual(at('a.ts'));
    expect(navHistory.back(at('x.ts', 1, 1, 'a2'), all)).toEqual(at('w.ts', 1, 1, 'a2'));
  });

  it('does not keep the same place twice in a row', () => {
    navHistory.push(at('a.ts', 2));
    navHistory.push(at('a.ts', 2));
    expect(navHistory.back(at('b.ts'), all)).toEqual(at('a.ts', 2));
    expect(navHistory.back(at('a.ts', 2), all)).toBeNull();
  });
});
