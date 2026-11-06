// @vitest-environment node
import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { AWAITING_PUBLICATION, listPlugins, parseArgs, problemsOf, readCargo, readPlugin, touchedBy } from '../plugins/ci-lib.mjs';

const made: string[] = [];
const folder = (files: Record<string, string>) => {
  const dir = mkdtempSync(join(tmpdir(), 'plugins-ci-'));
  made.push(dir);
  for (const [file, text] of Object.entries(files)) writeFileSync(join(dir, file), text);
  return dir;
};
afterEach(() => {
  for (const dir of made.splice(0)) rmSync(dir, { recursive: true, force: true });
});

const manifest = (fields: object) => JSON.stringify({ id: 'dev.escouade.x', version: '2.2.0', ...fields });
const APP = { run: { command: '{escouade}/escouade-plugins', args: ['x'] } };
const WASM = { run: { wasm: 'Cargo.toml' } };
const cargo = (version: string) =>
  `[package]\nname = "escouade-plugin-x"\nversion = "${version}"\n\n[dependencies]\nserde = { version = "1" }\n`;
const paperwork = { LICENSE: 'text', 'CHANGELOG.md': 'text' };

const problems = (files: Record<string, string>) => {
  const dir = folder(files);
  return problemsOf(dir, readPlugin(dir, 'x'));
};

describe('readCargo', () => {
  it('reads the package, not the version of a dependency or of another table', () => {
    const text = `[workspace]\nmembers = []\n[lints]\nversion = "9.9.9"\n[package]\nname = "a"\nversion = "1.2.3"\n[dependencies]\nb = "1"\nversion = "8.8.8"\n`;
    expect(readCargo(text)).toMatchObject({ name: 'a', version: '1.2.3' });
  });

  it('holds with a [package] that ends the file, and with CRLF lines', () => {
    expect(readCargo('[package]\r\nname = "a"\r\nversion = "1.0.0"\r\n')).toMatchObject({ name: 'a', version: '1.0.0' });
  });

  it('names the plugins a crate stands on, not the crates of crates/', () => {
    const text = `[package]\nname = "a"\n[dependencies]\nacp = { path = "../acp" }\nkit = { path = "../../crates/escouade-plugin-kit" }\n`;
    expect(readCargo(text).plugins).toEqual(['acp']);
  });

  it('reads every plugin crate of the repository', () => {
    const crates = listPlugins('plugins').filter((p) => p.cargo);
    expect(crates.length).toBeGreaterThan(0);
    for (const { cargo: c } of crates)
      expect(c).toMatchObject({ name: expect.stringMatching(/^escouade-plugin-/), version: expect.any(String) });
  });
});

describe('problemsOf', () => {
  it('finds nothing wrong in a crate whose manifest says its version', () => {
    expect(problems({ ...paperwork, 'escouade-plugin.json': manifest(WASM), 'Cargo.toml': cargo('2.2.0') })).toEqual([]);
  });

  it('refuses a manifest and a crate of two versions', () => {
    expect(problems({ ...paperwork, 'escouade-plugin.json': manifest(WASM), 'Cargo.toml': cargo('2.2.1') })).toEqual([
      'escouade-plugin.json says version 2.2.0, Cargo.toml 2.2.1',
    ]);
  });

  it('takes 0.0.0 from a plugin the app gives, whose version the app stamps, and from no other', () => {
    const stamped = { version: '0.0.0' };
    expect(problems({ ...paperwork, 'escouade-plugin.json': manifest({ ...stamped, ...APP }), 'Cargo.toml': cargo('1.0.0') })).toEqual([]);
    expect(problems({ ...paperwork, 'escouade-plugin.json': manifest({ ...stamped, ...WASM }), 'Cargo.toml': cargo('1.0.0') })).toEqual([
      'escouade-plugin.json says version 0.0.0, Cargo.toml 1.0.0',
    ]);
  });

  it('holds a plugin the app gives to its version when it says one', () => {
    expect(problems({ ...paperwork, 'escouade-plugin.json': manifest(APP), 'Cargo.toml': cargo('1.0.0') })).toEqual([
      'escouade-plugin.json says version 2.2.0, Cargo.toml 1.0.0',
    ]);
  });

  it('asks every plugin, of any sort, for a LICENSE and a CHANGELOG with something in them', () => {
    expect(problems({ 'escouade-plugin.json': manifest({}) })).toEqual(['LICENSE is missing or empty', 'CHANGELOG.md is missing or empty']);
    expect(problems({ LICENSE: '', 'CHANGELOG.md': 'text', 'escouade-plugin.json': manifest({}) })).toEqual([
      'LICENSE is missing or empty',
    ]);
  });

  it('refuses a Cargo.toml with no package', () => {
    expect(problems({ ...paperwork, 'escouade-plugin.json': manifest(WASM), 'Cargo.toml': '[dependencies]\n' })).toEqual([
      'Cargo.toml has no [package] name and version',
    ]);
  });
});

