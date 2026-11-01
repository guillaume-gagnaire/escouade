#!/usr/bin/env node
// A fake `escouade-plugin build <folder> --out <folder> --json` for the store's compiler (ESC-323): it writes a
// "module" made of the bundle's bytes, and says what compiled it, as the real one does. Its first words choose a
// behaviour: `--random` makes a compiler that is not deterministic (a different module each time). A bundle that
// imports node:net is refused as the real one refuses it; one holding `LOOP` never ends; one holding `GREEDY` takes
// memory. It fails when it is given a variable of the store's (the compilation's environment is its own).
import { createHash, randomBytes } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const args = process.argv.slice(2);
const random = args[0] === '--random';
const words = random ? args.slice(1) : args;
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
