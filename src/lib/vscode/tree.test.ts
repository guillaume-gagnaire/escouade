import { describe, expect, it } from 'vitest';
import { DIR, dirsOf, entriesOf, FILE, fileUriOf, relativeTo, typeOf } from './tree';

describe('the paths of the VS Code view', () => {
  it('lists the folders of a source from its files', () => {
    const dirs = dirsOf(['README.md', 'src/app.ts', 'src/lib/a.ts', 'src/lib/b.ts']);
    expect([...dirs.keys()]).toEqual(['', 'src', 'src/lib']);
    expect([...dirs.get('')!]).toEqual([
      ['README.md', FILE],
      ['src', DIR],
    ]);
    expect(typeOf(dirs, 'src/lib')).toBe(DIR);
    expect(typeOf(dirs, 'src/lib/a.ts')).toBe(FILE);
    expect(typeOf(dirs, 'src/none.ts')).toBeNull();
    expect(typeOf(dirs, '')).toBe(DIR);
    expect([...entriesOf(dirs).keys()].sort()).toEqual(['README.md', 'src', 'src/app.ts', 'src/lib', 'src/lib/a.ts', 'src/lib/b.ts']);
  });

  it('reads a path from its root, never out of it', () => {
    expect(relativeTo('/c:/code/demo', '/c:/code/demo/src/a.ts', true)).toBe('src/a.ts');
    expect(relativeTo('/c:/code/demo', '/C:/Code/Demo/src/a.ts', true)).toBe('src/a.ts');
    expect(relativeTo('/c:/code/demo', '/C:/Code/Demo/src/a.ts', false)).toBeNull();
    expect(relativeTo('/c:/code/demo/', '/c:/code/demo', true)).toBe('');
    expect(relativeTo('/c:/code/demo', '/c:/code/demo-2/a.ts', true)).toBeNull();
    for (const climbing of ['/c:/code/demo/../x', '/c:/code/demo/src/../../x', '/c:/code/demo/./a', '/c:/code/demo//a'])
      expect(relativeTo('/c:/code/demo', climbing, true)).toBeNull();
  });

  it('writes a path of the system as VS Code’s file URI does', () => {
    expect(fileUriOf('C:\\code\\demo\\a.ts')).toEqual({ authority: '', path: '/c:/code/demo/a.ts' });
    expect(fileUriOf('C:\\')).toEqual({ authority: '', path: '/c:/' });
    expect(fileUriOf('/home/me/demo')).toEqual({ authority: '', path: '/home/me/demo' });
    expect(fileUriOf('\\\\wsl.localhost\\Ubuntu\\home\\me')).toEqual({ authority: 'wsl.localhost', path: '/Ubuntu/home/me' });
    expect(fileUriOf('relative/path')).toBeNull();
  });
});
