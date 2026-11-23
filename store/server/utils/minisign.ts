import { createHash, createPublicKey, verify, type KeyObject } from 'node:crypto';

// minisign's signatures (Ed25519), read without a dependency (docs/STORE.md, §4.5): what the store checks before it
// takes a signed file in (the revocation list now, the reviews' signatures with STORE-8) and what the copy of the list
// on escouade.dev checks before it publishes. The app checks the same with the `minisign-verify` crate. Only the
// prehashed form (`ED`, BLAKE2b-512 of the file, minisign's default) is taken; the legacy one (`Ed`) on request.
// Plain TypeScript that Node runs as it is (scripts/copy-revocations.ts): no import but node's.

export interface PublicKey {
  /** The key's id, 8 bytes, as minisign prints it (hexadecimal, little-endian). */
  id: string;
  key: KeyObject;
  /** What it may sign (audit SEC-03, ESC-592): from its file's name in `store/keys/` (`ROLES_BY_FILE`). Absent for a
   *  key given as it is (a test's): every role, as the backup. */
  roles?: readonly Role[];
}

/**
 * What a key of the store may sign, as the app knows it (`crates/escouade-plugins/src/store_keys.rs`, `Role`): the key
 * of the first hand seals `dev.escouade.*` alone (kept without a password, the chief signs with it after his review);
 * the third parties' seals every other id and signs the revocation list (with a password); the backup, offline, does
 * all of it.
 */
export type Role = 'first-hand' | 'third-party' | 'revocation' | 'backup';

/** The roles of a key of `store/keys/` by its file's name, the same as the app's `roles_of_file`. */
export const ROLES_BY_FILE: Readonly<Record<string, readonly Role[]>> = {
  'store.pub': ['first-hand'],
  'third-party.pub': ['third-party', 'revocation'],
  'backup.pub': ['backup'],
};

/** The ids of Escouade's own hand: what the key of the first hand seals, and the third parties' never. */
export const FIRST_HAND_PREFIX = 'dev.escouade.';

/**
 * The revocation lists the key of the first hand signed before ESC-592, by the SHA-256 of their bytes (the app's
 * `LEGACY_REVOCATION_LISTS`): they hold still, and no other list of that key does.
 */
export const LEGACY_REVOCATION_LISTS: readonly string[] = ['0a9a3c2f9b497a487a1fd95d4ae943d5eee329267a3ab2733e9b1fcb82f300b9'];

const rolesOf = (k: PublicKey): readonly Role[] => k.roles ?? ['backup'];

/** The role in which `key` seals the plugin `id`, or null when none of its roles covers that id. */
export function sealRole(key: PublicKey, id: string): Role | null {
  const first = id.startsWith(FIRST_HAND_PREFIX);
  return rolesOf(key).find((r) => r === 'backup' || (r === 'first-hand' && first) || (r === 'third-party' && !first)) ?? null;
}

/** Whether a revocation list `json` signed by `key` holds: a key that signs lists, or the first hand's on a list it signed before. */
export function signsList(key: PublicKey, json: string | Buffer): boolean {
  const roles = rolesOf(key);
  if (roles.includes('revocation') || roles.includes('backup')) return true;
  return roles.includes('first-hand') && LEGACY_REVOCATION_LISTS.includes(createHash('sha256').update(Buffer.from(json)).digest('hex'));
}

/** The role a key is named by, for a person: its first. */
export const roleOf = (k: PublicKey): Role => rolesOf(k)[0]!;

/** A `.pub` file of `store/keys/`, its roles from its name; null if it does not read or its name has no role. */
export function readKeyFile(name: string, text: string): PublicKey | null {
  const roles = ROLES_BY_FILE[name];
  const key = roles ? readPublicKey(text) : null;
  return key ? { ...key, roles } : null;
}

export interface Signature {
  algorithm: 'Ed' | 'ED';
  keyId: string;
  signature: Buffer;
  trustedComment: string;
  globalSignature: Buffer;
}

const ED25519_SPKI = Buffer.from('302a300506032b6570032100', 'hex');
const idOf = (bytes: Buffer) => Buffer.from(bytes).reverse().toString('hex').toUpperCase();
const b64 = (s: string) => (/^[A-Za-z0-9+/]+={0,2}$/.test(s) ? Buffer.from(s, 'base64') : Buffer.alloc(0));

/** A public key: its base64 line, or a `.pub` file (an untrusted comment, then that line). Null if it does not read. */
export function readPublicKey(text: string): PublicKey | null {
  const line = text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l && !l.startsWith('untrusted comment:'))[0];
  const raw = b64(line ?? '');
  if (raw.length !== 42 || raw.subarray(0, 2).toString('latin1') !== 'Ed') return null;
  try {
    const key = createPublicKey({ key: Buffer.concat([ED25519_SPKI, raw.subarray(10)]), format: 'der', type: 'spki' });
    return { id: idOf(raw.subarray(2, 10)), key };
  } catch {
    return null;
  }
}

/** A `.minisig` file: four lines (untrusted comment, signature, trusted comment, global signature). */
export function readSignature(text: string): Signature | null {
  const lines = text.replace(/\r/g, '').split('\n');
  if (lines.length < 4 || !lines[0]!.startsWith('untrusted comment:') || !lines[2]!.startsWith('trusted comment: ')) return null;
  const sig = b64(lines[1]!.trim());
  const global = b64(lines[3]!.trim());
  if (sig.length !== 74 || global.length !== 64) return null;
  const algorithm = sig.subarray(0, 2).toString('latin1');
  if (algorithm !== 'Ed' && algorithm !== 'ED') return null;
  return {
    algorithm,
    keyId: idOf(sig.subarray(2, 10)),
    signature: sig.subarray(10),
    trustedComment: lines[2]!.slice('trusted comment: '.length),
    globalSignature: global,
  };
}

/**
 * Whether `data` is signed by one of `keys`: the key's id, the signature of the file (of its BLAKE2b-512 for `ED`)
 * and the global signature, which binds the trusted comment to it. Gives the key that signed, or null.
 */
export function verifySigned(
  data: Buffer | string,
  signature: Signature,
  keys: readonly PublicKey[],
  { legacy = false } = {},
): PublicKey | null {
  if (signature.algorithm === 'Ed' && !legacy) return null;
  const key = keys.find((k) => k.id === signature.keyId);
  if (!key) return null;
  const bytes = Buffer.from(data);
  const message = signature.algorithm === 'ED' ? createHash('blake2b512').update(bytes).digest() : bytes;
  try {
    if (!verify(null, message, key.key, signature.signature)) return null;
    const global = Buffer.concat([signature.signature, Buffer.from(signature.trustedComment, 'utf8')]);
    return verify(null, global, key.key, signature.globalSignature) ? key : null;
  } catch {
    return null;
  }
}
