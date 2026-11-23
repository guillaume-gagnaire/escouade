// The weekly attestation of the revocation list's freshness (audit SEC-15, ESC-596): run by the workflow
// `.github/workflows/revocations.yml` every Monday, after the copy. It signs, with the freshness key of the secret
// `REVOCATIONS_FRESHNESS_KEY`, « the list n° N of fingerprint X still holds on <date> » for the list copied in
// website/public/, and writes `revocations-fresh.json` and its `.minisig` beside it: the app reads them on
// escouade.dev and warns when what it holds is more than 7 days old. It never signs a list.
//
//   REVOCATIONS_FRESHNESS_KEY=… npx tsx store/scripts/attest-revocations.ts [--out website/public] [--keys store/keys]
//                                                                          [--fresh store/freshness]
//
// Without the secret, nothing is signed: the list stays as it was signed, and the run says so in its summary (a
// warning, never a failure). Prints `changed=true|false` (and appends it to $GITHUB_OUTPUT when set). Exit 1 on a list
// that does not hold, or on a secret that does not read.

import { appendFile, readdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { readKeyFile, readPublicKey, type PublicKey } from '../server/utils/minisign';
import { checkRevocations } from '../server/utils/revocation-list';
import { attest, checkFreshness, publicOf, readFreshnessKey } from '../server/utils/revocation-fresh';

const arg = (name: string, fallback: string) => {
  const i = process.argv.indexOf(`--${name}`);
  return i > 0 && process.argv[i + 1] ? process.argv[i + 1]! : fallback;
};
const out = arg('out', 'website/public');
const keysDir = arg('keys', 'store/keys');
const freshDir = arg('fresh', 'store/freshness');

async function say(changed: boolean, line: string, summary = line) {
  console.log(line);
  console.log(`changed=${changed}`);
  if (process.env.GITHUB_OUTPUT) await appendFile(process.env.GITHUB_OUTPUT, `changed=${changed}\n`);
  if (process.env.GITHUB_STEP_SUMMARY) await appendFile(process.env.GITHUB_STEP_SUMMARY, `${summary}\n`);
}

/** The keys of `dir`: with the roles of their files' names (`store/keys/`, ESC-592), or as they are (`store/freshness/`). */
async function keysIn(dir: string, byRole: boolean): Promise<PublicKey[]> {
  const names = (await readdir(dir).catch(() => [] as string[])).filter((n) => n.endsWith('.pub'));
  const read = await Promise.all(
    names.map(async (n) => {
      const text = await readFile(join(dir, n), 'utf8');
      return byRole ? readKeyFile(n, text) : readPublicKey(text);
    }),
  );
  return read.filter((k): k is PublicKey => !!k);
}

const secret = process.env.REVOCATIONS_FRESHNESS_KEY ?? '';
if (!secret.trim()) {
  await say(
    false,
    '::warning::No secret REVOCATIONS_FRESHNESS_KEY: the revocation list is republished as it was signed, without an attestation of freshness.',
    'No freshness key (secret `REVOCATIONS_FRESHNESS_KEY`): the revocation list is republished as it was signed, without an attestation of freshness.',
  );
  process.exit(0);
}
const key = readFreshnessKey(secret);
if (!key) {
  console.error('::error::The secret REVOCATIONS_FRESHNESS_KEY does not read as a freshness key.');
  process.exit(1);
}
const json = await readFile(join(out, 'revocations.json'), 'utf8').catch(() => null);
const minisig = await readFile(join(out, 'revocations.json.minisig'), 'utf8').catch(() => null);
if (json === null || minisig === null) {
  await say(false, 'No revocation list in the copy yet: nothing to attest.');
  process.exit(0);
}
// The list judged by its role, as the copy judges it: signed by the revocation key (or the backup), never by the first hand's.
const checked = checkRevocations(json, minisig, await keysIn(keysDir, true));
if ('error' in checked) {
  console.error(`::error::The copied revocation list does not hold (${checked.error}): nothing is attested.`);
  process.exit(1);
}
const fresh = attest(key, json, checked.list.serial);
const own = checkFreshness(json, checked.list.serial, fresh.json, fresh.minisig, [publicOf(key)]);
if ('error' in own) {
  console.error(`::error::The attestation does not check (${own.error}).`);
  process.exit(1);
}
const pinned = await keysIn(freshDir, false);
if (!pinned.some((k) => k.id === key.id))
  console.log(`::warning::The freshness key ${key.id} is not in ${freshDir}/: the app does not take its attestations.`);
await writeFile(join(out, 'revocations-fresh.json'), fresh.json);
await writeFile(join(out, 'revocations-fresh.json.minisig'), fresh.minisig);
await say(true, `Attested the revocation list n° ${checked.list.serial} on ${own.validAt} (freshness key ${key.id}).`);
