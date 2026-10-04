import { describe, expect, it } from 'vitest';
import { newFileError, newFilePath } from './create';

const files = ['README.md', 'src/app.ts', 'src/lib/x.ts'];
const error = (name: string, dir = '', windows = true) => newFileError(name, dir, files, windows);

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
});
