import { createVerify, generateKeyPairSync } from 'node:crypto';
import { describe, expect, it } from 'vitest';
import { invalidLines } from '../../server/utils/config';
import { appJwt, authorizeUrl, githubApp, installUrl, readPrivateKey } from '../../server/utils/github';

// A key made for the test, never written to a file: the repository holds no private key (tests/unit/no-secrets).
const { privateKey, publicKey } = generateKeyPairSync('rsa', { modulusLength: 2048 });
const pem = privateKey.export({ type: 'pkcs1', format: 'pem' }).toString();

const env = {
  GITHUB_APP_ID: '123456',
  GITHUB_APP_CLIENT_ID: 'Iv23liTest',
  GITHUB_APP_CLIENT_SECRET: 'secret',
  GITHUB_APP_PRIVATE_KEY: pem,
  GITHUB_WEBHOOK_SECRET: 'hook',
};

describe('the private key of the App, as Coolify holds it', () => {
  it('reads with real line breaks, with escaped ones, in quotes, or with CRLF', () => {
    expect(readPrivateKey(pem)).not.toBeNull();
    expect(readPrivateKey(pem.replace(/\n/g, '\\n'))).not.toBeNull();
    expect(readPrivateKey(`"${pem.replace(/\n/g, '\\n')}"`)).not.toBeNull();
    expect(readPrivateKey(pem.replace(/\n/g, '\r\n'))).not.toBeNull();
    expect(readPrivateKey(`  ${pem}  \n`)).not.toBeNull();
  });

  it('is refused when it is not a key, and the log says so without the value', () => {
    expect(readPrivateKey('')).toBeNull();
    const broken = ['-----BEGIN RSA ', 'PRIVATE KEY-----\nnope\n-----END RSA ', 'PRIVATE KEY-----'].join('');
    expect(readPrivateKey(broken)).toBeNull();
    expect(githubApp({ ...env, GITHUB_APP_PRIVATE_KEY: 'hunter2' })).toBeNull();
    const lines = invalidLines({ GITHUB_APP_PRIVATE_KEY: 'hunter2', NUXT_SESSION_SECRET: 'short' });
    expect(lines).toHaveLength(2);
    expect(lines.join('\n')).not.toContain('hunter2');
    expect(invalidLines({ GITHUB_APP_PRIVATE_KEY: pem.replace(/\n/g, '\\n'), NUXT_SESSION_SECRET: 'a'.repeat(64) })).toEqual([]);
  });
});

describe('the App', () => {
  it('needs its five variables, and points at GitHub unless told otherwise', () => {
    const app = githubApp(env)!;
    expect(app.web).toBe('https://github.com');
    expect(app.api).toBe('https://api.github.com');
    expect(githubApp({ ...env, GITHUB_WEBHOOK_SECRET: ' ' })).toBeNull();
    const fake = githubApp({ ...env, GITHUB_URL: 'http://127.0.0.1:9/', GITHUB_API_URL: 'http://127.0.0.1:9/api/' })!;
    expect(fake.web).toBe('http://127.0.0.1:9');
    expect(fake.api).toBe('http://127.0.0.1:9/api');
  });

  it('signs its JWT with its key (RS256, issued a minute back, ten minutes long)', () => {
    const app = githubApp(env)!;
    const now = Date.UTC(2026, 9, 29, 10);
    const jwt = appJwt(app, now);
    const [h, p, s] = jwt.split('.');
    expect(JSON.parse(Buffer.from(h!, 'base64url').toString())).toEqual({ alg: 'RS256', typ: 'JWT' });
    const claims = JSON.parse(Buffer.from(p!, 'base64url').toString());
    expect(claims).toEqual({ iat: now / 1000 - 60, exp: now / 1000 + 540, iss: '123456' });
    const verify = createVerify('RSA-SHA256');
    verify.update(`${h}.${p}`);
    expect(verify.verify(publicKey, Buffer.from(s!, 'base64url'))).toBe(true);
  });

  it('sends the browser to GitHub with its client id, the callback and the state', () => {
    const app = githubApp(env)!;
    const url = new URL(authorizeUrl(app, 'https://store.escouade.dev/api/auth/github/callback', 'st4te'));
    expect(url.origin + url.pathname).toBe('https://github.com/login/oauth/authorize');
    expect(Object.fromEntries(url.searchParams)).toEqual({
      client_id: 'Iv23liTest',
      redirect_uri: 'https://store.escouade.dev/api/auth/github/callback',
      state: 'st4te',
    });
    expect(installUrl(app, 'st4te')).toBe('https://github.com/apps/escouade-store/installations/new?state=st4te');
  });
});
