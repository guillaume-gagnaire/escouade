import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { BenchSetup } from '../../scripts/plugin-review/bench';
import {
  buildArgs,
  containerCompiler,
  DEFAULT_RUST_IMAGE,
  fetchArgs,
  prepareArgs,
  readArgs,
  TOOLS_VOLUME,
} from '../../scripts/plugin-review/container';
import { parseTarget } from '../../scripts/plugin-review/download';
import { findLeads, type Lead } from '../../scripts/plugin-review/leads';
import { blockers, proposed, renderPublicEn } from '../../scripts/plugin-review/report';
import { review, type ReviewOptions } from '../../scripts/plugin-review/review';
import { lockChanges, moduleImports, moduleLeads, readLock, readNativeList, rustLeads } from '../../scripts/plugin-review/rust-leads';
import { sandboxedCompiler, type CompileOne } from '../../server/utils/compile';
import { startReviewStore } from '../fake/review-store.mjs';

// The review of a `run.wasm` plugin (ESC-337, docs/PLUGINS.md §8.5.5): the leads of Rust, the recompilation in a container
// for a third party and on the machine for the first hand, on a fake store and a fake engine. ESCOUADE_REVIEW_REAL_RUST=1
// and ESCOUADE_PLUGIN_LINUX play the real one, in a real container, never in CI.

const here = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(here, '../../..');
const ENGINE = join(here, '../fake/fake-engine.mjs');
const FAKE_CLI = join(here, '../fake/fake-escouade-plugin.mjs');
const FIXTURE = join(REPO, 'tests/fixtures/plugins/rust-build');
// The store's image holds store/ alone: these tests read the repository's fixtures and the closed list of crates/.
const whole = existsSync(FIXTURE) && existsSync(join(REPO, 'crates/escouade-plugin-cli/src/rust_native.json'));

const text = (o: Record<string, string>) => new Map(Object.entries(o).map(([k, v]) => [k, Buffer.from(v)]));
const of = (leads: Lead[], kind: Lead['kind']) => leads.filter((l) => l.kind === kind);

/** A WASM module that has these imports of functions. */
function wasmWith(imports: [string, string][]): Buffer {
  const name = (s: string) => [s.length, ...Buffer.from(s)];
  const body = [imports.length, ...imports.flatMap(([m, n]) => [...name(m), ...name(n), 0, 0])];
  return Buffer.from([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0, 2, (body.length & 0x7f) | 0x80, body.length >> 7, ...body]);
}