describe('readPlugin', () => {
  it('sorts a plugin by its folder and its program', () => {
    const dir = folder({ ...paperwork, 'escouade-plugin.json': manifest(WASM), 'Cargo.toml': cargo('2.2.0') });
    expect(readPlugin(dir, 'x')).toMatchObject({ wasm: true, ofTheApp: false, node: false, cargo: { name: 'escouade-plugin-x' } });
    const app = folder({ 'escouade-plugin.json': manifest(APP) });
    expect(readPlugin(app, 'x')).toMatchObject({ wasm: false, ofTheApp: true, cargo: null });
  });
});

describe('touchedBy', () => {
  const plugin = (name: string, ...plugins: string[]) => ({ name, cargo: { plugins } });
  const plugins = [plugin('acp'), plugin('mistral', 'acp'), plugin('above', 'mistral'), plugin('github'), { name: 'slack', cargo: null }];
  const names = (files: string[]) => touchedBy(files, plugins).map((p: { name: string }) => p.name);

  it('runs only the folders touched, not a folder whose name starts the same', () => {
    expect(names(['plugins/github/src/lib.rs', 'plugins/slack/README.md'])).toEqual(['github', 'slack']);
    expect(names(['plugins/github-old/x', 'plugins/README.md', 'src/App.svelte'])).toEqual([]);
  });

  it('runs what stands on a plugin touched, step by step', () => {
    expect(names(['plugins/acp/src/lib.rs'])).toEqual(['acp', 'mistral', 'above']);
    expect(names(['plugins/mistral/Cargo.toml'])).toEqual(['mistral', 'above']);
  });

  it.each([
    'crates/escouade-plugin-kit/src/lib.rs',
    'schemas/plugin-1.json',
    'sdk/plugin/src/index.ts',
    'src-tauri/src/plugins/bench/mod.rs',
    'src-tauri/src/bin/escouade-plugin.rs',
    'src-tauri/Cargo.lock',
    'src-tauri/rust-toolchain.toml',
    'plugins/ci.mjs',
    'plugins/ci-lib.mjs',
  ])('runs them all for %s', (file) => {
    expect(names([file])).toEqual(plugins.map((p) => p.name));
  });

  it('runs nothing for a change that does not touch them', () => {
    expect(names(['src-tauri/src/core.rs', 'docs/PLUGINS.md', 'src-tauri/src/plugins-notes.md'])).toEqual([]);
    expect(names([])).toEqual([]);
  });
});

describe('the workspace of the modules', () => {
  const ported = listPlugins('plugins').filter((p) => p.wasm && p.cargo);
  const root = readFileSync('plugins/Cargo.toml', 'utf8');
  const members = /^members = \[(.*)\]$/m
    .exec(root)?.[1]
    .split(',')
    .map((m) => m.trim().replaceAll('"', ''));

  it('holds each plugin in run.wasm, and only those', () => {
    expect(ported.length).toBeGreaterThanOrEqual(4);
    expect([...(members ?? [])].sort()).toEqual(ported.map((p) => p.name).sort());
  });

  it('gives each of them a form the store can compile, its folder alone', () => {
    for (const { name } of ported) {
      const toml = readFileSync(`plugins/${name}/Cargo.toml`, 'utf8');
      // An `rlib` too for what the core still links (Claude Code, Codex, Copilot, OpenCode, Antigravity, until P8): docs/PLUGINS.md §8.2.
      expect(toml, name).toMatch(/^crate-type = \["cdylib"(, "rlib")?\]$/m);
      expect(toml, name).not.toMatch(/^workspace\s*=/m);
      expect(toml, name).not.toMatch(/path\s*=/);
    }
  });

  it('has a Cargo.lock for each, but those a crate of crates.io holds back, which are out of the list once it is there', () => {
    for (const { name } of ported) {
      const waiting = name in AWAITING_PUBLICATION;
      expect(existsSync(`plugins/${name}/Cargo.lock`), `${name}: AWAITING_PUBLICATION of plugins/ci-lib.mjs`).toBe(!waiting);
    }
    for (const name of Object.keys(AWAITING_PUBLICATION)) expect(ported.map((p) => p.name)).toContain(name);
  });

  it('pins the toolchain of the app', () => {
    const lf = (f: string) => readFileSync(f, 'utf8').replace(/\r\n/g, '\n');
    expect(lf('plugins/rust-toolchain.toml')).toBe(lf('src-tauri/rust-toolchain.toml'));
  });

  it('brings the kit back to crates/, for the crates its plugins stand on', () => {
    const patch = /^\[patch\.crates-io\]\n((?:.+\n)+)/m.exec(root.replace(/\r\n/g, '\n'))?.[1] ?? '';
    for (const name of ['escouade-plugin-protocol', 'escouade-plugin-kit'])
      expect(patch).toContain(`${name} = { path = "../crates/${name}" }`);
  });
});

