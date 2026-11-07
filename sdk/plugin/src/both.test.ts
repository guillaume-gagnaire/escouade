// @vitest-environment node
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// The plugin the runtime's tests play on both transports (ESC-321): tests/fixtures/plugins/sdk-both, written for the
// SDK as before, its bundle committed (the Rust tests run without npm). Its bundle must be the build of its source
// with this SDK, or the tests would play an SDK that is not this one.

const ROOT = resolve(__dirname, '..', '..', '..');
const FIXTURE = join(ROOT, 'tests', 'fixtures', 'plugins', 'sdk-both');
const lf = (s: string) => s.replace(/\r\n/g, '\n');

describe('the plugin of the host network', () => {
  it('has its dist/plugin.mjs built from its src/ with this SDK', async () => {
    const esbuild = createRequire(import.meta.url)('esbuild') as typeof import('esbuild');
    const built = await esbuild.build({
      absWorkingDir: ROOT,
      entryPoints: ['tests/fixtures/plugins/sdk-net/src/main.ts'],
      bundle: true,
      platform: 'node',
      format: 'esm',
      target: 'node20',
      write: false,
      logLevel: 'silent',
    });
    const committed = readFileSync(join(ROOT, 'tests', 'fixtures', 'plugins', 'sdk-net', 'dist', 'plugin.mjs'), 'utf8');
    expect(
      lf(built.outputFiles[0].text) === lf(committed),
      'npx esbuild tests/fixtures/plugins/sdk-net/src/main.ts --bundle --platform=node --format=esm --target=node20 --outfile=tests/fixtures/plugins/sdk-net/dist/plugin.mjs',
    ).toBe(true);
  });
});

describe('the plugin of both transports', () => {
  it('has its dist/plugin.mjs built from its src/ with this SDK', async () => {
    // By require: esbuild's own build for Node.
    const esbuild = createRequire(import.meta.url)('esbuild') as typeof import('esbuild');
    const built = await esbuild.build({
      absWorkingDir: ROOT,
      entryPoints: ['tests/fixtures/plugins/sdk-both/src/main.ts'],
      bundle: true,
      platform: 'node',
      format: 'esm',
      target: 'node20',
      outfile: 'tests/fixtures/plugins/sdk-both/dist/plugin.mjs',
      write: false,
      logLevel: 'silent',
    });
    const committed = readFileSync(join(FIXTURE, 'dist', 'plugin.mjs'), 'utf8');
    expect(
      lf(built.outputFiles[0].text) === lf(committed),
      'npx esbuild tests/fixtures/plugins/sdk-both/src/main.ts --bundle --platform=node --format=esm --target=node20 --outfile=tests/fixtures/plugins/sdk-both/dist/plugin.mjs',
    ).toBe(true);
    // Only what a plugin in WASM may import.
    const imports = [...committed.matchAll(/^import .* from "([^"]+)";$/gm)].map((m) => m[1]);
    expect(imports.sort()).toEqual(['node:fs', 'node:readline']);
  });
});