describe.skipIf(!whole)('the leads of Rust', () => {
  const leads = (files: Record<string, string>, module: Buffer | null = null, permissions = {}) =>
    rustLeads(text(files), module, permissions, readNativeList());

  it('point at build.rs and the procedural macros', () => {
    const got = leads({
      'build.rs': 'fn main() {}',
      'Cargo.toml': '[lib]\nproc-macro = true\n[build-dependencies]\ncc = "1"\n# proc-macro = true\n',
    });
    expect(of(got, 'native').map((l) => [l.file, l.line, l.why])).toEqual([
      ['build.rs', undefined, 'script de compilation : il tourne en natif à la compilation'],
      ['Cargo.toml', 2, 'macro procédurale : elle tourne en natif à la compilation'],
      ['Cargo.toml', 3, 'dépendances d’un script de compilation'],
    ]);
  });

  it('point at unsafe, extern and #[link], not at a comment', () => {
    const got = leads({
      'src/lib.rs':
        '// unsafe in words\nfn a() { unsafe { b() } }\n#[link(name = "c")]\nextern "C" { fn d(); }\n#[no_mangle]\npub fn e() {}\n',
    });
    expect(of(got, 'unsafe').map((l) => l.line)).toEqual([2, 3, 4, 5]);
  });

  it('point at what is included from outside the folder, not beside the source', () => {
    const got = of(
      leads({
        'src/a/lib.rs': [
          'const A: &[u8] = include_bytes!("/home/u/.ssh/id_ed25519");',
          'const B: &str = include_str!("C:\\\\Users\\\\u\\\\secret");',
          'include!("../../../up.rs");',
          'const D: &str = include_str!(',
          '  "../../../outside.txt"',
          ');',
          'const E: &str = include_str!("../beside.txt");',
          'const F: &str = include_str!(concat!(env!("HOME"), "/x"));',
          '#[path = "../../../up.rs"] mod up;',
          '#[path = "here.rs"] mod here;',
          'use std::include_bytes as grab;',
          '',
        ].join('\n'),
      }),
      'includes',
    );
    expect(got.map((l) => l.line)).toEqual([1, 2, 3, 4, 8, 9, 11]);
    expect(got[0]!.why).toBe('`include_bytes!` : chemin absolu : il lit un fichier de la machine qui compile');
    expect(got[2]!.why).toBe('`include!` : chemin qui sort du dossier du plugin');
    expect(got[4]!.why).toBe('`include_str!` d’un chemin calculé : on ne lit pas ce qu’il inclut');
    expect(got[5]!.why).toBe('`#[path]` : chemin qui sort du dossier du plugin');
    expect(got[6]!.why).toBe('macro d’inclusion renommée');
  });

  it('say what the compilation’s environment puts in the module', () => {
    const got = leads({ 'src/lib.rs': 'const K: &str = env!("AWS_SECRET_ACCESS_KEY");\n' });
    expect(of(got, 'secrets')[0]).toMatchObject({ line: 1, why: 'variable lue à la compilation : sa valeur entre dans le module' });
  });

  it('read Cargo.lock dependency by dependency, against crates.io and the closed list', () => {
    const lock = (name: string, version: string, source: string, sum?: string) =>
      `[[package]]\nname = "${name}"\nversion = "${version}"\nsource = "${source}"\n${sum ? `checksum = "${sum}"\n` : ''}\n`;
    const io = 'registry+https://github.com/rust-lang/crates.io-index';
    const serde = readNativeList().find((n) => n.name === 'serde')!;
    const got = of(
      leads({
        'Cargo.lock': [
          '# This file is automatically @generated by Cargo.\nversion = 4\n\n',
          '[[package]]\nname = "me"\nversion = "1.0.0"\n\n',
          lock('serde', serde.version, io, serde.checksum),
          lock('serde', '0.1.0', io, 'a'.repeat(64)),
          lock('left-pad', '1.3.0', io, 'b'.repeat(64)),
          lock('helper', '0.1.0', 'git+https://github.com/q7x/helper#abc'),
          lock('nosum', '0.1.0', io),
        ].join(''),
      }),
      'dependencies',
    );
    expect(got[0]!.why).toBe('6 paquets : 1 sur la liste fermée, 4 hors liste, 1 du plugin');
    expect(got.slice(1).map((l) => [l.text, l.why.split(' :')[0]])).toEqual([
      ['serde 0.1.0', 'sur la liste fermée (`rust_native.json`) sous une autre version ou une autre somme'],
      ['left-pad 1.3.0', 'hors de la liste fermée (`rust_native.json`)'],
      ['helper 0.1.0', 'source hors de crates.io'],
      ['helper 0.1.0', 'hors de la liste fermée (`rust_native.json`)'],
      ['nosum 0.1.0', 'pas de somme de contrôle'],
      ['nosum 0.1.0', 'hors de la liste fermée (`rust_native.json`)'],
    ]);
  });

  it('take a lock and a crate’s own pages for no host the plugin reaches', () => {
    const got = findLeads(
      text({
        'Cargo.lock': '[[package]]\nname = "a"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n',
        'Cargo.toml': '[package]\nrepository = "https://github.com/q7x/a"\n',
        'src/lib.rs': 'const U: &str = "https://collect.q7x-telemetry.net/k";\n',
      }),
      [],
    );
    expect(of(got, 'host').map((l) => l.file)).toEqual(['src/lib.rs']);
  });

  it('take the fixture’s lock from crates.io, on the closed list where it runs in native', () => {
    const packages = readLock(readFileSync(join(FIXTURE, 'Cargo.lock'), 'utf8'));
    expect(packages.length).toBeGreaterThan(5);
    const got = of(
      rustLeads(text({ 'Cargo.lock': readFileSync(join(FIXTURE, 'Cargo.lock'), 'utf8') }), null, {}, readNativeList()),
      'dependencies',
    );
    expect(got.filter((l) => /source hors|autre version/.test(l.why))).toEqual([]);
  });

  it('say what the lock adds or changes since the version before', () => {
    const before = '[[package]]\nname = "a"\nversion = "1.0.0"\nchecksum = "x"\n';
    const now = `${before.replace('1.0.0', '1.1.0')}\n[[package]]\nname = "b"\nversion = "2.0.0"\nchecksum = "y"\n`;
    expect(lockChanges(before, now).map((l) => [l.text, l.why.split(' :')[0]])).toEqual([
      ['a 1.1.0', 'change depuis la version d’avant (1.0.0)'],
      ['b 2.0.0', 'nouvelle depuis la version d’avant'],
    ]);
    expect(lockChanges(before, before)).toEqual([]);
  });

  it('read code that cannot be read', () => {
    const got = leads({ 'src/lib.rs': `// a\u202eb\nconst X: [u8; 16] = [${Array(16).fill('0x41').join(', ')},];\n` });
    expect(of(got, 'obscure').map((l) => l.line)).toEqual([1, 2]);
  });

  it('read the imports of a module', () => {
    const imports: [string, string][] = [
      ['wasi_snapshot_preview1', 'fd_write'],
      ['escouade', 'call'],
    ];
    expect(moduleImports(wasmWith(imports))).toEqual(imports.map(([module, name]) => ({ module, name, kind: 0 })));
    expect(moduleImports(Buffer.from('not a module'))).toBeNull();
    expect(moduleImports(Buffer.from([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0, 2, 9, 1]))).toBeNull();
    expect(moduleImports(Buffer.from([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0]))).toEqual([]);
  });

  it('read a real module: the reactor of the tests, its capabilities declared', () => {
    const module = readFileSync(join(REPO, 'tests/fixtures/plugins/reactor/plugin.wasm'));
    const manifest = JSON.parse(readFileSync(join(REPO, 'tests/fixtures/plugins/reactor/escouade-plugin.json'), 'utf8')) as {
      permissions: Record<string, unknown>;
    };
    const imports = moduleImports(module)!;
    expect(imports.some((i) => i.module === 'wasi_snapshot_preview1' && i.name === 'fd_write')).toBe(true);
    expect(imports.some((i) => i.module === 'escouade' && i.name === 'call')).toBe(true);
    const got = moduleLeads('plugin.wasm', module, manifest.permissions);
    expect(got.filter((l) => /seraient refusés|ne se chargerait pas/.test(l.why))).toEqual([]);
    expect(moduleLeads('plugin.wasm', module, {}).some((l) => /net\/fetch\b.* sans permissions\.network/.test(l.why))).toBe(true);
  });

  it('compare them with the capabilities the manifest declares', () => {
    const base = wasmWith([
      ['wasi_snapshot_preview1', 'fd_write'],
      ['wasi_snapshot_preview1', 'path_open'],
      ['wasi_snapshot_preview1', 'sock_recv'],
      ['escouade', 'call'],
      ['escouade', 'other'],
      ['env', 'evil'],
    ]);
    const module = Buffer.concat([base, Buffer.from('\0net/fetch\0fs/write\0library/std/src/fs/read.rs\0proc/spawn\0')]);
    const got = moduleLeads('plugin.wasm', module, { files: [{ root: 'agent', access: 'read' }] });
    expect(got.map((l) => l.text ?? l.why)).toEqual([
      '6 imports : wasi_snapshot_preview1, escouade, env',
      'wasi_snapshot_preview1.path_open',
      'wasi_snapshot_preview1.sock_recv',
      'escouade.other',
      'env.evil',
      'le module nomme net/fetch sans permissions.network : ces appels seraient refusés (-32003)',
      'le module nomme proc/spawn sans permissions.commands : ces appels seraient refusés (-32003)',
    ]);
    expect(moduleLeads('plugin.wasm', base, { network: ['api.linear.app'] }).at(-1)!.why).toMatch(
      /^permissions\.network est déclaré, et le module ne nomme/,
    );
    expect(moduleLeads('plugin.wasm', Buffer.from('garbage'), {})[0]).toMatchObject({ kind: 'obscure' });
  });

  it('find the methods in the glued strings of a module', () => {
    // As Codex's module has them: no separator between two strings.
    const glued = Buffer.concat([wasmWith([['escouade', 'call']]), Buffer.from('\0commitsfs/gitProtectedprotectedfs/read\0\0')]);
    const files = { files: [{ root: 'agentRepo', access: 'read' }] };
    expect(moduleLeads('plugin.wasm', glued, files).filter((l) => /ne nomme aucune/.test(l.why))).toEqual([]);
    expect(moduleLeads('plugin.wasm', glued, {}).at(-1)!.why).toBe(
      'le module nomme fs/gitProtected, fs/read sans permissions.files : ces appels seraient refusés (-32003)',
    );
    // Each alone, its neighbours glued or not; a path of the standard library is still none.
    for (const text of ['xfs/gitProtectedy', '\0fs/gitProtected\0', 'agentfs/read\0']) {
      const one = Buffer.concat([wasmWith([['escouade', 'call']]), Buffer.from(text)]);
      expect(moduleLeads('plugin.wasm', one, files).filter((l) => /ne nomme aucune/.test(l.why))).toEqual([]);
    }
    const path = Buffer.concat([wasmWith([['escouade', 'call']]), Buffer.from('\0library/std/src/fs/read.rs\0')]);
    expect(moduleLeads('plugin.wasm', path, files).at(-1)!.why).toMatch(/^permissions\.files est déclaré, et le module ne nomme/);
  });
});

