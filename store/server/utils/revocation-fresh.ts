import { createHash, createPrivateKey, createPublicKey, generateKeyPairSync, randomBytes, sign, type KeyObject } from 'node:crypto';
import { readSignature, verifySigned, type PublicKey } from './minisign';

// The attestation of freshness of the revocation list (audit SEC-15, ESC-596): once a week, the workflow
// `.github/workflows/revocations.yml` signs « the list n° N, of fingerprint X, still holds on <date> » with a key of its
// own, the freshness key, whose secret is a secret of GitHub. That key never signs a list: stolen, it can only prolong a
// list that was signed by the user, never take a plugin out nor put one back. The app reads the attestation beside the
// copy of the list on escouade.dev, refuses one that names another fingerprint than the list it holds, and warns when
// what it holds is more than 7 days old.
//
// Its secret is not a minisign file: the first line `escouade-freshness-key-v1 <key id>`, then the PKCS#8 PEM of the
// Ed25519 key. Its public half is a minisign public key (`store/freshness/*.pub`), which the app checks with
// `minisign-verify`. Plain TypeScript that Node runs as it is: no import but node's and minisign.ts.

/** What the trusted comment of an attestation starts with, followed by the list's serial and the date. */
export const FRESH_COMMENT = 'escouade-revocations-fresh-v1';
/** The first word of the secret's file. */
export const SECRET_HEADER = 'escouade-freshness-key-v1';

export interface Freshness {
  serial: number;
  /** The SHA-256 of the list's bytes, as served, in lower case hexadecimal. */
  sha256: string;
  /** When it still holds (RFC 3339, UTC). */
  valid_at: string;
}

export interface FreshnessKey {
  id: string;
  key: KeyObject;
}

const idOf = (bytes: Buffer) => Buffer.from(bytes).reverse().toString('hex').toUpperCase();
const idBytes = (id: string) => Buffer.from(id, 'hex').reverse();
const sha256 = (data: string | Buffer) => createHash('sha256').update(data).digest('hex');

/** A new freshness key: its secret (the file the workflow takes) and its public half (a minisign `.pub`). */
export function createFreshnessKey(): { secret: string; publicKey: string; id: string } {
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  const id = idOf(randomBytes(8));
  const raw = publicKey.export({ format: 'der', type: 'spki' }).subarray(-32);
  const pub = Buffer.concat([Buffer.from('Ed', 'latin1'), idBytes(id), raw]).toString('base64');
  const pem = privateKey.export({ format: 'pem', type: 'pkcs8' }).toString();
  return {
    id,
    secret: `${SECRET_HEADER} ${id}\n${pem}`,
    publicKey: `untrusted comment: escouade freshness key ${id}\n${pub}\n`,
  };
}

/** The secret's file read, or null. */
export function readFreshnessKey(text: string): FreshnessKey | null {
  const [first, ...rest] = text.replace(/\r/g, '').trim().split('\n');
  const [word, id] = (first ?? '').trim().split(/\s+/);
  if (word !== SECRET_HEADER || !/^[0-9A-F]{16}$/.test(id ?? '')) return null;
  try {
    const key = createPrivateKey({ key: rest.join('\n'), format: 'pem' });
    if (key.asymmetricKeyType !== 'ed25519') return null;
    return { id: id!, key };
  } catch {
    return null;
  }
}

/** The public half of a secret, as minisign.ts reads public keys. */
export function publicOf(k: FreshnessKey): PublicKey {
  return { id: k.id, key: createPublicKey(k.key) };
}

/** A `.minisig` of `data` by `k` (prehashed, `ED`), its trusted comment `comment`: what minisign itself writes. */
export function minisignWith(k: FreshnessKey, data: string | Buffer, comment: string): string {
  if (/[\r\n]/.test(comment)) throw new Error('a trusted comment is one line');
  const hashed = createHash('blake2b512').update(Buffer.from(data)).digest();
  const signature = sign(null, hashed, k.key);
  const global = sign(null, Buffer.concat([signature, Buffer.from(comment, 'utf8')]), k.key);
  const line = Buffer.concat([Buffer.from('ED', 'latin1'), idBytes(k.id), signature]).toString('base64');
  return `untrusted comment: escouade freshness attestation\n${line}\ntrusted comment: ${comment}\n${global.toString('base64')}\n`;
}

/** The attestation of `listJson` (serial `serial`) on `now`, and its signature. */
export function attest(k: FreshnessKey, listJson: string, serial: number, now = new Date()): { json: string; minisig: string } {
  const valid_at = now.toISOString().replace(/\.\d{3}Z$/, 'Z');
  const fresh: Freshness = { serial, sha256: sha256(listJson), valid_at };
  const json = `${JSON.stringify(fresh, null, 2)}\n`;
  return { json, minisig: minisignWith(k, json, `${FRESH_COMMENT} ${serial} ${valid_at}`) };
}

/**
 * Whether `json` and `minisig` attest that `listJson` (serial `serial`) holds: signed by one of `keys` with the
 * attestation's comment, for this serial and this fingerprint. Gives its date, or what is wrong. The app checks the same.
 */
export function checkFreshness(
  listJson: string,
  serial: number,
  json: string,
  minisig: string,
  keys: readonly PublicKey[],
): { validAt: string } | { error: string } {
  const sig = readSignature(minisig);
  if (!sig) return { error: 'the .minisig does not read' };
  if (!verifySigned(json, sig, keys)) return { error: 'not signed by a freshness key' };
  let f: Freshness;
  try {
    f = JSON.parse(json) as Freshness;
  } catch {
    return { error: 'not JSON' };
  }
  const [word, s, date, more] = sig.trustedComment.split(' ');
  if (word !== FRESH_COMMENT || s !== String(f.serial) || date !== f.valid_at || more !== undefined)
    return { error: `the trusted comment must be "${FRESH_COMMENT} ${f.serial} ${f.valid_at}"` };
  if (f.serial !== serial) return { error: `it attests list n° ${f.serial}, not n° ${serial}` };
  if (f.sha256 !== sha256(listJson)) return { error: 'it attests another list (its fingerprint)' };
  if (Number.isNaN(Date.parse(f.valid_at))) return { error: 'valid_at: a date' };
  return { validAt: f.valid_at };
}
