import { createHash, createPrivateKey, generateKeyPairSync, randomBytes, sign } from 'node:crypto';

// A minisign key made for a test (never a file of the repository): its `.pub` text and a signer that writes
// `.minisig` files as minisign does (prehashed, `ED`). The real key of the store lives on the user's machine alone.

/**
 * The head of a minisign secret key's file, its second line (ESC-592): the algorithm, then `Sc` (scrypt) for a key
 * locked by a password, two zero bytes for one without; what admin.ts's `locked` reads, and nothing else.
 */
const head = (password) =>
  Buffer.concat([Buffer.from('Ed'), password ? Buffer.from('Sc') : Buffer.alloc(2), Buffer.from('B2'), Buffer.alloc(152)]).toString(
    'base64',
  );

/** A secret of fake-minisign.mjs written as a key of minisign is: a comment, the head, then the test's secret. */
const keyFile = (secret, password) => `untrusted comment: a key of a test (fake-minisign.mjs)\n${head(password)}\n${secret}\n`;

/** A fresh key pair: `pub` is the `.pub` file's text, `sign(data, trusted)` the `.minisig` file's. */
export function testKey() {
  const { publicKey, privateKey } = generateKeyPairSync('ed25519');
  const raw = Buffer.from(publicKey.export({ format: 'jwk' }).x, 'base64url');
  const id = randomBytes(8);
  const hex = Buffer.from(id).reverse().toString('hex').toUpperCase();
  const pub = `untrusted comment: minisign public key ${hex}\n${Buffer.concat([Buffer.from('Ed'), id, raw]).toString('base64')}\n`;
  return {
    pub,
    id: hex,
    /** What tests/fake/fake-minisign.mjs reads in place of minisign's encrypted secret key (a test file, never the real one). */
    secret: JSON.stringify({ id: id.toString('hex'), pkcs8: privateKey.export({ format: 'pem', type: 'pkcs8' }) }),
    /** The same secret in a file shaped as minisign's, locked by a password (`locked`) or not (`open`). */
    get locked() {
      return keyFile(this.secret, true);
    },
    get open() {
      return keyFile(this.secret, false);
    },
    /** @param {Buffer | string} data @param {string} trusted */
    sign(data, trusted, { legacy = false } = {}) {
      const bytes = Buffer.from(data);
      const message = legacy ? bytes : createHash('blake2b512').update(bytes).digest();
      const sig = sign(null, message, privateKey);
      const global = sign(null, Buffer.concat([sig, Buffer.from(trusted, 'utf8')]), privateKey);
      const line = Buffer.concat([Buffer.from(legacy ? 'Ed' : 'ED'), id, sig]).toString('base64');
      return `untrusted comment: signature from minisign secret key\n${line}\ntrusted comment: ${trusted}\n${global.toString('base64')}\n`;
    },
  };
}

/**
 * Signs `data` as minisign does (prehashed, `ED`) with a key's `secret` (testKey().secret, a test file): what
 * fake-minisign.mjs and the end-to-end tests use in place of the store's key.
 * @param {string} secret @param {Buffer | string} data @param {string} trusted
 */
export function signWithSecret(secret, data, trusted) {
  const s = JSON.parse(secret.trim().startsWith('{') ? secret : secret.trim().split(/\r?\n/).at(-1));
  const key = createPrivateKey(s.pkcs8);
  const id = Buffer.from(s.id, 'hex');
  const sig = sign(null, createHash('blake2b512').update(Buffer.from(data)).digest(), key);
  const global = sign(null, Buffer.concat([sig, Buffer.from(trusted, 'utf8')]), key);
  const line = Buffer.concat([Buffer.from('ED'), id, sig]).toString('base64');
  return `untrusted comment: signature from minisign secret key\n${line}\ntrusted comment: ${trusted}\n${global.toString('base64')}\n`;
}