describe('parseArgs', () => {
  it('reads nothing, --changed and --changed <revision>', () => {
    expect(parseArgs([])).toEqual({ changed: false });
    expect(parseArgs(['--changed'])).toEqual({ changed: true, base: undefined });
    expect(parseArgs(['--changed', 'HEAD^1'])).toEqual({ changed: true, base: 'HEAD^1' });
  });

  it.each([[['--all']], [['--changed', '--all']], [['--changed', 'a', 'b']]])('refuses %j', (args) => {
    expect(() => parseArgs(args)).toThrow(/usage/);
  });
});

// The script itself, in a repository of its own: its copy of plugins/ sees only what this test put there. A dozen
// processes of git and node per test, which a loaded machine makes slow.
describe('plugins/ci.mjs --changed', { timeout: 60_000 }, () => {
  const repo = () => {
    const dir = folder({});
    const inside = join(dir, 'plugins');
    mkdirSync(join(inside, 'one'), { recursive: true });
    for (const file of ['ci.mjs', 'ci-lib.mjs']) writeFileSync(join(inside, file), readFileSync(join('plugins', file)));
    writeFileSync(join(inside, 'one', 'escouade-plugin.json'), manifest({}));
    const git = (...args: string[]) =>
      execFileSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@t', ...args], { cwd: dir, stdio: 'pipe' });
    git('init', '-q', '-b', 'main');
    git('add', '.');
    git('commit', '-qm', 'base');
    return { dir, git };
  };
  const ci = (cwd: string, ...args: string[]) => {
    const env = { ...process.env };
    delete env.ESCOUADE_PLUGIN_BIN;
    return spawnSync(process.execPath, ['plugins/ci.mjs', ...args], { cwd, encoding: 'utf8', env });
  };

  it('passes without the bench when no plugin is touched', () => {
    const { dir } = repo();
    writeFileSync(join(dir, 'notes.md'), 'text');
    const r = ci(dir, '--changed');
    expect(r.stdout).toContain('0 of 1 plugins to check');
    expect(r.status).toBe(0);
  });

  it('checks a plugin touched only in the working tree', () => {
    const { dir } = repo();
    writeFileSync(join(dir, 'plugins', 'one', 'new.txt'), 'text');
    const r = ci(dir, '--changed');
    expect(r.stdout).toContain('1 of 1 plugins to check');
    expect(r.stderr).toContain('ESCOUADE_PLUGIN_BIN');
    expect(r.status).toBe(2);
  });

  it('compares with the revision it is given', () => {
    const { dir, git } = repo();
    writeFileSync(join(dir, 'plugins', 'one', 'new.txt'), 'text');
    git('add', '.');
    git('commit', '-qm', 'next');
    expect(ci(dir, '--changed', 'HEAD').stdout).toContain('0 of 1 plugins to check');
    expect(ci(dir, '--changed', 'HEAD^1').stdout).toContain('1 of 1 plugins to check');
  });

  it('runs them all when git cannot compare: a shallow clone never passes by testing nothing', () => {
    const { dir } = repo();
    const r = ci(dir, '--changed', 'no-such-revision');
    expect(r.stderr).toContain('all the plugins run');
    expect(r.status).toBe(2);
  });
});
