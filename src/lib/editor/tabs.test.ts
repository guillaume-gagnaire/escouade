import { describe, expect, it } from 'vitest';
import { tabHints } from './tabs';

describe('tabHints', () => {
  it('gives no hint to files whose name is alone', () => {
    expect(tabHints(['src/a.ts', 'src/b.ts', 'README.md'])).toEqual({ 'src/a.ts': '', 'src/b.ts': '', 'README.md': '' });
  });

  it('tells two files of the same name apart by the nearest folder', () => {
    const hints = tabHints(['src/components/editor/index.ts', 'src/components/board/index.ts', 'src/main.ts']);
    expect(hints['src/components/editor/index.ts']).toBe('editor');
    expect(hints['src/components/board/index.ts']).toBe('board');
    expect(hints['src/main.ts']).toBe('');
  });

  it('goes up a folder more for the files that the nearest folder does not tell apart', () => {
    const hints = tabHints(['web/src/index.ts', 'api/src/index.ts', 'api/lib/index.ts']);
    expect(hints).toEqual({ 'web/src/index.ts': 'web/src', 'api/src/index.ts': 'api/src', 'api/lib/index.ts': 'lib' });
  });

  it('gives each of three files of the same name its own shortest suffix', () => {
    const hints = tabHints(['a/x/index.ts', 'b/x/index.ts', 'c/y/index.ts']);
    expect(hints).toEqual({ 'a/x/index.ts': 'a/x', 'b/x/index.ts': 'b/x', 'c/y/index.ts': 'y' });
  });

  it('keeps every folder of a file whose folders are the end of the other’s', () => {
    const hints = tabHints(['x/index.ts', 'a/x/index.ts']);
    expect(hints).toEqual({ 'x/index.ts': 'x', 'a/x/index.ts': 'a/x' });
  });

  it('leaves the file at the root without a hint next to one in a folder', () => {
    expect(tabHints(['index.ts', 'src/index.ts'])).toEqual({ 'index.ts': '', 'src/index.ts': 'src' });
  });

  it('counts names as they are written, the case included', () => {
    expect(tabHints(['a/Index.ts', 'b/index.ts'])).toEqual({ 'a/Index.ts': '', 'b/index.ts': '' });
  });

  it('knows nothing to hint with no file', () => {
    expect(tabHints([])).toEqual({});
  });
});
