import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../i18n';
import { brings, chosenTheme, filesToLay, runs, status, statusLabel, themeKey, type ExtState } from './extensions';

function ext(over: Partial<ExtState> = {}): ExtState {
  return {
    id: 'acme.night',
    key: 'acme.night',
    namespace: 'acme',
    name: 'night',
    version: '1.0.0',
    displayName: 'Acme Night',
    description: 'A theme',
    publisher: 'Acme Inc.',
    kind: 'declarative',
    enabled: true,
    verified: true,
    preRelease: false,
    sha256: 'ab'.repeat(32),
    signed: true,
    installedAt: 0,
    files: 3,
    bytes: 1200,
    contributes: { themes: ['Acme Night'], iconThemes: [], languages: [], grammars: 0, snippets: 0 },
    dir: 'acme.night-1.0.0',
    refused: null,
    waits: null,
    ...over,
  };
}

describe('the extensions of the view', () => {
  beforeEach(() => setLang('fr'));

  it('runs only what is declarative, on and allowed', () => {
    expect(status(ext())).toBe('active');
    expect(status(ext({ enabled: false }))).toBe('off');
    expect(status(ext({ refused: 'non permise' }))).toBe('refused');
    // Refused wins over off: the list is said first.
    expect(status(ext({ refused: 'x', enabled: false }))).toBe('refused');
    expect(status(ext({ kind: 'web' }))).toBe('waitsHost');
    expect(status(ext({ kind: 'node' }))).toBe('needsNode');
    expect([ext(), ext({ kind: 'web' }), ext({ kind: 'node' }), ext({ enabled: false })].map(runs)).toEqual([true, false, false, false]);
  });

  it('says each state in a few words (the core says the whole sentence), never VS Code’s misleading words for Node', () => {
    expect(statusLabel('waitsHost')).toBe('En attente de l’hôte');
    expect(statusLabel('needsNode')).toBe('Demande Node');
    expect(statusLabel('refused')).toBe('Non permise');
    setLang('en');
    expect(statusLabel('needsNode')).toBe('Needs Node');
  });

  it('says what an extension brings', () => {
    expect(brings({ themes: ['a', 'b'], iconThemes: [], languages: ['toml'], grammars: 1, snippets: 0 })).toBe(
      '2 thèmes, 1 grammaire, 1 langage',
    );
    expect(brings({ themes: [], iconThemes: [], languages: [], grammars: 0, snippets: 0 })).toBe('');
  });

  it('keeps Escouade’s theme unless one of an extension that runs is chosen', () => {
    const list = [ext(), ext({ id: 'b.c', key: 'b.c', kind: 'web', contributes: { ...ext().contributes, themes: ['W'] } })];
    expect(chosenTheme(list, null)).toBeNull();
    expect(chosenTheme(list, themeKey(list[0], 'Acme Night'))).toBe('Acme Night');
    // A theme it does not have, an extension turned off or that does not run: Escouade's.
    expect(chosenTheme(list, 'acme.night/Other')).toBeNull();
    expect(chosenTheme([ext({ enabled: false })], 'acme.night/Acme Night')).toBeNull();
    expect(chosenTheme(list, 'b.c/W')).toBeNull();
    expect(chosenTheme(list, 'gone.x/Acme Night')).toBeNull();
  });

  it('lays the files its contributions read, within a bound', () => {
    const f = (path: string, size = 10) => ({ path, mime: 'x', size });
    const laid = filesToLay(
      [
        f('package.json'),
        f('themes/a.json'),
        f('syntaxes/toml.tmLanguage'),
        f('icons/a.svg'),
        f('fonts/x.woff2'),
        f('dist/web.js'),
        f('node_modules/x/package.json'),
        f('README.md'),
        f('big.json', 100),
      ],
      80,
    );
    expect(laid.map((x) => x.path)).toEqual(['package.json', 'themes/a.json', 'syntaxes/toml.tmLanguage', 'icons/a.svg', 'fonts/x.woff2']);
  });
});
