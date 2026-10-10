import { describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { errorText, notFoundPath } from './errors';

describe('errors', () => {
  it('tells a missing file by the code the backend sends, whatever the path holds', () => {
    expect(notFoundPath('NOT_FOUND:src/a.ts')).toBe('src/a.ts');
    expect(notFoundPath('NOT_FOUND:dir/with: colon.ts')).toBe('dir/with: colon.ts');
    expect(notFoundPath(new Error('NOT_FOUND:x'))).toBeNull();
    // The text of an older backend, or one that only names a missing project, is no such code.
    expect(notFoundPath('src/a.ts introuvable')).toBeNull();
    expect(notFoundPath('projet introuvable')).toBeNull();
  });

  it('writes the codes of the backend in the language of the interface, and leaves its texts as they are', () => {
    expect(errorText('NOT_FOUND:src/a.ts')).toBe('src/a.ts introuvable');
    expect(errorText('git log a échoué')).toBe('git log a échoué');
    setLang('en');
    expect(errorText('NOT_FOUND:src/a.ts')).toBe('src/a.ts not found');
    expect(errorText('git log failed')).toBe('git log failed');
  });
});
