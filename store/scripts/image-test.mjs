#!/usr/bin/env node
// The store's image, once built (ESC-328, docs/STORE.md §3.3): its compiler answers, a `run.js` plugin compiles in it
// twice to the same bytes, by the store's own `sandboxedCompiler`, as the app's repository says it should, and the store
// starts with it.
//
//   node store/scripts/image-test.mjs <image> [<escouade-js.wasm.sha256>]
//
// The second argument is the escouade-js CI built for the app: the image must embed the same. ENGINE names the container
// engine (docker by default). IMAGE_TEST_DATABASE_URL (a base reachable from the host's network) adds the start of
// the store, on IMAGE_TEST_PORT (3000 by default). The test's own program, tests/image/compile.ts, runs in the image.
//
// A `run.wasm` plugin needs bubblewrap, which Docker's default seccomp refuses: under docker, the program runs twice, as
// the image comes (it must say that such a version waits) and with what lets it run (`RUST_OPTIONS`, ESC-336).

import { execFileSync, spawnSync } from 'node:child_process';
import { createHash, generateKeyPairSync, randomUUID } from 'node:crypto';
import { chmodSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const store = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repo = resolve(store, '..');
const engine = process.env.ENGINE || 'docker';
const [image, sumFile] = process.argv.slice(2);
if (!image) {
  console.error('usage: image-test.mjs <image> [<escouade-js.wasm.sha256>]');
  process.exit(2);
}

function fail(why) {
  console.error(`✗ ${why}`);
  process.exit(1);
}

function run(...args) {
  const r = spawnSync(engine, args, { encoding: 'utf8', maxBuffer: 16 << 20 });
  if (r.error) fail(`${engine}: ${r.error.message}`);
  return r;
}

/** A program of the image, its output whole; its failure ends the test. */
function inImage(words, options = []) {
  const r = run('run', '--rm', ...options, '--entrypoint', words[0], image, ...words.slice(1));
  if (r.status !== 0) fail(`${words.join(' ')}: code ${r.status}\n${r.stdout}${r.stderr}`);
  return r.stdout;
}

// ---- The compiler answers.
const version = inImage(['escouade-plugin', '--version']);
console.log(version.trim());
const said = (word) => version.match(new RegExp(`^${word} (\\S+)$`, 'm'))?.[1];
const engineSum = said('escouade-js');
if (said('javy') !== '9.1.0') fail('not the pinned Javy');
if (!engineSum || engineSum === 'absent') fail('escouade-plugin has no escouade-js');
if (sumFile) {
  const want = readFileSync(sumFile, 'utf8').trim();
  if (want !== engineSum) fail(`the image embeds escouade-js ${engineSum}, CI built ${want}: the review could not compare them`);
  console.log('✓ the same escouade-js as CI');
}
const env = inImage(['sh', '-c', 'echo "$STORE_ESCOUADE_PLUGIN $STORE_JAVY"; "$STORE_JAVY" --version; command -v unshare wget']);
console.log(env.trim());
if (!env.startsWith('/usr/local/bin/escouade-plugin /usr/local/bin/javy\n')) fail('STORE_ESCOUADE_PLUGIN and STORE_JAVY are not set');

// ---- A `run.js` plugin compiles twice to the same bytes, by the store's own code.
const work = mkdtempSync(join(tmpdir(), 'image-test-'));
// mkdtemp makes it 0700, and the image runs as `node`, which must read it.
chmodSync(work, 0o755);
let failed = false;
try {
  const esbuild = process.platform === 'win32' ? 'esbuild.cmd' : 'esbuild';
  execFileSync(
    join(store, 'node_modules', '.bin', esbuild),
    [
      'tests/image/compile.ts',
      '--bundle',
      '--platform=node',
      '--format=esm',
      `--outfile=${join(work, 'compile.mjs')}`,
      '--log-level=error',
    ],
    {
      cwd: store,
      stdio: 'inherit',
      shell: process.platform === 'win32',
    },
  );
  const mounts = [
    '-v',
    `${work}:/test:ro`,
    '-v',
    `${join(repo, 'plugins')}:/plugins:ro`,
    '-v',
    `${join(repo, 'tests', 'fixtures', 'plugins', 'rust-build')}:/rust-build:ro`,
  ];
  const parsed = (text) =>
    text
      .split('\n')
      .filter((l) => l.startsWith('{'))
      .map((l) => JSON.parse(l));
  let out = inImage(['node', '/test/compile.mjs'], mounts);
  console.log(out.trim());
  if (!parsed(out).find((l) => 'bubblewrap' in l)?.bubblewrap) {
    const waits = parsed(out).find((l) => 'rust' in l);
    if (waits?.rust !== 'waits' || !/no sandbox/.test(waits.said) || waits.plugin) fail(`without bubblewrap: ${JSON.stringify(waits)}`);
    console.log('✓ without bubblewrap, a run.wasm version waits');
    // What each option does alone: the options a server needs, for the profile of ESC-356.
    for (const option of ['seccomp=unconfined', 'apparmor=unconfined']) {
      const probe = parsed(inImage(['node', '/test/compile.mjs'], ['-e', 'IMAGE_TEST_PROBE=1', '--security-opt', option, ...mounts]));
      console.log(`${option} alone: ${JSON.stringify(probe.find((l) => 'bubblewrap' in l))}`);
    }
    const options = (process.env.RUST_OPTIONS ?? 'seccomp=unconfined apparmor=unconfined').split(' ').flatMap((o) => ['--security-opt', o]);
    out = inImage(['node', '/test/compile.mjs'], [...options, ...mounts]);
    console.log(out.trim());
  }
  const said2 = parsed(out);
  const lines = said2.filter((l) => 'plugin' in l && l.plugin !== 'rust-build' && 'compilation' in l).map((l) => JSON.stringify(l));
  const goldens = JSON.parse(readFileSync(join(repo, 'crates', 'escouade-plugin-cli', 'goldens.json'), 'utf8'));
  const prelude = said('prelude');
  for (const line of lines) {
    const { plugin, compilation } = JSON.parse(line);
    if (compilation.state !== 'built') fail(`${plugin}: ${JSON.stringify(compilation)}`);
    if (compilation.compiler.engine !== engineSum) fail(`${plugin}: compiled by another escouade-js`);
    const golden = goldens.plugins[plugin];
    const bundle = createHash('sha256')
      .update(readFileSync(join(repo, 'plugins', plugin, 'dist', 'plugin.mjs')))
      .digest('hex');
    if (goldens.engine === engineSum && goldens.prelude === prelude && golden?.bundle === bundle) {
      if (golden.module !== compilation.sha256) fail(`${plugin}: ${compilation.sha256}, not the ${golden.module} of goldens.json`);
      console.log(`✓ ${plugin}: twice the same module, the one of goldens.json (${compilation.sha256})`);
    } else {
      console.log(`✓ ${plugin}: twice the same module (${compilation.sha256}); goldens.json speaks of another compiler or bundle`);
    }
  }
  if (lines.length !== 2) fail(`${lines.length} plugins compiled, not 2`);

  // ---- A `run.wasm` plugin: the hash of goldens.json, what is refused, and what its sandbox shows.
  const sandbox = said2.find((l) => 'bubblewrap' in l);
  if (!sandbox?.bubblewrap) fail(`bubblewrap cannot run in this container: ${sandbox?.why}`);
  const rust = said2.find((l) => l.plugin === 'rust-build');
  if (!rust || rust.compilation.state !== 'built') fail(`rust-build: ${JSON.stringify(rust)}`);
  if (rust.compilation.sha256 !== goldens.rust.module)
    fail(`rust-build: ${rust.compilation.sha256}, not the ${goldens.rust.module} of goldens.json`);
  if (rust.compilation.compiler.canon !== goldens.rust.canon) fail('rust-build: another rustc-canon');
  console.log(`✓ rust-build: twice the same module, the one of goldens.json (${rust.compilation.sha256}), in ${rust.seconds} s`);
  const refused = (name, why) => {
    const r = said2.find((l) => l.refused === name);
    if (!r || !why.test(r.said)) fail(`${name}: ${JSON.stringify(r)}`);
    console.log(`✓ refused, ${name}: ${r.said.slice(0, 120)}`);
  };
  refused('lock', /does not match|needs to be updated/);
  refused('build.rs', /has a build\.rs/);
  refused('passwd', /couldn't read|No such file/);
  refused('store', /couldn't read|No such file/);
  const inside = said2.find((l) => 'inside' in l)?.inside ?? [];
  console.log(inside.join('\n'));
  const want = ['app absent', 'data absent', 'proc absent', 'etc []', 'env [PATH PWD ]', 'net unreachable'];
  if (want.join('|') !== inside.join('|')) fail(`the sandbox shows more than it should: ${inside.join(' | ')}`);
  console.log('✓ the sandbox shows neither the store, nor its variables, nor a network');

  // ---- The store starts, and says what it has.
  const database = process.env.IMAGE_TEST_DATABASE_URL;
  if (database) {
    const port = process.env.IMAGE_TEST_PORT || '3000';
    const name = `store-image-test-${randomUUID().slice(0, 8)}`;
    const { privateKey } = generateKeyPairSync('rsa', { modulusLength: 2048 });
    const key = privateKey.export({ type: 'pkcs1', format: 'pem' });
    const started = run(
      'run',
      '-d',
      '--name',
      name,
      '--network',
      'host',
      '-e',
      `PORT=${port}`,
      '-e',
      `DATABASE_URL=${database}`,
      '-e',
      'GITHUB_APP_ID=1',
      '-e',
      'GITHUB_APP_CLIENT_ID=test',
      '-e',
      'GITHUB_APP_CLIENT_SECRET=test',
      '-e',
      'GITHUB_WEBHOOK_SECRET=test',
      '-e',
      `GITHUB_APP_PRIVATE_KEY=${key}`,
      image,
    );
    if (started.status !== 0) fail(started.stderr);
    try {
      let ok = false;
      for (let i = 0; i < 60 && !ok; i++) {
        const health = run('exec', name, 'wget', '-qO-', `http://127.0.0.1:${port}/api/health`);
        ok = health.status === 0;
        if (ok) console.log(health.stdout.trim());
        else spawnSync(process.execPath, ['-e', 'setTimeout(() => {}, 1000)']);
      }
      const logs = run('logs', name);
      const log = logs.stdout + logs.stderr;
      console.log(log.trim());
      if (!ok) fail('/api/health never said ok');
      if (!/\[store\] compiler: \/usr\/local\/bin\/escouade-plugin, /.test(log)) fail('the start does not name the compiler');
      if (!log.includes('[store] run.wasm: ')) fail('the start does not say how a run.wasm compiles');
      if (!log.includes('[store] ready')) fail('the store did not say it is ready');
      console.log('✓ the store starts with its compiler');
    } finally {
      run('rm', '-f', name);
    }
  }
} catch (e) {
  failed = true;
  console.error(e);
} finally {
  rmSync(work, { recursive: true, force: true });
}
process.exit(failed ? 1 : 0);
