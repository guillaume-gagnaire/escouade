import { readSignature, verifySigned, type PublicKey } from './minisign';

// The revocation list's file (docs/STORE.md, §4.6), read and checked without a database: the store runs it before it
// takes a list in, and scripts/copy-revocations.ts before it copies the list on escouade.dev. It imports nothing but
// minisign.ts, so that the copy runs without the store's dependencies.

/** What a list's trusted comment starts with, followed by its serial: a signature of something else does not pass. */
export const REVOCATIONS_COMMENT = 'escouade-revocations-v1';

export interface RevocationEntry {
  id: string;
  versions: '*' | string[];
  action: 'unverify' | 'disable';
  reason: { fr: string; en: string };
  date: string;
}

export interface RevocationList {
  serial: number;
  issued_at: string;
  entries: RevocationEntry[];
  /** The ids of the keys no longer to be trusted (a key lost or stolen, §7), signed by the other key. */
  keys_revoked?: string[];
}

const ISO = /^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:\d{2}))?$/;
const isText = (v: unknown): v is string => typeof v === 'string' && v.trim() !== '';

/** The list read from its bytes, or what is wrong with it. */
export function parseRevocations(json: string): RevocationList | { error: string } {
  let v: unknown;
  try {
    v = JSON.parse(json);
  } catch {
    return { error: 'not JSON' };
  }
  if (!v || typeof v !== 'object' || Array.isArray(v)) return { error: 'not an object' };
  const o = v as Record<string, unknown>;
  if (!Number.isSafeInteger(o.serial) || (o.serial as number) < 1) return { error: 'serial: a whole number from 1' };
  if (!isText(o.issued_at) || !ISO.test(o.issued_at) || Number.isNaN(Date.parse(o.issued_at))) return { error: 'issued_at: a date' };
  if (!Array.isArray(o.entries)) return { error: 'entries: a list' };
  for (const [i, e] of (o.entries as unknown[]).entries()) {
    const x = (e ?? {}) as Record<string, unknown>;
    const at = `entries[${i}]`;
    if (!isText(x.id)) return { error: `${at}.id` };
    if (x.versions !== '*' && !(Array.isArray(x.versions) && x.versions.length && x.versions.every(isText)))
      return { error: `${at}.versions: "*" or a list of versions` };
    if (x.action !== 'unverify' && x.action !== 'disable') return { error: `${at}.action: unverify or disable` };
    const r = (x.reason ?? {}) as Record<string, unknown>;
    if (!isText(r.fr) || !isText(r.en)) return { error: `${at}.reason: fr and en` };
    if (!isText(x.date) || !ISO.test(x.date)) return { error: `${at}.date` };
  }
  if (o.keys_revoked !== undefined && !(Array.isArray(o.keys_revoked) && o.keys_revoked.every(isText)))
    return { error: 'keys_revoked: a list of key ids' };
  return o as unknown as RevocationList;
}

export type Checked = { list: RevocationList; key: PublicKey } | { error: string };

/**
 * Whether `json` and `minisig` make a list the store may serve: signed by one of `keys` (with the list's own
 * comment and serial), well formed, and above `after`. The same check runs before the copy on escouade.dev.
 */
export function checkRevocations(json: string, minisig: string, keys: readonly PublicKey[], after = 0): Checked {
  const sig = readSignature(minisig);
  if (!sig) return { error: 'the .minisig does not read' };
  const key = verifySigned(json, sig, keys);
  if (!key) return { error: 'not signed by a key of the store' };
  const list = parseRevocations(json);
  if ('error' in list) return list;
  const [word, serial] = sig.trustedComment.split(/\s+/);
  if (word !== REVOCATIONS_COMMENT || serial !== String(list.serial))
    return { error: `the trusted comment must be "${REVOCATIONS_COMMENT} ${list.serial} …"` };
  if (list.keys_revoked?.includes(key.id)) return { error: 'signed by a key the list itself revokes' };
  if (list.serial <= after) return { error: `serial ${list.serial} is not above ${after}` };
  return { list, key };
}

/** Whether `list` disables (or unverifies) version `version` of plugin `id`: its entry, or null. */
export function revoked(list: RevocationList | null, id: string, version: string, action: RevocationEntry['action']) {
  return list?.entries.find((e) => e.id === id && e.action === action && (e.versions === '*' || e.versions.includes(version))) ?? null;
}
