import { describe, expect, it } from 'vitest';
import { randomToken, sameSecret, seal, tokenHash, unseal, webhookSignature, webhookSigned } from '../../server/utils/tokens';

describe('the tokens', () => {
  it('are random, URL-safe, and kept by their hash only', () => {
    const a = randomToken();
    expect(a).toMatch(/^[\w-]{43}$/);
    expect(randomToken()).not.toBe(a);
    expect(tokenHash(a)).toMatch(/^[0-9a-f]{64}$/);
    expect(tokenHash(a)).not.toContain(a);
  });

  it('compare in constant time, whatever their lengths', () => {
    expect(sameSecret('abc', 'abc')).toBe(true);
    expect(sameSecret('abc', 'abd')).toBe(false);
    expect(sameSecret('abc', 'abcd')).toBe(false);
  });
});

describe('the GitHub token sealed in the base', () => {
  const secret = 'x'.repeat(64);

  it('opens with the same secret, never shows in clear, and differs each time', () => {
    const a = seal(secret, 'ghu_token');
    expect(a).not.toContain('ghu_token');
    expect(seal(secret, 'ghu_token')).not.toBe(a);
    expect(unseal(secret, a)).toBe('ghu_token');
  });

  it('does not open with another secret, or once changed', () => {
    const a = seal(secret, 'ghu_token');
    expect(unseal('y'.repeat(64), a)).toBeNull();
    const parts = a.split('.');
    parts[3] = Buffer.from('tampered').toString('base64url');
    expect(unseal(secret, parts.join('.'))).toBeNull();
    expect(unseal(secret, 'v0.a.b.c')).toBeNull();
    expect(unseal(secret, 'garbage')).toBeNull();
  });
});

describe('the signature of the webhooks', () => {
  const body = Buffer.from('{"action":"created"}');

  it('is GitHub’s: HMAC-SHA256 of the raw body, in hex, after sha256=', () => {
    // The example of GitHub's documentation ("Validating webhook deliveries").
    expect(webhookSignature("It's a Secret to Everybody", 'Hello, World!')).toBe(
      'sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17',
    );
  });

  it('accepts the right one and refuses any other, a missing one, or a missing secret', () => {
    const sig = webhookSignature('s3cret', body);
    expect(webhookSigned('s3cret', body, sig)).toBe(true);
    expect(webhookSigned('other', body, sig)).toBe(false);
    expect(webhookSigned('s3cret', Buffer.from('{"action":"deleted"}'), sig)).toBe(false);
    expect(webhookSigned('s3cret', body, sig.replace('sha256=', 'sha1='))).toBe(false);
    expect(webhookSigned('s3cret', body, undefined)).toBe(false);
    expect(webhookSigned('', body, webhookSignature('', body))).toBe(false);
  });
});
