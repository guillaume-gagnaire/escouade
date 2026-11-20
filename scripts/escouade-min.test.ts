// @vitest-environment node
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { current, FILES, rewrite, ROOT } from './escouade-min.mjs';

// The `escouade.min` of the contract of ESC-524 to ESC-531, said in one place (schemas/plugin-protocol-1.json) and
// written everywhere else by `node scripts/escouade-min.mjs <version>` (ESC-531).

const read = (f: string) => readFileSync(join(ROOT, f), 'utf8');

describe('the escouade.min of the contract', () => {
  const v = current();

  it('is the same in the code of the protocol, of the two readers, of the SDK and of the test plugin', () => {
    expect(v).toMatch(/^\d+\.\d+\.\d+/);
    expect(read('crates/escouade-plugin-protocol/src/lib.rs')).toContain(`pub const ESCOUADE_MIN: &str = "${v}";`);
    expect(read('crates/escouade-plugin-cli/src/manifest.rs')).toContain(`pub const CONTRACT_MIN: &str = "${v}";`);
    for (const f of ['schemas/plugin-manifest.mjs', 'sdk/plugin/vendor/plugin-manifest.mjs'])
      expect(read(f), f).toContain(`export const CONTRACT_MIN = '${v}';`);
    expect(read('sdk/plugin/src/protocol.ts')).toContain(`export const ESCOUADE_MIN = '${v}';`);
    expect(JSON.parse(read('tests/fixtures/plugins/sdk-contract/escouade-plugin.json')).escouade.min).toBe(v);
    expect(read('website/public/schemas/plugin-protocol-1.json')).toBe(read('schemas/plugin-protocol-1.json'));
  });

  it('is what every file of the list says, and the list names files that exist', () => {
    for (const f of FILES) expect(read(f), f).toContain(v);
  });

  it('is written by the script without touching a longer version or another number', () => {
    expect(rewrite('Escouade 2.2.5 (2.2.50, 12.2.5, 2.2.5.1) #le-contrat-de-la-2-2-5', '2.2.5', '2.2.6')).toBe(
      'Escouade 2.2.6 (2.2.50, 12.2.5, 2.2.5.1) #le-contrat-de-la-2-2-6',
    );
  });
});