describe.skipIf(!whole)('the steps of a third party’s compilation', () => {
  const s = { engine: { program: 'podman' }, cli: '/opt/cli', image: 'img', work: '/w' };
  const volumes = (args: string[]) => args.filter((_, i) => args[i - 1] === '--volume');

  it('runs fetch alone with a network, and writes to the tools there alone', () => {
    expect(fetchArgs('n', s, 'tools', '/w/plugin')).not.toContain('--network=none');
    for (const args of [
      buildArgs('n', s, 'tools', 'work', '/w/plugin'),
      readArgs('n', s, 'work'),
      prepareArgs('n', 'img', 'tools', 'work'),
    ])
      expect(args).toContain('--network=none');
    expect(volumes(fetchArgs('n', s, 'tools', '/w/plugin'))).toEqual([
      '/opt/cli:/usr/local/bin/escouade-plugin:ro',
      'tools:/tools',
      '/w/plugin:/plugin:ro',
    ]);
    expect(volumes(buildArgs('n', s, 'tools', 'work', '/w/plugin'))).toEqual([
      '/opt/cli:/usr/local/bin/escouade-plugin:ro',
      'tools:/tools:ro',
      'work:/work',
      '/w/plugin:/plugin:ro',
    ]);
    expect(volumes(readArgs('n', s, 'work'))).toEqual(['/opt/cli:/usr/local/bin/escouade-plugin:ro', 'work:/work:ro']);
    // Nothing of the plugin in the step that runs as root.
    expect(volumes(prepareArgs('n', 'img', 'tools', 'work'))).toEqual(['tools:/tools', 'work:/work']);
  });

  it('holds what the plugin’s code meets to the bench’s limits', () => {
    const args = buildArgs('n', s, 'tools', 'work', '/w/plugin');
    for (const word of ['--read-only', '--cap-drop=ALL', '--security-opt=no-new-privileges', '--user=65534:65534', '--env=HOME=/tmp'])
      expect(args).toContain(word);
    expect(args.join(' ')).toContain('--env=CARGO_NET_OFFLINE=true');
    expect(args.slice(args.indexOf('build'))).toEqual(['build', '/plugin', '--out', '/work/out', '--json', '--lang', 'en']);
  });
});

