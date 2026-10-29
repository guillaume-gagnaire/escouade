import { signatures } from '../db/schema';
import type { Db } from './db';
import { revoked, type RevocationList } from './revocation-list';

// Which versions are verified (docs/STORE.md, §4.4): a signature taken in for the version's very `version.json` (its
// SHA-256), by a key the revocation list does not revoke, and no `unverify` entry of the list for it. Read by the
// catalogue, the public pages and the author's space; the signatures themselves come from the review (review.ts).

export interface Signed {
  sha256: string;
  keyId: string;
  signedAt: Date;
  summary: { fr: string; en: string } | null;
}

export type SignatureIndex = Map<string, Signed>;

const at = (id: string, version: string) => `${id}@${version}`;

/** Every signature taken in, by `<id>@<version>`. */
export async function signatureIndex(db: Db): Promise<SignatureIndex> {
  const rows = await db.select().from(signatures);
  return new Map(
    rows.map((r) => [at(r.pluginId, r.version), { sha256: r.versionSha256, keyId: r.keyId, signedAt: r.signedAt, summary: r.summary }]),
  );
}

/** The signature that makes version `version` of `id` (fingerprint `sha256`) verified, or null. */
export function verifiedBy(index: SignatureIndex, list: RevocationList | null, id: string, version: string, sha256: string | null) {
  const s = index.get(at(id, version));
  if (!s || !sha256 || s.sha256 !== sha256) return null;
  if (list?.keys_revoked?.includes(s.keyId)) return null;
  if (revoked(list, id, version, 'unverify')) return null;
  return s;
}
