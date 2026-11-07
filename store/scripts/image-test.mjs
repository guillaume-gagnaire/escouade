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
// A `run.wasm` plugin needs bubblewrap, which Docker's default profiles refuse: under docker, the program runs twice, as
// the image comes (it must say that such a version waits) and with the two profiles of store/deploy (ESC-356, docs/STORE.md
// §3.3). The AppArmor one must be loaded on the host first, which a test does not do for itself:
//
//   sudo apparmor_parser -r store/deploy/apparmor-bubblewrap
//
// `RUST_OPTIONS` replaces them, to try others (`--security-opt` values, separated by spaces). With the two of them, what
// they do not open must stay closed: system calls under seccomp, what bubblewrap may mount under AppArmor.

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

// ---- What lets bubblewrap run: the seccomp profile, and the AppArmor one where this engine has AppArmor.
const seccompOption = `seccomp=${join(store, 'deploy', 'seccomp-bubblewrap.json')}`;
const apparmorOption = 'apparmor=escouade-store-bubblewrap';
const secured = (options) => options.flatMap((o) => ['--security-opt', o]);

/** System calls the profiles leave refused: the x86-64 number and arguments of each. */
const CALLS = {
  // keyctl(KEYCTL_GET_KEYRING_ID, KEY_SPEC_SESSION_KEYRING, 0).
  keyctl: [250, 0, 0xfffffffd, 0],
  // A bad descriptor: seccomp answers EPERM before the kernel answers EBADF.
  setns: [308, -1, 0],
  // What `unshare -rn` asks: bubblewrap's rule is CLONE_NEWUSER alone.
  'unshare -rn': [272, 0x50000000],
  // CLONE_NEWUSER and SIGCHLD: neither of the two sets of namespaces of bubblewrap.
  'clone one namespace': [56, 0x10000011, 0, 0, 0, 0],
};

