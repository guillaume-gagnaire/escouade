import { existsSync, lstatSync, mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  bwrapArgs,
  inSandbox,
  canUnshare,
  compileTwice,
  compilerOf,
  isolationOf,
  materialise,
  Refused,
  sandboxedCompiler,
  whyNoBubblewrap,
  type CompileJs,
  type CompileRust,
  type RustSandbox,
} from '../../server/utils/compile';
import { readArchive } from '../../server/utils/tar';
import { tarball } from '../fake/plugins.mjs';
import { sha256 } from '../../server/utils/versions';

// The store's compiler of the run.js bundles (ESC-323, docs/STORE.md §3.3): escouade-plugin build in a child
// process of its own, twice, each time in a new folder and with an environment of its own; two modules that differ
// refuse the version. Played on a fake escouade-plugin (tests/fake/fake-escouade-plugin.mjs) that can be made not
// deterministic, and on the real one when STORE_TEST_ESCOUADE_PLUGIN names it (with STORE_TEST_JAVY for its Javy).

const FAKE = join(import.meta.dirname, '../fake/fake-escouade-plugin.mjs');
const fake = (...before: string[]) => sandboxedCompiler({ command: [process.execPath, FAKE, ...before], isolation: 'none' });

const input = (bundle: string, over: Partial<CompileJs> = {}): CompileJs => ({
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

// A `run.wasm` plugin (ESC-336, docs/PLUGINS.md §8.5.3): its dependencies with a network, then the compilation without.
const RUST_MANIFEST = Buffer.from(JSON.stringify({ id: 'io.github.sroux.rust', version: '1.0.0', run: { wasm: 'Cargo.toml' } }));
const rust = (files: Record<string, string> = {}, over: Partial<CompileRust> = {}): CompileRust => ({
  manifest: RUST_MANIFEST,
  wasm: 'Cargo.toml',
  files: new Map(
    Object.entries({ 'Cargo.toml': '[package]\n', 'Cargo.lock': 'version = 4\n', 'src/lib.rs': '', ...files }).map(([k, v]) => [
      k,
      Buffer.from(v),
    ]),
  ),
  ...over,
});
const fakeRust = (...before: string[]) => sandboxedCompiler({ command: [process.execPath, FAKE, ...before], rust: 'none' });

describe('the compilation of a run.wasm plugin', () => {
  it('fetches, then compiles offline, twice, each time anew, and keeps the module both gave', async () => {
    const made = await compileTwice(fakeRust(), rust());
    expect(made.compilation).toMatchObject({
      state: 'built',
      compiler: { rust: '1.98.0', target: 'wasm32-wasip1', canon: 'c'.repeat(64), lock: sha256('version = 4\n') },
    });
    expect(sha256(made.bytes!)).toBe((made.compilation as { sha256: string }).sha256);
  });

  it('two modules that differ refuse the version', async () => {
    expect((await compileTwice(fakeRust('--random'), rust())).compilation.state).toBe('different');
  });

  it('a build.rs off the list and a lock that does not match are the plugin’s fault, said in the compiler’s words', async () => {
    const script = await compileTwice(fakeRust(), rust({ 'build.rs': 'fn main() {}' }));
    expect(script.compilation).toMatchObject({ state: 'refused', message: expect.stringContaining('has a build.rs') });
    const lock = await compileTwice(fakeRust(), rust({ 'Cargo.lock': 'STALE' }));
    expect(lock.compilation).toMatchObject({ state: 'refused', message: expect.stringContaining('Cargo.lock does not match') });
    const none = rust();
    (none.files as Map<string, Buffer>).delete('Cargo.lock');
    expect((await compileTwice(fakeRust(), none)).compilation).toMatchObject({
      state: 'refused',
      message: expect.stringContaining('no Cargo.lock'),
    });
  });

  it('a network that fails is the store’s fault: the version waits', async () => {
    await expect(compileTwice(fakeRust(), rust({ 'Cargo.toml': 'NET-DOWN' }))).rejects.not.toBeInstanceOf(Refused);
    await expect(compileTwice(fakeRust(), rust({ 'Cargo.toml': 'NET-DOWN' }))).rejects.toThrow(/fetch stopped/);
  });

  it('never compiles without a sandbox: none outside production alone, and a version waits in error', async () => {
    await expect(sandboxedCompiler({ command: [process.execPath, FAKE] })(rust())).rejects.toThrow(/no sandbox/);
    const said = sandboxedCompiler({ command: [process.execPath, FAKE], noSandbox: 'bwrap: No permissions' })(rust());
    await expect(said).rejects.toThrow('no sandbox for a run.wasm plugin here: bwrap: No permissions');
    expect(await compilerOf({ STORE_ESCOUADE_PLUGIN: FAKE, STORE_COMPILE_ISOLATION: 'none' })!(rust())).toMatchObject({
      compiler: { rust: '1.98.0' },
    });
    const made = compilerOf({ STORE_ESCOUADE_PLUGIN: FAKE, STORE_COMPILE_ISOLATION: 'none', NODE_ENV: 'production' })!;
    await expect(made(rust())).rejects.toThrow(/no sandbox/);
    expect(await made(input('console.log(1);\n'))).toMatchObject({ compiler: { javy: '9.1.0' } });
  });

  it('the sandbox: nothing shared, no network to compile, no variable it was not given, an empty /etc, no /proc', () => {
    const s: RustSandbox = { bwrap: 'bwrap', root: '/opt/sandbox/', program: '/usr/local/bin/escouade-plugin', toolchain: '/opt/rust' };
    const build = bwrapArgs(s, '/tmp/x', false, { PATH: '/usr/bin' });
    expect(build.slice(0, 4)).toEqual(['--unshare-all', '--clearenv', '--new-session', '--die-with-parent']);
    expect(build).not.toContain('--share-net');
    const after = (args: string[], word: string) => args.flatMap((w, i) => (w === word ? [`${args[i + 1]} ${args[i + 2]}`] : []));
    expect(after(build, '--ro-bind')).toEqual(['/opt/sandbox/usr /usr', '/opt/sandbox/opt /opt']);
    expect(after(build, '--bind')).toEqual(['/tmp/x /work']);
    expect(after(build, '--setenv')).toEqual(['PATH /usr/bin']);
    expect(build.join(' ')).not.toMatch(/\/app|\/data|--ro-bind-try/);
    expect(build.at(-1)).toBe('--');
    const fetch = bwrapArgs(s, '/tmp/x', true, {});
    expect(fetch).toContain('--share-net');
    expect(after(fetch, '--ro-bind')).toContain('/opt/sandbox/etc/ssl /etc/ssl');
    expect(build).not.toContain('--proc');
    expect(build.join(' ')).not.toContain('/proc');
    if (process.platform !== 'linux') expect(whyNoBubblewrap(s)).toBe('not Linux');
    // Its pid 1 keeps the environment it started with: nothing of the store's goes there.
    process.env.STORE_TEST_SECRET = 'seen';
    expect(Object.keys(inSandbox(s, '/tmp/x', false, {}, ['/bin/true'])[2])).toEqual(['PATH']);
    delete process.env.STORE_TEST_SECRET;
  });

  it('what is written holds no link: those of the archive stay out, and copy_plugin has none to follow', async () => {
    const read = readArchive(
      tarball({
        'Cargo.toml': '[package]\n',
        'src/lib.rs': '',
        'src/evil.rs': { symlink: '/etc/passwd' },
        'docs/hard': { hardlink: 'Cargo.toml' },
        'up/out': { symlink: '../../..' },
      } as Parameters<typeof tarball>[0]),
    );
    const to = mkdtempSync(join(tmpdir(), 'materialise-'));
    try {
      await materialise(read.files, to);
      const walk = (dir: string): string[] =>
        readdirSync(dir).flatMap((n) => {
          const at = join(dir, n);
          const stat = lstatSync(at);
          expect(stat.isSymbolicLink()).toBe(false);
          return stat.isDirectory() ? walk(at) : [relative(to, at).replace(/\\/g, '/')];
        });
      expect(walk(to).sort()).toEqual(['Cargo.toml', 'src/lib.rs']);
    } finally {
      rmSync(to, { recursive: true, force: true });
    }
    await expect(materialise(new Map([['../evil', Buffer.from('x')]]), to)).rejects.toBeInstanceOf(Refused);
    await expect(materialise(new Map([['/etc/evil', Buffer.from('x')]]), to)).rejects.toBeInstanceOf(Refused);
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

  it('compiles the Rust fixture twice to the hash of goldens.json, its dependencies fetched first', { timeout: 900_000 }, async () => {
    const repo = join(import.meta.dirname, '../../..');
    const dir = join(repo, 'tests/fixtures/plugins/rust-build');
    const walk = (at: string): string[] =>
      readdirSync(at, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(join(at, e.name)) : [join(at, e.name)]));
    const files = new Map(walk(dir).map((f) => [relative(dir, f).replace(/\\/g, '/'), readFileSync(f)]));
    const one = sandboxedCompiler({
      command: [REAL],
      rust: 'none',
      data: process.env.STORE_TEST_RUST_DATA || undefined,
    });
    const made = await compileTwice(one, { manifest: files.get('escouade-plugin.json')!, wasm: 'Cargo.toml', files });
    const golden = JSON.parse(readFileSync(join(repo, 'crates/escouade-plugin-cli/goldens.json'), 'utf8')).rust;
    expect(made.compilation).toMatchObject({ state: 'built', sha256: golden.module, compiler: { canon: golden.canon, lock: golden.lock } });
  });

  it('refuses a Cargo.lock that does not match and a build.rs', { timeout: 300_000 }, async () => {
    const dir = join(import.meta.dirname, '../../../tests/fixtures/plugins/rust-build');
    const walk = (at: string): string[] =>
      readdirSync(at, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(join(at, e.name)) : [join(at, e.name)]));
    const files = new Map(walk(dir).map((f) => [relative(dir, f).replace(/\\/g, '/'), readFileSync(f)]));
    const one = sandboxedCompiler({ command: [REAL], rust: 'none', data: process.env.STORE_TEST_RUST_DATA || undefined });
    const base = { manifest: files.get('escouade-plugin.json')!, wasm: 'Cargo.toml' };
    const lock = files
      .get('Cargo.lock')!
      .toString()
      .replace(/\[\[package\]\]\nname = "itoa"[\s\S]*?\n\n/, '');
    await expect(one({ ...base, files: new Map(files).set('Cargo.lock', Buffer.from(lock)) })).rejects.toThrow(
      /does not match|needs to be updated/,
    );
    await expect(one({ ...base, files: new Map(files).set('build.rs', Buffer.from('fn main() {}')) })).rejects.toThrow(/has a build\.rs/);
  });
});
