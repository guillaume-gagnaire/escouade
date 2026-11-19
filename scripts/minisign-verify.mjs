// Checks a file signed as the Tauri updater signs (ESC-515): `.sig` is a minisign signature file in base64, the key a
// minisign public key file in base64, as `plugins.updater.pubkey` of src-tauri/tauri.conf.json holds it (its key when
// none is given). The signature of the file (`ED`: Ed25519 over its BLAKE2b-512), the key's id, then the global
// signature over it and the trusted comment. Node's crypto alone. The Release checks each archive it signs with it.
//
//   node scripts/minisign-verify.mjs <file> <file.sig> [<public key, base64>]
import { createHash, createPublicKey, verify } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

/** The public key of the app's updater. */
export function updaterKey() {
  const conf = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'src-tauri', 'tauri.conf.json');
  return JSON.parse(fs.readFileSync(conf, 'utf8')).plugins.updater.pubkey;
}

const b64 = (s) => Buffer.from(String(s).trim(), 'base64');

/** Throws unless `data` is signed by `sigB64` under `keyB64` (both in the Tauri updater's base64). */
export function check(data, sigB64, keyB64) {
  const key = b64(keyB64).toString('utf8').split('\n');
  const sig = b64(sigB64).toString('utf8').split('\n');
  const pk = b64(key[1] ?? '');
  const s = b64(sig[1] ?? '');
  if (pk.length !== 42 || pk.subarray(0, 2).toString() !== 'Ed') throw new Error('not a minisign public key');
  if (s.length !== 74 || s.subarray(0, 2).toString() !== 'ED') throw new Error('not a prehashed minisign signature');
  if (!pk.subarray(2, 10).equals(s.subarray(2, 10))) throw new Error('signed by another key');
  const publicKey = createPublicKey({
    key: Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), pk.subarray(10)]),
    format: 'der',
    type: 'spki',
  });
  const hash = createHash('blake2b512').update(data).digest();
  if (!verify(null, hash, publicKey, s.subarray(10))) throw new Error('the signature does not hold');
  const comment = (sig[2] ?? '').replace(/^trusted comment: /, '');
  const global = b64(sig[3] ?? '');
  if (!verify(null, Buffer.concat([s.subarray(10), Buffer.from(comment)]), publicKey, global)) {
    throw new Error('the trusted comment is not signed');
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  const [file, sigFile, key] = process.argv.slice(2);
  if (!file || !sigFile) {
    console.error('usage: minisign-verify.mjs <file> <file.sig> [<public key, base64>]');
    process.exit(2);
  }
  try {
    check(fs.readFileSync(file), fs.readFileSync(sigFile, 'utf8'), key || updaterKey());
    console.log(`${path.basename(file)}: signature checked`);
  } catch (e) {
    console.error(`${path.basename(file)}: ${e.message}`);
    process.exit(1);
  }
}
