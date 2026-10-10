import { describe, expect, it } from 'vitest';
import { setLang } from '../i18n';
import { languageLabel, loadLanguage } from './languages';

describe('languages', () => {
  it('names the language of a file from its extension or its name', () => {
    expect(languageLabel('src/app.ts')).toBe('TypeScript');
    expect(languageLabel('a/B.TSX')).toBe('TypeScript JSX');
    expect(languageLabel('Cargo.toml')).toBe('TOML');
    expect(languageLabel('Dockerfile')).toBe('Dockerfile');
    expect(languageLabel('prisma/schema.prisma')).toBe('Prisma');
    expect(languageLabel('notes')).toBe('Texte');
  });

  it('calls a file of an unknown language plain text in English, and keeps the names of the others', () => {
    setLang('en');
    expect(languageLabel('notes')).toBe('Plain text');
    expect(languageLabel('src/app.ts')).toBe('TypeScript');
  });

  it('loads the syntax of a known language, none for plain text', async () => {
    expect(await loadLanguage('a.ts')).not.toBeNull();
    expect(await loadLanguage('a.toml')).not.toBeNull();
    expect(await loadLanguage('schema.prisma')).toBeNull();
    expect(await loadLanguage('notes')).toBeNull();
  });
});
