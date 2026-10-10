import { describe, expect, it } from 'vitest';
import { notFoundPath } from './errors';

describe('errors', () => {
  it('tells a missing file by the code the backend sends, whatever the path holds', () => {
    expect(notFoundPath('NOT_FOUND:src/a.ts')).toBe('src/a.ts');
    expect(notFoundPath('NOT_FOUND:dir/with: colon.ts')).toBe('dir/with: colon.ts');
    expect(notFoundPath(new Error('NOT_FOUND:x'))).toBeNull();
    // The text of an older backend, or one that only names a missing project, is no such code.
    expect(notFoundPath('src/a.ts introuvable')).toBeNull();
    expect(notFoundPath('projet introuvable')).toBeNull();
  });
});
