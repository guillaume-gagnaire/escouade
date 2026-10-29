// The copy of the revocation list on escouade.dev (docs/STORE.md §4.6 and §8.1): a second road for the app when the
// store is down, and a list a broken store cannot hide for long. Run every 15 minutes by the workflow
// `.github/workflows/revocations.yml`, which commits what it writes on main; Coolify then redeploys the site (its
// watched paths take website/**): the copy is an hour late at most.
//
// It takes the store's list only if one of the public keys of store/keys/ signed it, with its own comment and
// serial, and only above the serial of the copy: a store that serves an old or a forged list changes nothing, and
// the run fails, which GitHub tells. The files are copied byte for byte, the signature with them: the app checks the
// copy as it checks the store's.
//
//   npx tsx store/scripts/copy-revocations.ts [--store https://store.escouade.dev] [--out website/public]
//                                             [--keys store/keys]
//
// Prints `changed=true|false` (and appends it to $GITHUB_OUTPUT when set). Exit 1 on a list that does not hold.

import { appendFile, readdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { readPublicKey, type PublicKey } from '../server/utils/minisign';
import { checkRevocations, parseRevocations } from '../server/utils/revocation-list';

const arg = (name: string, fallback: string) => {
  const i = process.argv.indexOf(`--${name}`);
  return i > 0 && process.argv[i + 1] ? process.argv[i + 1]! : fallback;
};
const store = arg('store', 'https://store.escouade.dev').replace(/\/+$/, '');
const out = arg('out', 'website/public');
const keysDir = arg('keys', 'store/keys');

async function say(changed: boolean, line: string) {
  console.log(line);
  console.log(`changed=${changed}`);
  if (process.env.GITHUB_OUTPUT) await appendFile(process.env.GITHUB_OUTPUT, `changed=${changed}\n`);
}

async function keys(): Promise<PublicKey[]> {
  const names = (await readdir(keysDir).catch(() => [] as string[])).filter((n) => n.endsWith('.pub'));
  const read = await Promise.all(names.map(async (n) => readPublicKey(await readFile(join(keysDir, n), 'utf8'))));
  return read.filter((k): k is PublicKey => !!k);
}

async function get(path: string): Promise<string | null> {
  const r = await fetch(`${store}${path}`, { headers: { 'user-agent': 'escouade-revocations-copy' } });
  if (r.status === 404) return null;
  if (!r.ok) throw new Error(`${path}: the store answered ${r.status}`);
  return r.text();
}

const known = await keys();
if (!known.length) {
  await say(false, `No public key in ${keysDir}: nothing can be checked, nothing is copied.`);
  process.exit(0);
}
let json: string | null;
let minisig: string | null;
try {
  json = await get('/api/v1/revocations.json');
  minisig = json === null ? null : await get('/api/v1/revocations.json.minisig');
} catch (e) {
  // The store down is what the copy is for: it stays as it is. The store's own probe tells that it is down.
  await say(false, `::warning::The store did not answer (${e instanceof Error ? e.message : e}): the copy stays as it is.`);
  process.exit(0);
}
if (json === null || minisig === null) {
  await say(false, 'The store has no revocation list yet.');
  process.exit(0);
}
const current = await readFile(join(out, 'revocations.json'), 'utf8').catch(() => null);
const parsed = current === null ? null : parseRevocations(current);
const after = parsed && !('error' in parsed) ? parsed.serial : 0;
if (current === json) {
  await say(false, `The copy is up to date (serial ${after}).`);
  process.exit(0);
}
const checked = checkRevocations(json, minisig, known, after);
if ('error' in checked) {
  console.error(`::error::The store's revocation list is not copied: ${checked.error}.`);
  process.exit(1);
}
await writeFile(join(out, 'revocations.json'), json);
await writeFile(join(out, 'revocations.json.minisig'), minisig);
await say(true, `Copied the revocation list n° ${checked.list.serial} (signed by key ${checked.key.id}), after n° ${after}.`);
