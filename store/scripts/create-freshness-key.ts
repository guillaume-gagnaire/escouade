// Creates the freshness key of the revocation list (audit SEC-15, ESC-596) on the user's machine, once:
//
//   cd store && npx tsx scripts/create-freshness-key.ts [--dir ~/.escouade-signing] [--fresh freshness]
//
// The secret goes to `<dir>/freshness.key` (owner only, never printed, never overwritten); the public half to
// `store/freshness/freshness.pub`. Then, as it prints: `gh secret set REVOCATIONS_FRESHNESS_KEY < <dir>/freshness.key`
// gives it to the workflow `revocations.yml`, and its public line goes into `FRESHNESS_KEYS`
// (crates/escouade-plugins/src/store_keys.rs), which a test compares with store/freshness/*.pub.
//
// That key only attests that a list signed by the user still holds: it never signs a list.

import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { createFreshnessKey } from '../server/utils/revocation-fresh';

const arg = (name: string, fallback: string) => {
  const i = process.argv.indexOf(`--${name}`);
  return i > 0 && process.argv[i + 1] ? process.argv[i + 1]! : fallback;
};
const dir = resolve(arg('dir', process.env.ESCOUADE_SIGNING_DIR ?? join(homedir(), '.escouade-signing')));
const freshDir = resolve(arg('fresh', 'freshness'));
const secretFile = join(dir, 'freshness.key');
const pubFile = join(freshDir, 'freshness.pub');

if (existsSync(secretFile) || existsSync(pubFile)) {
  console.error(`A freshness key is already there (${existsSync(secretFile) ? secretFile : pubFile}): nothing is written.`);
  process.exit(1);
}
const k = createFreshnessKey();
mkdirSync(dir, { recursive: true, mode: 0o700 });
writeFileSync(secretFile, k.secret, { mode: 0o600, flag: 'wx' });
mkdirSync(freshDir, { recursive: true });
writeFileSync(pubFile, k.publicKey, { flag: 'wx' });
const line = k.publicKey.split('\n')[1];
console.log(`Freshness key ${k.id} created.`);
console.log(`- its secret: ${secretFile} (owner only; not shown)`);
console.log(`- its public half: ${pubFile}`);
console.log('');
console.log('Then:');
console.log(`1. gh secret set REVOCATIONS_FRESHNESS_KEY < "${secretFile}"`);
console.log(
  `2. add { file: "freshness.pub", public: "${line}", roles: &[Role::Freshness] } to FRESHNESS_KEYS in crates/escouade-plugins/src/store_keys.rs, and commit it with ${pubFile}`,
);
console.log('3. keep a copy of the secret off line; if it is lost, make another one and replace both.');
