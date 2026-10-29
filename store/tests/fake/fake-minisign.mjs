#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs';
import { signWithSecret } from './minisign.mjs';

// A stand-in for `minisign -S` in the tests of the review's script (ESCOUADE_MINISIGN): it reads the secret of a key
// made by minisign.mjs (`testKey().secret`, a test file, never the store's key) and writes the `.minisig` as minisign
// does (prehashed, `ED`). It asks no password. FAKE_MINISIGN_FAIL=1 makes it fail as a wrong password would.
const args = process.argv.slice(2);
const at = (flag) => {
  const i = args.indexOf(flag);
  return i >= 0 ? args[i + 1] : undefined;
};
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