/** Runs each `name:number:arguments` in a process of its own, and prints `name=errno`; 0 when the kernel let it through. */
const CALLS_PERL = String.raw`
  $| = 1;
  for my $line (@ARGV) {
    my ($name, @call) = split /:/, $line;
    my $pid = fork();
    if (!$pid) {
      my ($number, @words) = map { $_ + 0 } @call;
      my $r = syscall($number, @words);
      my $errno = $! + 0;
      exit 0 if $r == 0 && $call[0] == 56;
      print "$name=", ($r < 0 ? $errno : 0), "\n";
      exit 0;
    }
    waitpid($pid, 0);
  }
`;

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
  let profiles = [];

  /** What the profiles leave closed. Each call is also made without them, or its refusal would prove nothing. */
  const narrow = (armoured) => {
    const asked = Object.entries(CALLS).map(([name, call]) => [name, ...call].join(':'));
    const errnos = (options) =>
      Object.fromEntries(
        inImage(['perl', '-e', CALLS_PERL, ...asked], options)
          .trim()
          .split('\n')
          .map((l) => l.split('=')),
      );
    const closed = errnos(secured(profiles));
    const open = errnos(secured(['seccomp=unconfined', ...(armoured ? ['apparmor=unconfined'] : [])]));
    console.log(`errno with the profiles ${JSON.stringify(closed)}, without them ${JSON.stringify(open)}`);
    for (const name of Object.keys(CALLS)) {
      if (closed[name] !== '1') fail(`${name}: errno ${closed[name]} with the profiles, not EPERM`);
      if (open[name] === '1') fail(`${name}: refused without the profiles too, so this proves nothing`);
      console.log(`✓ still refused: ${name}`);
    }
    if (!armoured) return;
    // What bubblewrap may mount: a line that runs to its end, then with a filesystem or a source the profile leaves out.
    const root = ['--ro-bind', '/opt/sandbox/usr', '/usr'];
    const links = ['bin', 'lib', 'lib64'].flatMap((d) => ['--symlink', `usr/${d}`, `/${d}`]);
    const bwrap = (more) => ['bwrap', '--unshare-all', ...root, ...links, ...more, '/bin/true'];
    inImage(bwrap([]), secured(profiles));
    for (const [what, more] of [
      ['a filesystem of another type', ['--mqueue', '/mq']],
      ["the store's folder", ['--ro-bind', '/app', '/app']],
      ["the store's archives", ['--ro-bind', '/data', '/data']],
    ]) {
      const [exe, ...words] = bwrap(more);
      const r = run('run', '--rm', ...secured(profiles), '--entrypoint', exe, image, ...words);
      if (r.status === 0 || !/Permission denied/.test(r.stderr)) fail(`${what}: code ${r.status}, ${r.stderr}`);
      console.log(`✓ AppArmor refuses to mount ${what}: ${r.stderr.trim()}`);
    }
  };

  let out = inImage(['node', '/test/compile.mjs'], mounts);
  console.log(out.trim());
  if (!parsed(out).find((l) => 'bubblewrap' in l)?.bubblewrap) {
    const waits = parsed(out).find((l) => 'rust' in l);
    if (waits?.rust !== 'waits' || !/no sandbox/.test(waits.said) || waits.plugin) fail(`without bubblewrap: ${JSON.stringify(waits)}`);
    console.log('✓ without bubblewrap, a run.wasm version waits');
    // Where the engine has AppArmor it takes both profiles, and neither is enough alone: each one opens something for the other.
    const armoured = run('info', '--format', '{{.SecurityOptions}}').stdout.includes('name=apparmor');
    console.log(`this engine ${armoured ? 'has' : 'has no'} AppArmor`);
    profiles = process.env.RUST_OPTIONS?.split(' ') ?? [seccompOption, ...(armoured ? [apparmorOption] : [])];
    if (armoured) {
      for (const option of [seccompOption, apparmorOption]) {
        const probe = parsed(inImage(['node', '/test/compile.mjs'], ['-e', 'IMAGE_TEST_PROBE=1', ...secured([option]), ...mounts]));
        const line = probe.find((l) => 'bubblewrap' in l);
        console.log(`${option} alone: ${JSON.stringify(line)}`);
        if (line.bubblewrap) fail(`${option} alone lets bubblewrap run`);
      }
    }
    out = inImage(['node', '/test/compile.mjs'], [...secured(profiles), ...mounts]);
    console.log(out.trim());
    if (!process.env.RUST_OPTIONS) narrow(armoured);
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
    const { privateKey } = generateKeyPairSync('rsa', { modulusLength: 2048 });
    const key = privateKey.export({ type: 'pkcs1', format: 'pem' });
    /** The store started with these options: it must end its start, and say `says` of a `run.wasm`. */
    const start = (options, says) => {
      const name = `store-image-test-${randomUUID().slice(0, 8)}`;
      const started = run(
        'run',
        '-d',
        '--name',
        name,
        '--network',
        'host',
        ...secured(options),
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
        if (!says.test(log)) fail(`the start does not say ${says}`);
        if (!log.includes('[store] ready')) fail('the store did not say it is ready');
      } finally {
        run('rm', '-f', name);
      }
    };
    // As the image comes first, where that is not already the sandbox: the store starts, and says why such a version waits.
    if (profiles.length) {
      start([], /\[store\] run\.wasm: no sandbox here \(bwrap: .+\): such versions wait in error/);
      console.log(`✓ as the image comes, the store starts and says that a run.wasm waits, with bubblewrap's reason`);
    }
    start(profiles, /\[store\] run\.wasm: in a sandbox \(bubblewrap\)/);
    console.log('✓ the store starts with its compiler, and a run.wasm has its sandbox');
  }
} catch (e) {
  failed = true;
  console.error(e);
} finally {
  rmSync(work, { recursive: true, force: true });
}
process.exit(failed ? 1 : 0);
