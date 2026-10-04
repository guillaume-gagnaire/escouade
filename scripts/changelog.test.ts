// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { FALLBACK, releaseNotes } from './changelog.mjs';

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
