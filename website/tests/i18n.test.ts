import { existsSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { CATALOGS, featuresFor } from '../app/data/catalogs';
import { fmt, plural } from '../app/data/catalog';
import { en } from '../app/data/en';
import { fr } from '../app/data/fr';
import { FEATURE_SHOTS, IMAGE_DIR, VIDEO, imageOf } from '../app/data/site';
import { LANGS, langOfPath } from '../app/data/language';

const PUBLIC = new URL('../public/', import.meta.url);

/** Every text of a catalog by its path: `hero.line1`, `faq.items[2].q`… */
function leaves(value: unknown, path = ''): Map<string, string> {
  const out = new Map<string, string>();
  if (typeof value === 'string') out.set(path, value);
  else if (Array.isArray(value)) value.forEach((v, i) => leaves(v, `${path}[${i}]`).forEach((s, p) => out.set(p, s)));
  else if (value && typeof value === 'object') {
    for (const [k, v] of Object.entries(value)) leaves(v, path ? `${path}.${k}` : k).forEach((s, p) => out.set(p, s));
  } else throw new Error(`${path}: neither a text, a list nor an object`);
  return out;
}

const frLeaves = leaves(fr);
const enLeaves = leaves(en);
const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe('catalogs', () => {
  it('have the same keys and the same lists, in French and in English', () => {
    expect([...enLeaves.keys()].sort()).toEqual([...frLeaves.keys()].sort());
    expect(frLeaves.size).toBeGreaterThan(150);
  });

  it('say something in every place', () => {
    for (const [name, all] of [
      ['fr', frLeaves],
      ['en', enLeaves],
    ] as const) {
      for (const [path, s] of all) expect(s.trim(), `${name} ${path}`).not.toBe('');
    }
  });

  it('take the same {parameters} in both languages', () => {
    for (const [path, s] of frLeaves) expect(placeholders(enLeaves.get(path)!), path).toEqual(placeholders(s));
  });

  it('translate every text, but the names that are the same in both languages', () => {
    const same = [...frLeaves]
      .filter(([path, s]) => enLeaves.get(path) === s)
      .map(([path]) => path)
      .sort();
    expect(same).toEqual(
      [
        'header.sections',
        'header.faq',
        'install.steps[1].link.href',
        'header.github',
        'faq.eyebrow',
        'footer.github',
        'demo.status.question',
        'demo.agents',
        'demo.tokens',
        'squad.names.docs',
        'squad.names.login',
      ].sort(),
    );
  });

  it('write apostrophes and quotation marks the typographic way', () => {
    for (const [name, all] of [
      ['fr', frLeaves],
      ['en', enLeaves],
    ] as const) {
      for (const [path, s] of all) {
        expect(s, `${name} ${path}`).not.toMatch(/['"]/);
        expect(s, `${name} ${path}`).not.toContain('...');
      }
    }
    for (const [path, s] of frLeaves) expect(s, `fr ${path}`).not.toMatch(/[“”]/);
    for (const [path, s] of enLeaves) expect(s, `en ${path}`).not.toMatch(/[«»]/);
  });

  it('are found by language', () => {
    expect(CATALOGS.fr).toBe(fr);
    expect(CATALOGS.en).toBe(en);
    expect(LANGS.map((l) => l.code)).toEqual(['fr', 'en']);
  });

  it('keep the name of the product', () => {
    for (const all of [frLeaves, enLeaves]) {
      const said = [...all.values()].join(' ');
      expect(said).toContain('Escouade');
      expect(said).not.toContain('Claude Code Manager');
    }
  });
});

describe('texts with parameters', () => {
  it('fills the {parameters} in', () => {
    expect(fmt('Version {version} · {os}', { version: '1.7.0', os: 'Windows' })).toBe('Version 1.7.0 · Windows');
    expect(fmt('{n} et {n}', { n: 2 })).toBe('2 et 2');
    expect(fmt('Sans paramètre', {})).toBe('Sans paramètre');
  });

  it('refuses a parameter that was not given, rather than showing {it}', () => {
    expect(() => fmt('Bonjour {name}', {})).toThrow(/name/);
  });

  it('picks the singular or the plural form of the language, and fills in the count', () => {
    const files = { one: '{count} file', other: '{count} files' };
    expect(plural('en', files, 1)).toBe('1 file');
    expect(plural('en', files, 0)).toBe('0 files');
    expect(plural('en', files, 2)).toBe('2 files');
    const fichiers = { one: '{count} fichier', other: '{count} fichiers' };
    // In French, zero is singular.
    expect(plural('fr', fichiers, 0)).toBe('0 fichier');
    expect(plural('fr', fichiers, 1)).toBe('1 fichier');
    expect(plural('fr', fichiers, 2)).toBe('2 fichiers');
    expect(plural('fr', fichiers, 1_000_000)).toBe('1000000 fichiers');
    expect(plural('en', { one: '{met} of {total}', other: '{met} of {total}' }, 2, { met: 2, total: 3 })).toBe('2 of 3');
  });
});

describe('what the two pages share', () => {
  it('knows which language a path of the site is in', () => {
    expect(langOfPath('/')).toBe('fr');
    expect(langOfPath('')).toBe('fr');
    expect(langOfPath('/en')).toBe('en');
    expect(langOfPath('/en/')).toBe('en');
    expect(langOfPath('/en/#faq')).toBe('en');
    expect(langOfPath('/english')).toBe('fr');
    expect(langOfPath('/fr/')).toBe('fr');
  });

  it('shows the same features, in the same order, with the same screenshots', () => {
    const french = featuresFor('fr');
    const english = featuresFor('en');
    expect(english.map((f) => f.id)).toEqual(french.map((f) => f.id));
    expect(english.map((f) => f.id)).toEqual(FEATURE_SHOTS.map((f) => f.id));
    expect(french).toHaveLength(11);
    for (const [name, list] of [
      ['fr', french],
      ['en', english],
    ] as const) {
      for (const f of list) {
        expect(f.image, `${name} ${f.id}`).toBe(imageOf(name, FEATURE_SHOTS.find((s) => s.id === f.id)!.file));
        expect(existsSync(new URL(f.image, PUBLIC)), `${name} ${f.id}: ${f.image}`).toBe(true);
        expect(f.title, f.id).not.toBe('');
        expect(f.points.length, f.id).toBe(3);
      }
    }
  });

  it('keeps the screenshots of each language in a folder of its own, ready for the English ones', () => {
    expect(Object.keys(IMAGE_DIR)).toEqual(['fr', 'en']);
    expect(imageOf('fr', 'poster.jpg')).toBe(`${IMAGE_DIR.fr}poster.jpg`);
    // The French folder is the one the site has always used: its links stay good.
    expect(IMAGE_DIR.fr).toBe('images/');
    for (const lang of ['fr', 'en'] as const) expect(existsSync(new URL(imageOf(lang, 'poster.jpg'), PUBLIC)), lang).toBe(true);
  });

  it('plays the one video in both languages, with the subtitles of its voice', () => {
    for (const lang of ['fr', 'en'] as const) {
      expect(VIDEO[lang].src).toBe('escouade.mp4');
      expect(VIDEO[lang].tracks.length).toBeGreaterThan(0);
      for (const t of VIDEO[lang].tracks) expect(existsSync(new URL(t.src, PUBLIC)), t.src).toBe(true);
    }
    expect(VIDEO.fr.tracks[0]).toMatchObject({ srclang: 'fr', label: 'Français', src: 'escouade.vtt' });
  });
});
