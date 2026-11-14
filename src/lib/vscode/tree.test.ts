import { describe, expect, it } from 'vitest';
import { DIR, FILE, dirsOf, relativeTo, typeOf } from './tree';

describe('the folders of the VS Code view', () => {
  it('makes the folders from the files, each with its entries', () => {
    const dirs = dirsOf(['README.md', 'src/app.ts', 'src/lib/a.ts']);
    expect([...dirs.get('')!]).toEqual([
      ['README.md', FILE],
      ['src', DIR],
    ]);
    expect([...dirs.get('src')!]).toEqual([
      ['app.ts', FILE],
      ['lib', DIR],
    ]);
    expect(typeOf(dirs, 'src/lib')).toBe(DIR);
    expect(typeOf(dirs, 'src/lib/a.ts')).toBe(FILE);
    expect(typeOf(dirs, 'src/none.ts')).toBeNull();
    expect(typeOf(dirs, '')).toBe(DIR);
  });

  it('tells a path below the root, by case on Linux and without it on Windows', () => {
    expect(relativeTo('/c:/code/demo', '/c:/code/demo/src/a.ts', true)).toBe('src/a.ts');
    expect(relativeTo('/c:/code/demo', '/C:/Code/Demo/src/a.ts', true)).toBe('src/a.ts');
    expect(relativeTo('/home/me/demo', '/home/me/Demo/a.ts', false)).toBeNull();
    expect(relativeTo('/home/me/demo', '/home/me/demo', false)).toBe('');
    expect(relativeTo('/home/me/demo', '/home/me/demo-api/a.ts', false)).toBeNull();
  });
});
