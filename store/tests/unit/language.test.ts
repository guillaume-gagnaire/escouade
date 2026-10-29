import { describe, expect, it } from 'vitest';
import { langOfPath, otherLang, pathIn } from '../../app/data/language';
import { LINKS, TEXTS } from '../../app/data/text';

describe('the two languages', () => {
  it('reads the language from the address', () => {
    expect(langOfPath('/')).toBe('fr');
    expect(langOfPath('/en')).toBe('en');
    expect(langOfPath('/en/plugins/x')).toBe('en');
    expect(langOfPath('/english')).toBe('fr');
  });

  it('gives the same page in the other language', () => {
    expect(pathIn('en', '/')).toBe('/en');
    expect(pathIn('fr', '/en')).toBe('/');
    expect(pathIn('en', '/plugins/x?tab=docs')).toBe('/en/plugins/x');
    expect(pathIn('fr', '/en/plugins/x')).toBe('/plugins/x');
    expect(otherLang('fr')).toBe('en');
  });

  it('has every text in both languages', () => {
    const keys = (o: object, at = ''): string[] =>
      Object.entries(o).flatMap(([k, v]) => (v && typeof v === 'object' && !Array.isArray(v) ? keys(v, `${at}${k}.`) : [`${at}${k}`]));
    expect(keys(TEXTS.en)).toEqual(keys(TEXTS.fr));
    for (const lang of ['fr', 'en'] as const) {
      expect(TEXTS[lang].home.steps).toHaveLength(3);
      for (const v of Object.values(TEXTS[lang].home).flat(2)) expect(String(v).trim()).not.toBe('');
    }
  });

  it("links to the site's legal pages in each language", () => {
    expect(LINKS.fr.privacy).toBe('https://escouade.dev/privacy/');
    expect(LINKS.en.privacy).toBe('https://escouade.dev/en/privacy/');
    expect(LINKS.fr.legal).toBe('https://escouade.dev/legal/');
    expect(LINKS.en.legal).toBe('https://escouade.dev/en/legal/');
  });
});