// A review starts node several times by step: minutes on a machine other agents are using.
describe.skipIf(!whole)('a run.wasm plugin', { timeout: 180_000 }, () => {
  let root: string;
  let store: Awaited<ReturnType<typeof startReviewStore>>;
  let log: string;
  const noCompile: CompileOne = () => Promise.reject(new Error('a third party’s run.wasm reached this machine’s compiler'));

  beforeAll(async () => {
    root = mkdtempSync(join(tmpdir(), 'plugin-review-rust-'));
    store = await startReviewStore();
    process.env.FAKE_ENGINE_VOLUMES = join(root, 'volumes');
    log = join(root, 'engine.log');
    process.env.FAKE_ENGINE_LOG = log;
  });
  afterAll(async () => {
    delete process.env.FAKE_ENGINE_VOLUMES;
    delete process.env.FAKE_ENGINE_LOG;
    await store.close();
    rmSync(root, { recursive: true, force: true });
  });

  const options = (target: string, over: Partial<ReviewOptions> = {}): ReviewOptions => {
    const work = join(root, 'work');
    const bench: BenchSetup = { engine: null, cli: null, image: 'img', timeout: 30_000, local: FAKE_CLI };
    return {
      ...parseTarget(target)!,
      source: { store: store.url, githubApi: store.githubApi },
      work,
      reports: join(root, 'reports'),
      bench,
      compile: noCompile,
      container: containerCompiler({
        engine: { program: process.execPath, prefix: [ENGINE] },
        cli: '/opt/escouade-plugin',
        image: 'img',
        work,
      }),
      repo: REPO,
      now: new Date('2026-11-05T10:00:00Z'),
      ...over,
    };
  };
  const ran = () =>
    existsSync(log)
      ? readFileSync(log, 'utf8')
          .trim()
          .split('\n')
          .map((l) => JSON.parse(l) as string[])
      : [];

  it('compiles a third party’s sources in the container, plays the module in WASM, down to the report', async () => {
    rmSync(log, { force: true });
    const done = await review(options('dev.exemple.rust-lines@1.0.0'));
    expect(done.code).toBe(0);
    expect(done.data.recompile).toMatchObject({ state: 'same', sha256: done.data.wasm!.sha256, where: 'container' });
    expect(done.data.bench).toMatchObject({ played: true, passed: true, where: 'wasm' });
    expect(blockers(done.data)).toEqual([]);
    expect(proposed(done.data).verdict).toBeNull();
    const md = readFileSync(done.report, 'utf8');
    expect(md).toContain('- Recompilé dans un conteneur sans réseau avec les mêmes versions : **même empreinte**');
    expect(md).toContain('rustc 1.98.0, wasm32-wasip1, rustc-canon');
    expect(md).toContain('- tourne en WASM (`run.wasm`)');
    expect(md).toContain('### Dépendances');
    expect(readFileSync(done.summary, 'utf8')).toContain('tourne en WASM (`run.wasm`)');
    expect(renderPublicEn(done.data)).toContain('runs in WASM (`run.wasm`)');
    // The four steps, and the volume of this compilation gone.
    const steps = ran();
    expect(steps.filter((a) => a[0] === 'run')).toHaveLength(4);
    expect(steps.at(-1)!.slice(0, 3)).toEqual(['volume', 'rm', '--force']);
  });

  it('shows a container neither HOME, nor the signing key, nor the repository, and leaves nothing behind', async () => {
    rmSync(log, { force: true });
    process.env.ESCOUADE_STORE_TOKEN = 'esc_admin_must-not-leak';
    try {
      await review(options('dev.exemple.rust-lines@1.0.0'));
    } finally {
      delete process.env.ESCOUADE_STORE_TOKEN;
    }
    const steps = ran().filter((a) => a[0] === 'run');
    const hosts = steps.flatMap((a) => a.filter((_, i) => a[i - 1] === '--volume')).map((v) => v.slice(0, v.search(/:\/[a-z]/)));
    const folders = [...new Set(hosts)].filter((h) => /[\\/]/.test(h) && h !== '/opt/escouade-plugin');
    expect(folders).toHaveLength(1);
    expect(folders[0]!.startsWith(join(root, 'work', 'compile-'))).toBe(true);
    for (const host of folders) {
      expect(host).not.toBe(homedir());
      expect(host).not.toContain('.escouade-signing');
      expect(resolve(host).startsWith(REPO)).toBe(false);
    }
    expect(ran().flat().join(' ')).not.toContain('esc_admin_must-not-leak');
    expect(readdirSync(join(root, 'work')).filter((n) => n.startsWith('compile-'))).toEqual([]);
  });

  it('refuses a published module that is not its sources compiled again: « Refuser »', async () => {
    const done = await review(options('dev.exemple.rust-lines@1.0.1'));
    expect(done.code).toBe(1);
    expect(done.data.recompile?.state).toBe('different');
    expect(done.data.integrity).toMatch(/n’est pas la recompilation des sources de l’archive/);
    expect(done.data.bench).toBeUndefined();
    expect(proposed(done.data).verdict).toBe('Refuser');
  });

  it('says it was not compiled again: another rustc-canon, no container, a refusal', async () => {
    const other = await review(options('dev.exemple.rust-lines@1.0.2'));
    expect(other.code).toBe(0);
    expect(other.data.recompile).toMatchObject({ state: 'not-done' });
    expect((other.data.recompile as { why: string }).why).toContain('rustc-canon dddddddddddd…');
    expect(proposed(other.data).verdict).toBe('Demander des changements');
    expect(readFileSync(other.report, 'utf8')).toContain('que le module WASM publié soit le build des sources relues');

    const none = await review(options('dev.exemple.rust-lines@1.0.0', { container: null, containerWhy: 'aucun moteur (test)' }));
    expect(none.data.recompile).toEqual({ state: 'not-done', why: 'pas de recompilation d’un tiers sans conteneur : aucun moteur (test)' });
    expect(blockers(none.data).join()).toMatch(/pas de recompilation d’un tiers sans conteneur/);
  });

  it('never gives a third party to this machine, whatever it has', async () => {
    let asked = 0;
    const done = await review(
      options('dev.exemple.rust-lines@1.0.0', {
        container: null,
        compile: () => {
          asked++;
          return Promise.reject(new Error('no'));
        },
      }),
    );
    expect(asked).toBe(0);
    expect(done.data.recompile?.state).toBe('not-done');
  });

  it('compiles the first hand on this machine, as the store does', async () => {
    rmSync(log, { force: true });
    const compile = sandboxedCompiler({ command: [process.execPath, FAKE_CLI], rust: 'none', data: join(root, 'data') });
    const done = await review(options('dev.escouade.rust-first@1.0.0', { compile, container: null }));
    expect(done.data.recompile).toMatchObject({ state: 'same', where: 'local' });
    expect(ran()).toEqual([]);
    expect(readFileSync(done.report, 'utf8')).toContain('- Recompilé ici avec les mêmes versions : **même empreinte**');
  });

  it('reads what the lock adds since the verified version', async () => {
    const done = await review(options('dev.exemple.rust-lines@1.1.0'));
    expect(done.data.base?.version).toBe('1.0.0');
    expect(done.data.leads!.filter((l) => l.why === 'nouvelle depuis la version d’avant : son code est à lire').map((l) => l.text)).toEqual(
      ['left-pad 1.3.0'],
    );
  });

  it('raises the leads of Rust, in the report', async () => {
    const done = await review(options('dev.exemple.rust-lines@1.2.0'));
    expect(done.code).toBe(0);
    const leads = done.data.leads!;
    expect(of(leads, 'native').map((l) => l.file)).toEqual(['native/build.rs']);
    expect(
      of(leads, 'unsafe')
        .filter((l) => l.file === 'src/trap.rs')
        .map((l) => l.line),
    ).toEqual([6]);
    expect(
      of(leads, 'includes')
        .filter((l) => l.file === 'src/trap.rs')
        .map((l) => l.line),
    ).toEqual([2, 3, 5]);
    expect(leads.some((l) => l.why === 'source hors de crates.io : git+https://github.com/q7x/helper#abcdef')).toBe(true);
    expect(leads.some((l) => l.kind === 'obscure' && l.file === 'plugin.wasm')).toBe(true);
    const md = readFileSync(done.report, 'utf8');
    for (const title of ['### Code natif à la compilation', '### Code `unsafe`', '### Fichiers inclus à la compilation', '### Dépendances'])
      expect(md).toContain(title);
  });
});

