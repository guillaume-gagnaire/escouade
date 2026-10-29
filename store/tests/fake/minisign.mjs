import { createHash, generateKeyPairSync, randomBytes, sign } from 'node:crypto';

// A minisign key made for a test (never a file of the repository): its `.pub` text and a signer that writes
// `.minisig` files as minisign does (prehashed, `ED`). The real key of the store lives on the user's machine alone.

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
