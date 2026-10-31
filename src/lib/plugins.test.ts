import { describe, expect, it } from 'vitest';
import { sourceUrl } from './plugins';

// Where a store plugin comes from (ESC-313): its repository, or its folder at its tag when it is in one.
describe('the source of a store plugin', () => {
  it('links the folder at the tag, and the repository for a plugin at its root or an old version.json', () => {
    expect(sourceUrl({ repository: 'https://github.com/o/r', tag: 'slack@1.0.0', path: 'plugins/slack' })).toBe(
      'https://github.com/o/r/tree/slack@1.0.0/plugins/slack',
    );
    expect(sourceUrl({ repository: 'https://github.com/o/r/', tag: 'plugins/x/v1.0.0', path: 'plugins/x' })).toBe(
      'https://github.com/o/r/tree/plugins/x/v1.0.0/plugins/x',
    );
    expect(sourceUrl({ repository: 'https://github.com/o/r', tag: 'v1.0.0', path: '' })).toBe('https://github.com/o/r');
    expect(sourceUrl({ repository: 'https://github.com/o/r', tag: 'v1.0.0' })).toBe('https://github.com/o/r');
  });
});
