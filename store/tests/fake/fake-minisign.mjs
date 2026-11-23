#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs';
import { signWithSecret, testKey } from './minisign.mjs';

// A stand-in for `minisign -S` in the tests of the review's script (ESCOUADE_MINISIGN): it reads the secret of a key
// made by minisign.mjs (`testKey().secret`, a test file, never the store's key) and writes the `.minisig` as minisign
// does (prehashed, `ED`). It asks no password. FAKE_MINISIGN_FAIL=1 makes it fail as a wrong password would.
// `-G -p <pub> -s <key> [-W]` makes a key of a test as minisign makes a pair: locked, as by a password typed (ESC-592),
// or without one under -W (or FAKE_MINISIGN_OPEN=1, as a minisign that would not ask).
const args = process.argv.slice(2);
const at = (flag) => {
  const i = args.indexOf(flag);
  return i >= 0 ? args[i + 1] : undefined;
};
if (args.includes('-G') && at('-p') && at('-s')) {
  if (process.env.FAKE_MINISIGN_FAIL === '1') {
    process.stderr.write('Passwords don’t match\n');
    process.exit(1);
  }
  const k = testKey();
  writeFileSync(at('-p'), k.pub);
  writeFileSync(at('-s'), args.includes('-W') || process.env.FAKE_MINISIGN_OPEN === '1' ? k.open : k.locked);
  process.exit(0);
}
if (!args.includes('-S') || !at('-s') || !at('-m')) {
  process.stderr.write('usage: fake-minisign -S -s <key> -m <file> [-x <sig>] [-t <trusted comment>]\n');
  process.exit(2);
}
if (process.env.FAKE_MINISIGN_FAIL === '1') {
  process.stderr.write('Wrong password for that key\n');
  process.exit(1);
}
const file = at('-m');
const trusted = at('-t') ?? `timestamp:${Math.floor(Date.now() / 1000)}`;
writeFileSync(at('-x') ?? `${file}.minisig`, signWithSecret(readFileSync(at('-s'), 'utf8'), readFileSync(file), trusted));
