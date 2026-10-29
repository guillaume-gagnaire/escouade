import { readSignature, verifySigned, type PublicKey } from './minisign';

// The signature of a verified version (docs/STORE.md, §4.5): minisign's, made on the user's machine with the store's
// key, over the version's `version.json` byte for byte, its trusted comment `escouade-verified-v1 <id>@<version>
// <date>`. The comment binds the signature to the id and the number: a verified archive served under another name
// does not pass. The store checks it before it takes it in (a mistake is not published), the review's script before
// it sends it, the app before it shows the seal. It imports nothing but minisign.ts, so that the script runs it alone.

/** What a version's trusted comment starts with, then `<id>@<version> <date>`. */
export const VERIFIED_COMMENT = 'escouade-verified-v1';

/** The trusted comment of version `version` of plugin `id`, signed on `date` (an ISO date). */
export const verifiedComment = (id: string, version: string, date: string) => `${VERIFIED_COMMENT} ${id}@${version} ${date}`;

export type CheckedSignature = { key: PublicKey; date: string } | { error: string };

/**
 * Whether `minisig` signs `versionJson` (its exact bytes) as version `version` of `id`, with one of `keys` that
 * `revokedKeys` does not name (the revocation list's `keys_revoked`, §7). Gives the key and the date of the signature.
 */
export function checkVersionSignature(
  versionJson: string | Buffer,
  minisig: string,
  keys: readonly PublicKey[],
  id: string,
  version: string,
  revokedKeys: readonly string[] = [],
): CheckedSignature {
  if (!keys.length) return { error: 'the store has no public key yet (store/keys/*.pub)' };
  const sig = readSignature(minisig);
  if (!sig) return { error: 'the .minisig does not read' };
  const trusted = keys.filter((k) => !revokedKeys.includes(k.id));
  const key = verifySigned(versionJson, sig, trusted);
  if (!key)
    return {
      error: keys.some((k) => k.id === sig.keyId && revokedKeys.includes(k.id))
        ? 'signed by a key the revocation list revokes'
        : 'not a signature of this version.json by a key of the store',
    };
  const [word, target, date, ...rest] = sig.trustedComment.split(' ');
  if (word !== VERIFIED_COMMENT || target !== `${id}@${version}` || !date || rest.length || Number.isNaN(Date.parse(date)))
    return { error: `the trusted comment must be "${verifiedComment(id, version, '<date>')}"` };
  return { key, date };
}
