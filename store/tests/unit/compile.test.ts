import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  canUnshare,
  compileTwice,
  compilerOf,
  isolationOf,
  Refused,
  sandboxedCompiler,
  type CompileInput,
} from '../../server/utils/compile';
import { sha256 } from '../../server/utils/versions';

// The store's compiler of the run.js bundles (ESC-323, docs/STORE.md §3.3): escouade-plugin build in a child
// process of its own, twice, each time in a new folder and with an environment of its own; two modules that differ
// refuse the version. Played on a fake escouade-plugin (tests/fake/fake-escouade-plugin.mjs) that can be made not
// deterministic, and on the real one when STORE_TEST_ESCOUADE_PLUGIN names it (with STORE_TEST_JAVY for its Javy).

const FAKE = join(import.meta.dirname, '../fake/fake-escouade-plugin.mjs');
const fake = (...before: string[]) => sandboxedCompiler({ command: [process.execPath, FAKE, ...before], isolation: 'none' });

const input = (bundle: string, over: Partial<CompileInput> = {}): CompileInput => ({
  manifest: Buffer.from(JSON.stringify({ id: 'io.github.sroux.rules', version: '1.0.0', run: { js: 'dist/plugin.mjs' } })),
  js: 'dist/plugin.mjs',
  bundle: Buffer.from(bundle),
  ...over,
});

describe('the compilation of a run.js bundle', () => {
  it('compiles twice, each time anew, and keeps the module both gave', async () => {
    let calls = 0;
    const one = fake();
    const made = await compileTwice(async (i) => {
      calls++;
      return one(i);
    }, input('console.log("rules")'));
    expect(calls).toBe(2);
    expect(made.compilation).toMatchObject({ state: 'built', compiler: { javy: '9.1.0', prelude: '2.2.0' } });
    const c = made.compilation as { sha256: string; size: number };
    expect(sha256(made.bytes!)).toBe(c.sha256);
    expect(made.bytes!.length).toBe(c.size);
    expect(made.bytes!.subarray(0, 4)).toEqual(Buffer.from([0, 0x61, 0x73, 0x6d]));
  });

  it('a compiler made not deterministic refuses the version: two modules, two fingerprints', async () => {
    const made = await compileTwice(fake('--random'), input('console.log("rules")'));
    expect(made.compilation.state).toBe('different');
    expect(made.bytes).toBeUndefined();
    const d = made.compilation as { first: string; second: string };
    expect(d.first).not.toBe(d.second);
  });

  it('a bundle that does not compile is the plugin’s fault, said in the compiler’s words', async () => {
    const made = await compileTwice(fake(), input('import net from "node:net";'));
    expect(made.compilation).toEqual({
      state: 'refused',
      message: '✗ dist/plugin.mjs does not compile to WASM: import of node:net refused (only node:fs and node:readline)',
    });
  });

  it('a compilation past its time is stopped, and refused', async () => {
    const one = sandboxedCompiler({ command: [process.execPath, FAKE], isolation: 'none', timeoutMs: 1500 });
    const started = Date.now();
    await expect(one(input('LOOP'))).rejects.toThrow(new Refused('the compilation took more than 2 s'));
    expect(Date.now() - started).toBeLessThan(10_000);
  });

  it.runIf(process.platform === 'linux')('a compilation past its memory is stopped, and refused', async () => {
    const one = sandboxedCompiler({ command: [process.execPath, FAKE], isolation: 'none', memoryBytes: 256 * 1024 * 1024 });
    await expect(one(input('GREEDY'))).rejects.toThrow(/more than 256 MiB/);
  });

  it('gets none of the store’s variables, and reads a first party plugin as such', async () => {
    process.env.STORE_TEST_SECRET = 'hunter2';
    try {
      const made = await fake()(input('console.log(1)', { firstParty: true }));
      expect(made.compiler.prelude).toBe('2.2.0-first');
    } finally {
      delete process.env.STORE_TEST_SECRET;
    }
  });

  it('refuses a bundle path out of the plugin, and a compiler that is not there is the store’s fault', async () => {
    await expect(fake()(input('x', { js: '../x.mjs' }))).rejects.toBeInstanceOf(Refused);
    const missing = sandboxedCompiler({ command: [join(import.meta.dirname, 'no-such-escouade-plugin')], isolation: 'none' });
    const e = await missing(input('x')).catch((x: unknown) => x);
    expect(e).toBeInstanceOf(Error);
    expect(e).not.toBeInstanceOf(Refused);
    // A crash that is no refusal of the bundle (no ✗ line) is the store's too: the version is checked again.
    const crash = await fake()(input('CRASH')).catch((x: unknown) => x);
    expect(crash).not.toBeInstanceOf(Refused);
    expect(String(crash)).toContain('escouade-plugin build stopped (101): thread main panicked: out of disk');
  });

  it('is off without STORE_ESCOUADE_PLUGIN, and never isolated by a namespace off Linux', () => {
    expect(compilerOf({})).toBeNull();
    expect(compilerOf({ STORE_ESCOUADE_PLUGIN: '/opt/escouade/escouade-plugin' })).toBeTypeOf('function');
    expect(isolationOf('none')).toBe('none');
    if (process.platform !== 'linux') {
      expect(canUnshare()).toBe(false);
      expect(() => isolationOf('require')).toThrow(/unshare -rn/);
    }
  });
});

// The real escouade-plugin and its pinned Javy: the same module twice, the fingerprint escouade-plugin build gives.
const REAL = process.env.STORE_TEST_ESCOUADE_PLUGIN ?? '';
describe.runIf(REAL && existsSync(REAL))('the real escouade-plugin', () => {
  it('compiles the SDK’s plugin twice to the same module, without the store’s cache', { timeout: 300_000 }, async () => {
    const dir = join(import.meta.dirname, '../../../tests/fixtures/plugins/sdk-both');
    const one = sandboxedCompiler({ command: [REAL], javy: process.env.STORE_TEST_JAVY || undefined });
    const made = await compileTwice(one, {
      manifest: readFileSync(join(dir, 'escouade-plugin.json')),
      js: 'dist/plugin.mjs',
      bundle: readFileSync(join(dir, 'dist/plugin.mjs')),
    });
    expect(made.compilation).toMatchObject({ state: 'built', compiler: { javy: '9.1.0' } });
    console.info('[compile] real escouade-plugin:', JSON.stringify(made.compilation), 'isolation', isolationOf());
  });
});
