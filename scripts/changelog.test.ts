// @vitest-environment node
import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { CHANGELOGS, FALLBACK, FALLBACKS, changelogText, langOf, releaseNotes } from './changelog.mjs';

const ROOT = path.resolve(__dirname, '..');

const CHANGELOG = `# Journal des versions

Intro.

## [1.3.0] — 2026-10-04

### Ajouts

- Le tableau.

### Corrections

- Les liens.

## [1.1.0] — 2026-10-03

- L'éditeur.
`;

describe('release notes', () => {
  it('takes the section of the tag’s version, without its heading, up to the next version', () => {
    expect(releaseNotes(CHANGELOG, 'v1.3.0')).toBe('### Ajouts\n\n- Le tableau.\n\n### Corrections\n\n- Les liens.');
    expect(releaseNotes(CHANGELOG, '1.1.0')).toBe("- L'éditeur.");
  });

  it('falls back to the commits sentence when the version has no section', () => {
    expect(releaseNotes(CHANGELOG, 'v9.9.9')).toBe(FALLBACK);
    expect(releaseNotes('', 'v1.3.0')).toBe(FALLBACK);
  });

  it('does not take a version for another that starts the same way', () => {
    const text = '## [1.3.10]\n\n- Dix.\n\n## [1.3.1]\n\n- Un.\n';
    expect(releaseNotes(text, 'v1.3.1')).toBe('- Un.');
  });

  it('reads a file written with Windows line endings', () => {
    expect(releaseNotes(CHANGELOG.replace(/\n/g, '\r\n'), 'v1.1.0')).toBe("- L'éditeur.");
  });
});

describe('a pre-release', () => {
  const WITH_UNRELEASED = `# Journal

## [Non publié]

### Nouveautés

- Les bêtas.

## [2.1.3] — 2026-11-06

- Les quotas.
`;

  it('takes the section of what is not out yet when it has none of its own, in French and in English', () => {
    expect(releaseNotes(WITH_UNRELEASED, 'v2.2.0-beta.1')).toBe('### Nouveautés\n\n- Les bêtas.');
    const en = WITH_UNRELEASED.replace('[Non publié]', '[Unreleased]').replace('Nouveautés', 'New');
    expect(releaseNotes(en, 'v2.2.0-beta.1', FALLBACKS.en)).toBe('### New\n\n- Les bêtas.');
  });

  it('takes its own section when it has one', () => {
    const own = WITH_UNRELEASED.replace('## [Non publié]', '## [2.2.0-beta.1] — 2026-11-08\n\n- La bêta.\n\n## [Non publié]');
    expect(releaseNotes(own, 'v2.2.0-beta.1')).toBe('- La bêta.');
  });

  it('falls back to the commits sentence without either, and a stable version never takes what is not out', () => {
    expect(releaseNotes(CHANGELOG, 'v2.2.0-beta.1')).toBe(FALLBACK);
    expect(releaseNotes(WITH_UNRELEASED, 'v2.2.0')).toBe(FALLBACK);
  });
});

describe('the two changelogs', () => {
  it('reads the language of the command line, French by default, and refuses another', () => {
    expect(langOf(['v1.8.7'])).toBe('fr');
    expect(langOf(['v1.8.7', '--lang', 'en'])).toBe('en');
    expect(() => langOf(['v1.8.7', '--lang', 'de'])).toThrow('de');
  });

  it('falls back to a sentence in the changelog’s own language', () => {
    expect(releaseNotes('', 'v9.9.9', FALLBACKS.en)).toBe('See the commits since the previous version.');
    expect(FALLBACKS.fr).toBe(FALLBACK);
  });

  // The GitHub release and the English Discord channel take the English section: a version without one would
  // go out with the fallback sentence. Checked here, by the CI that comes before the tag.
  it('has, in English, every version of CHANGELOG.md from 1.8.2 on, and what is not out yet', () => {
    const versions = (text: string) => [...text.matchAll(/^## \[([^\]]+)\]/gm)].map((m) => m[1]);
    // What is not out yet: « [Non publié] » in French, « [Unreleased] » in English.
    const fr = versions(changelogText(ROOT, 'fr')).map((v) => (v === 'Non publié' ? 'Unreleased' : v));
    const en = versions(changelogText(ROOT, 'en'));
    const since = fr.slice(0, fr.indexOf('1.8.2') + 1);
    expect(since.length).toBeGreaterThan(0);
    expect(since.filter((v) => !en.includes(v))).toEqual([]);
    for (const v of since) expect(releaseNotes(changelogText(ROOT, 'en'), v, '')).not.toBe('');
  });

  it('names the two files the scripts read', () => {
    for (const file of Object.values(CHANGELOGS)) expect(fs.existsSync(path.join(ROOT, file))).toBe(true);
  });
});
