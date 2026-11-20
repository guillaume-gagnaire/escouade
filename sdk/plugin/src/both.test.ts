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

describe('the plugin of the contract of 2.2.5 (ESC-531)', () => {
  const FOLDER = join(ROOT, 'tests', 'fixtures', 'plugins', 'sdk-contract');

  it('has its dist/plugin.mjs built from its src/ with this SDK', async () => {
    const esbuild = createRequire(import.meta.url)('esbuild') as typeof import('esbuild');
    const built = await esbuild.build({
      absWorkingDir: ROOT,
      entryPoints: ['tests/fixtures/plugins/sdk-contract/src/main.ts'],
      bundle: true,
      platform: 'node',
      format: 'esm',
      target: 'node20',
      write: false,
      logLevel: 'silent',
    });
    const committed = readFileSync(join(FOLDER, 'dist', 'plugin.mjs'), 'utf8');
    expect(
      lf(built.outputFiles[0].text) === lf(committed),
      'npx esbuild tests/fixtures/plugins/sdk-contract/src/main.ts --bundle --platform=node --format=esm --target=node20 --outfile=tests/fixtures/plugins/sdk-contract/dist/plugin.mjs',
    ).toBe(true);
  });

  it('runs its tunnel in the fake Escouade as in the app: the profile chosen, five programs at once, every word admitted', async () => {
    const { plugin } = await import('../../../tests/fixtures/plugins/sdk-contract/src/plugin');
    const { connect } = await import('./testing');
    const manifest = JSON.parse(readFileSync(join(FOLDER, 'escouade-plugin.json'), 'utf8'));
    const profile = '0123456789abcdef';
    const fake = connect(plugin, {
      manifest,
      version: '2.2.5',
      profiles: {
        vpn: [
          {
            id: profile,
            fields: { name: 'Acme', server: 'vpn.acme.example', proto: 'tcp' },
            secrets: { ovpn: 'client\nremote a 1194\n', password: 'p' },
          },
        ],
      },
      projectValues: { 'bench-project': { profile } },
      secrets: { notes: 'one\ntwo' },
      files: { 'rules.txt': readFileSync(join(FOLDER, 'rules.txt'), 'utf8') },
    });
    await fake.initialize();
    const done = fake.fact('ticket.column');
    // Its programs end as they are closed, or at once for the checks.
    for (let turn = 0; turn < 50 && fake.spawned.length < 6; turn++) {
      await new Promise((r) => setTimeout(r, 5));
      for (const program of [...fake.programs.values()])
        if (program.closed || program.asked[1] === 'hash-object' || program.asked[2] === 'version') await fake.exit(program.proc, 0);
    }
    for (let turn = 0; turn < 50 && fake.programs.size; turn++) {
      await new Promise((r) => setTimeout(r, 5));
      for (const program of [...fake.programs.values()])
        if (program.closed || program.asked[2] === 'version') await fake.exit(program.proc, 0);
    }
    await done;
    expect(fake.refusals).toEqual([]);
    expect(fake.spawned.map((s) => s.words.slice(1).join(' '))).toEqual([
      'cat-file --batch',
      `column --mode=plain --width ${fake.spawned[0].pid}`,
      'column --mode=plain --padding 2',
      `hash-object /escouade/plugins/temp/dev.test.plugin/tunnel.ovpn`,
      `hash-object /escouade/plugins/dev.test.plugin/rules.txt`,
      '--no-pager version',
    ]);
    expect(fake.logs.map((l) => l.message)).toEqual([
      'profile Acme: vpn.acme.example in tcp, 3 lines of configuration, a password, notes, routes 10.0.0.0/8',
      'tunnel checked: 0 0 0 0 0 0',
    ]);
  });
});
