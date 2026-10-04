import { describe, expect, it } from 'vitest';
import { fileIcon } from './icons';

describe('fileIcon', () => {
  it('marks a file by its extension, in any case', () => {
    expect(fileIcon('app.ts')).toEqual({ glyph: 'TS', color: 'var(--info)' });
    expect(fileIcon('src/Main.RS').glyph).toBe('RS');
    expect(fileIcon('data.json').glyph).toBe('{}');
    expect(fileIcon('App.svelte').glyph).toBe('S');
  });

  it('knows the files named for their role rather than their extension', () => {
    expect(fileIcon('Dockerfile').glyph).toBe('D');
    expect(fileIcon('web/.gitignore').glyph).toBe('git');
    expect(fileIcon('.env.local').glyph).toBe('env');
    expect(fileIcon('package-lock.json').glyph).toBe('lck');
    expect(fileIcon('Cargo.lock').glyph).toBe('lck');
    expect(fileIcon('LICENSE').glyph).toBe('©');
  });

  it('gives any other file, or a name being typed, the plain one', () => {
    const plain = fileIcon('notes');
    expect(plain).toEqual({ glyph: '≡', color: 'var(--dim)' });
    expect(fileIcon('archive.xyz')).toEqual(plain);
    expect(fileIcon('')).toEqual(plain);
    expect(fileIcon('src/')).toEqual(plain);
    expect(fileIcon('x.constructor')).toEqual(plain);
    expect(fileIcon('x.toString')).toEqual(plain);
  });
});
