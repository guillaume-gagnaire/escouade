import { describe, expect, it } from 'vitest';
import { newFileError, newFilePath, renameError } from './create';

const files = ['README.md', 'src/app.ts', 'src/lib/x.ts'];
const error = (name: string, dir = '', windows = true) => newFileError(name, dir, { files, windows });

describe('newFilePath', () => {
  it('puts the name typed in its folder, backslashes read as slashes', () => {
    expect(newFilePath('', 'a.ts')).toBe('a.ts');
    expect(newFilePath('src', 'lib\\b.ts')).toBe('src/lib/b.ts');
  });
});

describe('newFileError', () => {
  it('lets a new name through, folders to create included', () => {
    expect(error('b.ts')).toBeNull();
    expect(error('new/deep/b.ts', 'src')).toBeNull();
    expect(error('app.ts')).toBeNull();
  });

  it('has nothing to say while nothing is typed', () => {
    expect(error('')).toBeNull();
    expect(error('   ')).toBeNull();
  });

  it('refuses a file or folder already there, in any case', () => {
    expect(error('app.ts', 'src')).toBe('« app.ts » existe déjà à cet endroit.');
    expect(error('APP.TS', 'src')).toBe('« APP.TS » existe déjà à cet endroit.');
    expect(error('lib', 'src')).toBe('« lib » existe déjà à cet endroit.');
    expect(error('src/lib/x.ts')).toBe('« src/lib/x.ts » existe déjà à cet endroit.');
  });

  it('says that git ignores the file already there when the tree shows it as such', () => {
    const names = { files: [...files, '.env'], ignored: ['.env'], windows: true };
    expect(newFileError('.env', '', names)).toBe('« .env » existe déjà à cet endroit (ignoré par git).');
    expect(newFileError('.ENV', '', names)).toBe('« .ENV » existe déjà à cet endroit (ignoré par git).');
    expect(newFileError('app.ts', 'src', names)).toBe('« app.ts » existe déjà à cet endroit.');
  });

  it('refuses a folder where a file is', () => {
    expect(error('README.md/a.ts')).toBe('« README.md » est un fichier.');
    expect(error('app.ts/b/c.ts', 'src')).toBe('« app.ts » est un fichier.');
  });

  it('refuses a path that is not a file name below the folder', () => {
    expect(error('/a.ts')).toBe('Un nom ne peut pas commencer par une barre oblique.');
    expect(error('\\a.ts')).toBe('Un nom ne peut pas commencer par une barre oblique.');
    expect(error('a/')).toBe('Le nom doit finir par celui d’un fichier.');
    expect(error('a//b.ts')).toBe('« a//b.ts » n’est pas un nom de fichier valide.');
    expect(error('../a.ts')).toBe('« ../a.ts » n’est pas un nom de fichier valide.');
    expect(error('a/./b.ts')).toBe('« a/./b.ts » n’est pas un nom de fichier valide.');
  });

  it('refuses on Windows the names it would change or keeps for a device', () => {
    for (const name of ['a.', 'b ', 'x./c.ts', 'd?.ts', 'e:f', 'g*', 'h<', 'i|j', 'k"', 'CON', 'nul.txt', 'x/com1', 'LPT9.md']) {
      expect(error(name), name).toBe(`« ${name} » n’est pas un nom de fichier valide.`);
    }
    for (const name of ['com10', 'console.log', 'a.b', 'lpt0']) expect(error(name), name).toBeNull();
  });

  it('lets those names through elsewhere', () => {
    for (const name of ['a.', 'd?.ts', 'e:f', 'CON']) expect(error(name, '', false), name).toBeNull();
  });

  it('names a folder as a folder', () => {
    const folder = (name: string, dir = '') => newFileError(name, dir, { files, windows: true }, 'dir');
    expect(folder('lib', 'src')).toBe('« lib » existe déjà à cet endroit.');
    expect(folder('a/')).toBe('Le nom doit finir par celui d’un dossier.');
    expect(folder('CON')).toBe('« CON » n’est pas un nom de dossier valide.');
    expect(folder('new/deep', 'src')).toBeNull();
  });

  it('knows the empty folders the tree shows, made since it was read', () => {
    const names = { files, dirs: ['src/empty', 'docs/a/b'], windows: true };
    expect(newFileError('empty', 'src', names)).toBe('« empty » existe déjà à cet endroit.');
    expect(newFileError('DOCS', '', names, 'dir')).toBe('« DOCS » existe déjà à cet endroit.');
    expect(newFileError('docs/a', '', names, 'dir')).toBe('« docs/a » existe déjà à cet endroit.');
    expect(newFileError('a.ts', 'src/empty', names)).toBeNull();
  });
});

describe('renameError', () => {
  const rename = (name: string, from: string, kind: 'file' | 'dir' = 'file', dirs: string[] = []) =>
    renameError(name, from, kind, { files, dirs, windows: true });

  it('lets a free name through, in its folder or below it, and the same name (nothing to do)', () => {
    expect(rename('main.ts', 'src/app.ts')).toBeNull();
    expect(rename('core/app.ts', 'src/app.ts')).toBeNull();
    expect(rename('app.ts', 'src/app.ts')).toBeNull();
    expect(rename('source', 'src', 'dir')).toBeNull();
  });

  it('lets only the case change: the file there is the one renamed', () => {
    expect(rename('App.ts', 'src/app.ts')).toBeNull();
    expect(rename('SRC', 'src', 'dir')).toBeNull();
    expect(rename('Lib', 'src/lib', 'dir')).toBeNull();
  });

  it('refuses what is already there, in any case, and says why as the creation does', () => {
    expect(rename('lib', 'src/app.ts')).toBe('« lib » existe déjà à cet endroit.');
    expect(rename('README.MD', 'src', 'dir')).toBe('« README.MD » existe déjà à cet endroit.');
    expect(rename('x.ts', 'src/app.ts', 'file', ['src/x.ts'])).toBe('« x.ts » existe déjà à cet endroit.');
    expect(rename('a.', 'src/app.ts')).toBe('« a. » n’est pas un nom de fichier valide.');
    expect(rename('a/', 'src', 'dir')).toBe('Le nom doit finir par celui d’un dossier.');
    expect(rename('', 'src/app.ts')).toBeNull();
  });

  it('refuses a folder moved into itself, and a file taken for a folder', () => {
    expect(rename('lib/inner', 'src/lib', 'dir')).toBe('Un dossier ne peut pas aller dans lui-même.');
    expect(rename('LIB/inner', 'src/lib', 'dir')).toBe('Un dossier ne peut pas aller dans lui-même.');
    expect(rename('app.ts/b.ts', 'src/app.ts')).toBe('« app.ts » est un fichier.');
    expect(rename('README.md/b.ts', 'src', 'dir')).toBe('« README.md » est un fichier.');
  });
});