describe.runIf(process.env.ESCOUADE_REVIEW_REAL_RUST === '1')('the real compilation, in a real container', () => {
  // Read when a test runs, not when the file is collected: the store's image has no crates/.
  const golden = () =>
    (JSON.parse(readFileSync(join(REPO, 'crates/escouade-plugin-cli/goldens.json'), 'utf8')) as { rust: { module: string } }).rust.module;
  const sources = () => {
    const files = new Map<string, Buffer>();
    const walk = (dir: string) => {
      for (const name of readdirSync(dir)) {
        const at = join(dir, name);
        if (statSync(at).isDirectory()) walk(at);
        else files.set(relative(FIXTURE, at).split(sep).join('/'), readFileSync(at));
      }
    };
    walk(FIXTURE);
    return files;
  };
  const setup = () => {
    const work = mkdtempSync(join(tmpdir(), 'plugin-review-real-'));
    const one = containerCompiler({
      engine: { program: process.env.ESCOUADE_REVIEW_ENGINE ?? 'podman' },
      cli: process.env.ESCOUADE_PLUGIN_LINUX!,
      image: process.env.ESCOUADE_REVIEW_RUST_IMAGE ?? DEFAULT_RUST_IMAGE,
      work,
      tools: TOOLS_VOLUME,
    });
    return { work, one };
  };

  it('gives the fingerprint of goldens.json', async () => {
    const { work, one } = setup();
    try {
      const files = sources();
      const got = await one({ manifest: files.get('escouade-plugin.json')!, wasm: 'Cargo.toml', files });
      expect(got.sha256).toBe(golden());
    } finally {
      rmSync(work, { recursive: true, force: true });
    }
  }, 900_000);

  it('finds no file of this machine for an include_bytes! with its absolute path', async () => {
    const { work, one } = setup();
    try {
      const files = sources();
      const home = homedir().split(sep).join('/');
      const own = join(REPO, 'store/package.json').split(sep).join('/');
      const lib = files.get('src/lib.rs')!.toString('utf8');
      files.set(
        'src/lib.rs',
        Buffer.from(
          `${lib}\npub static A: &[u8] = include_bytes!("${own}");\npub static B: &[u8] = include_bytes!("${home}/.gitconfig");\n`,
        ),
      );
      await expect(one({ manifest: files.get('escouade-plugin.json')!, wasm: 'Cargo.toml', files })).rejects.toThrow(/couldn't read/);
    } finally {
      rmSync(work, { recursive: true, force: true });
    }
  }, 900_000);
});
