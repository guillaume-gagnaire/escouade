// XChaCha20-Poly1305 en JavaScript, sans dépendance : HChaCha20 (ici) puis le ChaCha20-Poly1305 de Node (12 octets de
// nonce). Le même format que `shared/core` : nonce (24 octets) puis texte chiffré et étiquette, en base64 URL.
import { createCipheriv, createDecipheriv, randomBytes } from 'node:crypto';

const rotl = (v, c) => ((v << c) | (v >>> (32 - c))) >>> 0;

function quarter(s, a, b, c, d) {
  s[a] = (s[a] + s[b]) >>> 0; s[d] = rotl(s[d] ^ s[a], 16);
  s[c] = (s[c] + s[d]) >>> 0; s[b] = rotl(s[b] ^ s[c], 12);
  s[a] = (s[a] + s[b]) >>> 0; s[d] = rotl(s[d] ^ s[a], 8);
  s[c] = (s[c] + s[d]) >>> 0; s[b] = rotl(s[b] ^ s[c], 7);
}

/** HChaCha20 (draft-irtf-cfrg-xchacha) : la sous-clé de 32 octets tirée de la clé et des 16 premiers octets du nonce. */
export function hchacha20(key, nonce16) {
  const s = new Uint32Array(16);
  s.set([0x61707865, 0x3320646e, 0x79622d32, 0x6b206574]);
  for (let i = 0; i < 8; i++) s[4 + i] = key.readUInt32LE(i * 4);
  for (let i = 0; i < 4; i++) s[12 + i] = nonce16.readUInt32LE(i * 4);
  for (let i = 0; i < 10; i++) {
    quarter(s, 0, 4, 8, 12); quarter(s, 1, 5, 9, 13); quarter(s, 2, 6, 10, 14); quarter(s, 3, 7, 11, 15);
    quarter(s, 0, 5, 10, 15); quarter(s, 1, 6, 11, 12); quarter(s, 2, 7, 8, 13); quarter(s, 3, 4, 9, 14);
  }
  const out = Buffer.alloc(32);
  [0, 1, 2, 3, 12, 13, 14, 15].forEach((w, i) => out.writeUInt32LE(s[w], i * 4));
  return out;
}

const parts = (key, nonce) => {
  const sub = hchacha20(key, nonce.subarray(0, 16));
  const iv = Buffer.concat([Buffer.alloc(4), nonce.subarray(16, 24)]);
  return { sub, iv };
};

export function seal(keyB64, plaintext, nonce = randomBytes(24)) {
  const key = Buffer.from(keyB64, 'base64url');
  if (key.length !== 32) throw new Error('clé de 32 octets attendue');
  const { sub, iv } = parts(key, nonce);
  const c = createCipheriv('chacha20-poly1305', sub, iv, { authTagLength: 16 });
  const ct = Buffer.concat([c.update(plaintext, 'utf8'), c.final(), c.getAuthTag()]);
  return Buffer.concat([nonce, ct]).toString('base64url');
}

export function open(keyB64, sealedB64) {
  const key = Buffer.from(keyB64, 'base64url');
  const raw = Buffer.from(sealedB64, 'base64url');
  const nonce = raw.subarray(0, 24);
  const tag = raw.subarray(raw.length - 16);
  const ct = raw.subarray(24, raw.length - 16);
  const { sub, iv } = parts(key, nonce);
  const d = createDecipheriv('chacha20-poly1305', sub, iv, { authTagLength: 16 });
  d.setAuthTag(tag);
  return Buffer.concat([d.update(ct), d.final()]).toString('utf8');
}
