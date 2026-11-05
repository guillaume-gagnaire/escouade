#!/usr/bin/env node
// A fake `escouade-plugin build <folder> --out <folder> --json` for the store's compiler (ESC-323): it writes a
// "module" made of the bundle's bytes, and says what compiled it, as the real one does. Its first words choose a
// behaviour: `--random` makes a compiler that is not deterministic (a different module each time). A bundle that
// imports node:net is refused as the real one refuses it; one holding `LOOP` never ends; one holding `GREEDY` takes
// memory. It fails when it is given a variable of the store's (the compilation's environment is its own).
//
// A `run.wasm` plugin (ESC-336): `fetch <folder>` stands for `cargo fetch --locked`, and refuses what the real one does
// (no Cargo.lock, a build.rs off the list, a lock that does not match, here one that holds `STALE`); a Cargo.toml that
// holds `NET-DOWN` fails as a network does, without a ✗. `build` then needs what `fetch` left in the data folder, and
// no network, and makes its module of the sources.
//
// `test <folder> --wasm <module> --offline --json` answers as the bench of the local review (ESC-324) does when the
// module passes, after writing its arguments beside it (<module>.args.json: the bench's environment is its own, so no
// variable of the tests reaches it); it fails when it is given a variable of the review's (a token above all).
import { createHash, randomBytes } from 'node:crypto';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const args = process.argv.slice(2);
const random = args[0] === '--random';
const words = random ? args.slice(1) : args;
if (words[0] === 'test') {
  for (const k of Object.keys(process.env))
    if (/TOKEN|SECRET|STORE_|ESCOUADE_REVIEW/.test(k)) {
      console.error(`✗ the review's variable ${k} reached the bench`);
      process.exit(3);
    }
  const module = words[words.indexOf('--wasm') + 1];
  const wasm = readFileSync(module);
  writeFileSync(`${module}.args.json`, JSON.stringify(words));
  console.log(
    JSON.stringify({
      plugin: 'x 1.0.0',
      escouade: words.includes('--escouade') ? words[words.indexOf('--escouade') + 1] : '2.1.2',
      outcomes: [
        { scenario: 'manifest', verdict: 'pass', said: 'valide, en WASM' },
        { scenario: 'initialize', verdict: 'pass', said: `répond en 3 ms (module de ${wasm.length} octets)` },
      ],
      timings: [],
      seen: [],
    }),
  );
  process.exit(0);
}
/** Every file under `dir`, by its path, sorted. */
function tree(dir, rest = '') {
  return readdirSync(join(dir, rest), { withFileTypes: true })
    .sort((a, b) => (a.name < b.name ? -1 : 1))
    .flatMap((e) => (e.isDirectory() ? tree(dir, join(rest, e.name)) : [join(rest, e.name).replace(/\\/g, '/')]));
}

if (words[0] === 'fetch') {
  const folder = words[1];
  const manifest = JSON.parse(readFileSync(join(folder, 'escouade-plugin.json'), 'utf8'));
  if (!manifest.run?.wasm) {
    console.error('✗ only a `run.wasm` plugin has dependencies to download');
    process.exit(1);
  }
  if (!existsSync(join(folder, 'Cargo.lock'))) {
    console.error('✗ no Cargo.lock: commit it, the store compiles exactly what it pins (cargo generate-lockfile)');
    process.exit(1);
  }
  if (existsSync(join(folder, 'build.rs'))) {
    console.error('✗ fake has a build.rs: a script runs in native while compiling, a plugin has none');
    process.exit(1);
  }
  if (readFileSync(join(folder, 'Cargo.lock'), 'utf8').includes('STALE')) {
    console.error('✗ Cargo.lock does not match Cargo.toml or crates.io (cargo generate-lockfile, then commit it): --locked');
    process.exit(1);
  }
  if (readFileSync(join(folder, manifest.run.wasm), 'utf8').includes('NET-DOWN')) {
    console.error('error: failed to download from `https://static.crates.io`');
    process.exit(101);
  }
  mkdirSync(process.env.ESCOUADE_DATA_DIR, { recursive: true });
  writeFileSync(join(process.env.ESCOUADE_DATA_DIR, 'fetched'), '');
  process.exit(0);
}
if (words[0] !== 'build') {
  console.error(`✗ fake escouade-plugin: ${words.join(' ')}`);
  process.exit(2);
}
for (const k of Object.keys(process.env))
  if (/DATABASE|SECRET|GITHUB|SMTP|STORE_/.test(k)) {
    console.error(`✗ the store's variable ${k} reached the compiler`);
    process.exit(3);
  }
