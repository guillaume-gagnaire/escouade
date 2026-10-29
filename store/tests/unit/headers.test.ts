import { describe, expect, it } from 'vitest';
import { cleanEmail } from '../../server/utils/accounts';
import { nonceChunks, pagePolicy, STATIC_HEADERS, withNonce } from '../../server/utils/headers';

describe('the security headers', () => {
  it('are the ones asked for on 2026-10-29', () => {
    expect(STATIC_HEADERS['strict-transport-security']).toBe('max-age=31536000; includeSubDomains');
    expect(STATIC_HEADERS['x-content-type-options']).toBe('nosniff');
    expect(STATIC_HEADERS['referrer-policy']).toBe('strict-origin-when-cross-origin');
    expect(STATIC_HEADERS['permissions-policy']).toMatch(/camera=\(\).*geolocation=\(\).*microphone=\(\)/);
    expect(STATIC_HEADERS['x-frame-options']).toBe('DENY');
    expect(STATIC_HEADERS['content-security-policy']).toContain("default-src 'none'");
  });

  it('give each page a policy without any inline script but its nonce', () => {
    const p = pagePolicy('abc');
    expect(p).toContain("default-src 'self'");
    expect(p).toContain("script-src 'self' 'nonce-abc'");
    expect(p).not.toContain('unsafe-inline');
    expect(p).not.toContain('unsafe-eval');
    expect(p).toContain("frame-ancestors 'none'");
    expect(p).toContain("form-action 'self' https://github.com");
    expect(p).toContain("object-src 'none'");
  });

  it('put the nonce on scripts and styles, not on what only starts like them', () => {
    expect(withNonce('<script>a</script><style>b</style><script type="application/json">c</script><scripts>', 'n')).toBe(
      '<script nonce="n">a</script><style nonce="n">b</style><script nonce="n" type="application/json">c</script><scripts>',
    );
  });

  it('put the nonce on Nuxt’s chunks only, never on the body where an author’s README is', () => {
    const html = {
      head: ['<style>h</style>'],
      bodyPrepend: [],
      body: ['<div id="__nuxt"><article class="readme"><script>alert(1)</script><style>*{}</style></article></div>'],
      bodyAppend: ['<script type="application/json" data-nuxt-data="nuxt-app">[]</script>'],
    };
    nonceChunks(html, 'n');
    expect(html.head).toEqual(['<style nonce="n">h</style>']);
    expect(html.bodyAppend[0]).toContain('<script nonce="n" type="application/json"');
    expect(html.body[0]).not.toContain('nonce');
  });
});

describe('an e-mail address', () => {
  it('is kept trimmed, its domain in lower case', () => {
    expect(cleanEmail('  Lea@Exemple.DEV ')).toBe('Lea@exemple.dev');
  });

  it('is refused when it would not reach anyone, or could carry a header', () => {
    for (const bad of [
      '',
      'lea',
      'lea@',
      '@exemple.dev',
      'lea@exemple',
      'lea @exemple.dev',
      'lea@exemple.dev\nBcc: x@y.z',
      'a@b.c',
      42,
      null,
    ])
      expect(cleanEmail(bad)).toBeNull();
    expect(cleanEmail(`${'a'.repeat(250)}@b.dev`)).toBeNull();
  });
});
