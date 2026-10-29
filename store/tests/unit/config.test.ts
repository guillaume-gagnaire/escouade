import { describe, expect, it } from 'vitest';
import { admins, archiveDir, missingFeatures, missingLines } from '../../server/utils/config';

describe('the variables the store reads', () => {
  it('says each feature turned off and the variables it lacks, never a value', () => {
    const lines = missingLines({ DATABASE_URL: 'hunter2', GITHUB_APP_ID: '12', GITHUB_APP_CLIENT_SECRET: 's3cr3t' });
    expect(lines).toHaveLength(4);
    expect(lines.find((l) => l.includes('github off'))).toContain(
      'missing GITHUB_APP_CLIENT_ID, GITHUB_APP_PRIVATE_KEY, GITHUB_WEBHOOK_SECRET',
    );
    expect(lines.join('\n')).not.toMatch(/hunter2|s3cr3t/);
    expect(lines.some((l) => l.includes('database'))).toBe(false);
  });

  it('starts with the variables Coolify has on 2026-10-29: GitHub and the mail off, the rest on', () => {
    const pem = '-----BEGIN RSA ' + 'PRIVATE KEY-----\nMIIE…\n-----END RSA ' + 'PRIVATE KEY-----\n';
    const lines = missingLines({
      DATABASE_URL: 'x',
      NUXT_SESSION_SECRET: 'x',
      GITHUB_APP_ID: '1',
      GITHUB_APP_CLIENT_ID: 'Iv1.x',
      GITHUB_APP_PRIVATE_KEY: pem,
      GITHUB_WEBHOOK_SECRET: 'x',
      STORE_ADMINS: '1',
      ARCHIVE_DIR: '/data/archives',
      NUXT_PUBLIC_SITE_URL: 'https://store.escouade.dev',
    });
    expect(lines).toEqual([
      '[store] github off (missing GITHUB_APP_CLIENT_SECRET): signing in with GitHub, submissions and webhooks are off',
      '[store] mail off (missing SMTP_URL, MAIL_FROM): no e-mail is sent',
    ]);
  });

  it('counts a blank variable as missing', () => {
    expect(missingFeatures({ DATABASE_URL: '   ' }).map((m) => m.feature.name)).toContain('database');
  });

  it('has nothing to say when everything is set', () => {
    const all = Object.fromEntries(
      [
        'DATABASE_URL',
        'NUXT_SESSION_SECRET',
        'GITHUB_APP_ID',
        'GITHUB_APP_CLIENT_ID',
        'GITHUB_APP_CLIENT_SECRET',
        'GITHUB_APP_PRIVATE_KEY',
        'GITHUB_WEBHOOK_SECRET',
        'SMTP_URL',
        'MAIL_FROM',
        'STORE_ADMINS',
      ].map((k) => [k, 'x']),
    );
    expect(missingLines(all)).toEqual([]);
  });

  it('keeps the archives in the volume when it is named, in the checkout otherwise', () => {
    expect(archiveDir({ ARCHIVE_DIR: '/data/archives' })).toBe('/data/archives');
    expect(archiveDir({})).toBe('./.data/archives');
  });

  it('reads the administrators as GitHub numeric ids', () => {
    expect(admins({ STORE_ADMINS: '1234, 5678 nope -3' })).toEqual([1234, 5678]);
    expect(admins({})).toEqual([]);
  });
});
