import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { desc } from 'drizzle-orm';
import { revocationLists } from '../db/schema';
import type { Db } from './db';
import { readPublicKey, type PublicKey } from './minisign';
import { checkRevocations } from './revocation-list';

// The revocation list (docs/STORE.md, §4.6): signed on the user's machine with the store's key, never on the server.
// The store takes a list in only if one of the public keys of `store/keys/` (the ones the app embeds) signed it and
// its serial is above the last one; it serves the last list as it was signed, bytes and `.minisig`, and drops the
// versions it disables from the catalogue. A copy is published on escouade.dev (scripts/copy-revocations.ts).

/** The public keys of `dir` (`*.pub`, minisign's files): the ones the app embeds. A file that does not read is said. */
export async function loadKeys(dir: string): Promise<PublicKey[]> {
  const names = await readdir(dir).catch(() => [] as string[]);
  const keys: PublicKey[] = [];
  for (const name of names.filter((n) => n.endsWith('.pub')).sort()) {
    const key = readPublicKey(await readFile(join(dir, name), 'utf8'));
    if (key) keys.push(key);
    else console.warn(`[store] ${name} in ${dir} is not a minisign public key: left out`);
  }
  return keys;
}

export interface StoredList {
  serial: number;
  json: string;
  minisig: string;
  issuedAt: Date;
  receivedAt: Date;
}

/** The last list taken in, or null before the first. */
export async function lastRevocations(db: Db): Promise<StoredList | null> {
  const [row] = await db.select().from(revocationLists).orderBy(desc(revocationLists.serial)).limit(1);
  return row ?? null;
}

/** Takes a list in (the deposit of STORE-8 calls it): checked against the keys and the last serial, then kept. */
export async function acceptRevocations(
  db: Db,
  keys: readonly PublicKey[],
  json: string,
  minisig: string,
  now = new Date(),
): Promise<{ serial: number } | { error: string }> {
  const last = await lastRevocations(db);
  const checked = checkRevocations(json, minisig, keys, last?.serial ?? 0);
  if ('error' in checked) return checked;
  const done = await db
    .insert(revocationLists)
    .values({ serial: checked.list.serial, json, minisig, issuedAt: new Date(checked.list.issued_at), receivedAt: now })
    .onConflictDoNothing()
    .returning({ serial: revocationLists.serial });
  if (!done.length) return { error: `serial ${checked.list.serial} is already taken` };
  return { serial: checked.list.serial };
}
