import { existsSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { CATALOGS, featuresFor } from '../app/data/catalogs';
import { fmt, plural } from '../app/data/catalog';
import { en } from '../app/data/en';
import { fr } from '../app/data/fr';
import { FEATURE_SHOTS, IMAGE_DIR, VIDEO, imageOf } from '../app/data/site';
import { LANGS, LANG_KEY, langOfPath, pickLanguage, rememberLanguage } from '../app/data/language';
import { leaves } from './helpers';

const PUBLIC = new URL('../public/', import.meta.url);

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

describe('the language the visitor chose', () => {
  it('is kept in the browser’s storage under the name the redirection reads', () => {
    const kept = new Map<string, string>();
    rememberLanguage('en', () => ({ setItem: (k, v) => void kept.set(k, v) }));
    expect(kept).toEqual(new Map([[LANG_KEY, 'en']]));
    rememberLanguage('fr', () => ({ setItem: (k, v) => void kept.set(k, v) }));
    expect(kept.get(LANG_KEY)).toBe('fr');
    expect(LANG_KEY).toBe('escouade-lang');
  });

  it('lets the visit go on when the browser refuses to store it', () => {
    const refusing = () => ({
      setItem: () => {
        throw new DOMException('The quota has been exceeded.', 'QuotaExceededError');
      },
    });
    expect(() => rememberLanguage('en', refusing)).not.toThrow();
    // Some browsers refuse to even hand over the storage (cookies blocked).
    expect(() =>
      rememberLanguage('en', () => {
        throw new DOMException('The operation is insecure.', 'SecurityError');
      }),
    ).not.toThrow();
  });
});

describe('which language a visit to the French root should end up in', () => {
  it('stays in French for a browser that prefers French, wherever it sits in its list', () => {
    expect(pickLanguage(['fr-FR', 'en-US', 'en'], null, null)).toBe('fr');
    expect(pickLanguage(['fr'], null, null)).toBe('fr');
    expect(pickLanguage(['FR-ca'], null, null)).toBe('fr');
    expect(pickLanguage(['de-DE', 'fr-BE'], null, null)).toBe('fr');
  });

  it('goes to English for a browser that has no French among its languages', () => {
    expect(pickLanguage(['en-US', 'en'], null, null)).toBe('en');
    expect(pickLanguage(['de-DE', 'es'], null, null)).toBe('en');
    expect(pickLanguage(['ja'], null, null)).toBe('en');
  });

  it('does not take a language that merely starts like French for French', () => {
    expect(pickLanguage(['fro', 'frr-DE'], null, null)).toBe('en');
  });

  it('stays where it is when it does not know the browser’s languages', () => {
    expect(pickLanguage([], null, null)).toBe('fr');
  });

  it('goes where the visitor said they wanted to be, whatever the browser says', () => {
    expect(pickLanguage(['en-US'], 'fr', null)).toBe('fr');
    expect(pickLanguage(['fr-FR'], 'en', null)).toBe('en');
    expect(pickLanguage([], 'en', null)).toBe('en');
  });

  it('ignores a kept choice that is not one of the site’s languages', () => {
    expect(pickLanguage(['en-US'], 'de', null)).toBe('en');
    expect(pickLanguage(['fr-FR'], '', null)).toBe('fr');
    expect(pickLanguage(['en-US'], '__proto__', null)).toBe('en');
  });

  it('obeys the address first: ?lang=fr asks for French, over a kept choice and over the browser', () => {
    expect(pickLanguage(['en-US'], null, 'fr')).toBe('fr');
    expect(pickLanguage(['en-US'], 'en', 'fr')).toBe('fr');
    expect(pickLanguage(['fr-FR'], 'fr', 'en')).toBe('en');
    // An address that asks for something else is not an answer.
    expect(pickLanguage(['en-US'], null, 'de')).toBe('en');
    expect(pickLanguage(['en-US'], null, '')).toBe('en');
  });
});
