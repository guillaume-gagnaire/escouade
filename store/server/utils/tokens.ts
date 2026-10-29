import { createCipheriv, createDecipheriv, createHash, createHmac, hkdfSync, randomBytes, timingSafeEqual } from 'node:crypto';

// The store's secrets at rest and in transit: the random tokens (sessions, e-mail links, the sign-in's state) whose
// base keeps only the hash, the user's GitHub token sealed with a key drawn from NUXT_SESSION_SECRET, and the
// signature of GitHub's webhooks, compared in constant time.

/** A random token: 32 bytes, URL-safe. */
export const randomToken = () => randomBytes(32).toString('base64url');

/** What the base keeps of a token: its SHA-256, so that a copy of the base opens no session. */
export const tokenHash = (token: string) => createHash('sha256').update(token).digest('hex');

/** Two strings compared in constant time (their lengths are not secret). */
export function sameSecret(a: string, b: string): boolean {
  const x = Buffer.from(a);
  const y = Buffer.from(b);
  return x.length === y.length && timingSafeEqual(x, y);
}

const sealKey = (secret: string) => Buffer.from(hkdfSync('sha256', secret, 'escouade-store', 'github-token', 32));

/** Seals `plain` (AES-256-GCM, a fresh nonce each time): `v1.<nonce>.<tag>.<cipher>`, in base64url. */
export function seal(secret: string, plain: string): string {
  const iv = randomBytes(12);
  const cipher = createCipheriv('aes-256-gcm', sealKey(secret), iv);
  const body = Buffer.concat([cipher.update(plain, 'utf8'), cipher.final()]);
  return ['v1', iv, cipher.getAuthTag(), body].map((p) => (typeof p === 'string' ? p : p.toString('base64url'))).join('.');
}

/** Opens what `seal` sealed with the same secret; null if it was changed, or sealed with another one. */
export function unseal(secret: string, sealed: string): string | null {
  const [v, iv, tag, body] = sealed.split('.');
  if (v !== 'v1' || !iv || !tag || body === undefined) return null;
  try {
    const decipher = createDecipheriv('aes-256-gcm', sealKey(secret), Buffer.from(iv, 'base64url'));
    decipher.setAuthTag(Buffer.from(tag, 'base64url'));
    return Buffer.concat([decipher.update(Buffer.from(body, 'base64url')), decipher.final()]).toString('utf8');
  } catch {
    return null;
  }
}

/** GitHub's signature of a webhook's body (`X-Hub-Signature-256`). */
export const webhookSignature = (secret: string, body: Buffer | string) =>
  `sha256=${createHmac('sha256', secret).update(body).digest('hex')}`;

/** Whether `header` is GitHub's signature of `body` with `secret`, compared in constant time. */
export function webhookSigned(secret: string, body: Buffer | string, header: string | undefined | null): boolean {
  if (!secret || !header) return false;
  return sameSecret(webhookSignature(secret, body), header);
}
