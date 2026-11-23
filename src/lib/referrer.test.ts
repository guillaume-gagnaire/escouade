import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

/**
 * ESC-654, point 6 of the audit of OshOEz's server (« Agent-authored link opened without noreferrer and without a
 * Referrer-Policy leaks the server origin »): our links open in the system's browser (`open_url`), which sends no
 * Referer, and the Markdown's carry `noreferrer`. The page itself says it too, for any request it would make
 * elsewhere (an image, a link of a view that slips past the opener): no Referer, whatever origin the window has.
 */
describe('the page of the window', () => {
  it('asks for no Referer', () => {
    const html = readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), '../../index.html'), 'utf8');
    expect(html).toMatch(/<meta\s+name="referrer"\s+content="no-referrer"\s*\/?>/);
  });
});