const folder = words[1];
const out = words[words.indexOf('--out') + 1];
const manifest = JSON.parse(readFileSync(join(folder, 'escouade-plugin.json'), 'utf8'));
if (manifest.run.wasm) {
  if (!existsSync(join(process.env.ESCOUADE_DATA_DIR, 'fetched'))) {
    console.error('error: the dependencies were not fetched');
    process.exit(101);
  }
  if (process.env.CARGO_NET_OFFLINE !== 'true') {
    console.error('error: build was given a network');
    process.exit(101);
  }
  const files = tree(folder);
  const sources = Buffer.concat(files.flatMap((f) => [Buffer.from(f), readFileSync(join(folder, f))]));
  const bytes = Buffer.concat([Buffer.from([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0]), sources, random ? randomBytes(8) : Buffer.alloc(0)]);
  mkdirSync(out, { recursive: true });
  const name = `${manifest.id}-${manifest.version}.wasm`;
  writeFileSync(join(out, name), bytes);
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  const lock = createHash('sha256')
    .update(readFileSync(join(folder, 'Cargo.lock')))
    .digest('hex');
  console.log(
    JSON.stringify({
      file: join(out, name),
      size: bytes.length,
      sha256,
      compiler: { rust: '1.98.0', target: 'wasm32-wasip1', canon: 'c'.repeat(64), lock },
    }),
  );
  process.exit(0);
}
const bundle = readFileSync(join(folder, manifest.run.js));
const text = bundle.toString('utf8');
if (text.includes('CRASH')) {
  console.error('thread main panicked: out of disk');
  process.exit(101);
}
if (text.includes('node:net')) {
  console.error(`✗ ${manifest.run.js} does not compile to WASM: import of node:net refused (only node:fs and node:readline)`);
  process.exit(1);
}
// `NETCHECK`: says in its prelude whether it could reach the network (the store's sandbox must not let it).
let net = '';
if (text.includes('NETCHECK')) {
  const { connect } = await import('node:net');
  net = await new Promise((done) => {
    const s = connect({ host: '1.1.1.1', port: 53, timeout: 3000 });
    s.on('connect', () => (s.destroy(), done('+network')));
    s.on('timeout', () => (s.destroy(), done('+no-network:timeout')));
    s.on('error', (e) => done(`+no-network:${e.code}`));
  });
}
if (text.includes('LOOP')) setInterval(() => {}, 1000);
else {
  if (text.includes('GREEDY')) {
    const hold = [];
    for (let i = 0; i < 64; i++) hold.push(Buffer.alloc(16 * 1024 * 1024, i));
    globalThis.hold = hold;
    await new Promise((r) => setTimeout(r, 5000));
  }
  const bytes = Buffer.concat([Buffer.from([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0]), bundle, random ? randomBytes(8) : Buffer.alloc(0)]);
  mkdirSync(out, { recursive: true });
  const name = `${manifest.id}-${manifest.version}.wasm`;
  writeFileSync(join(out, name), bytes);
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  writeFileSync(join(out, `${name}.sha256`), `${sha256}  ${name}\n`);
  console.log(
    JSON.stringify({
      file: join(out, name),
      size: bytes.length,
      sha256,
      compiler: { javy: '9.1.0', engine: 'e'.repeat(64), prelude: (words.includes('--first-party') ? '2.2.0-first' : '2.2.0') + net },
    }),
  );
}
