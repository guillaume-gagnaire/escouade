import { describe, expect, it } from 'vitest';
import { readInstall } from '../../server/utils/installs';
import { clientAddress, Limiter, ruleFor, RULES } from '../../server/utils/limits';

describe('the rate limits', () => {
  it('let a window’s requests through, then say how long to wait', () => {
    const l = new Limiter();
    const rule = { name: 't', max: 3, windowMs: 60_000 };
    const t = 1_000_000;
    for (let i = 0; i < 3; i++) expect(l.take(rule, '203.0.113.7', t + i).ok).toBe(true);
    expect(l.take(rule, '203.0.113.7', t + 10_000)).toEqual({ ok: false, retryAfter: 50 });
    // Another address has its own window; the next window starts afresh.
    expect(l.take(rule, '203.0.113.8', t + 10_000).ok).toBe(true);
    expect(l.take(rule, '203.0.113.7', t + 60_000).ok).toBe(true);
  });

  it('never keep an address: only a keyed hash, which another process cannot match', () => {
    const a = new Limiter();
    const b = new Limiter();
    expect(a.who('203.0.113.7')).not.toContain('203');
    expect(a.who('203.0.113.7')).toBe(a.who('203.0.113.7'));
    expect(a.who('203.0.113.7')).not.toBe(b.who('203.0.113.7'));
  });

  it('count a version once a day from one address', () => {
    const l = new Limiter();
    const d = new Date('2026-10-29T10:00:00Z');
    expect(l.firstToday('198.51.100.1', 'dev.x.a@1.0.0:install', d)).toBe(true);
    expect(l.firstToday('198.51.100.1', 'dev.x.a@1.0.0:install', new Date('2026-10-29T23:59:00Z'))).toBe(false);
    expect(l.firstToday('198.51.100.1', 'dev.x.a@1.0.0:update', d)).toBe(true);
    expect(l.firstToday('198.51.100.2', 'dev.x.a@1.0.0:install', d)).toBe(true);
    expect(l.firstToday('198.51.100.1', 'dev.x.a@1.0.0:install', new Date('2026-10-30T00:01:00Z'))).toBe(true);
  });

  it('put each request under its rule', () => {
    expect(ruleFor('GET', '/api/v1/catalog.json')).toBe(RULES.api);
    expect(ruleFor('GET', '/api/v1/revocations.json')).toBe(RULES.api);
    expect(ruleFor('GET', '/api/v1/plugins/dev.x.a/1.0.0/version.json')).toBe(RULES.api);
    expect(ruleFor('GET', '/api/v1/plugins/dev.x.a/1.0.0/archive.tar.gz')).toBe(RULES.downloads);
    expect(ruleFor('HEAD', '/api/v1/plugins/dev.x.a/1.0.0/binaries/linux-x64')).toBe(RULES.downloads);
    expect(ruleFor('POST', '/api/v1/installs')).toBe(RULES.installs);
    expect(ruleFor('POST', '/api/submissions')).toBe(RULES.submissions);
    expect(ruleFor('POST', '/api/submissions/12/retry')).toBe(RULES.submissions);
    expect(ruleFor('POST', '/api/submissions/12/publish')).toBe(RULES.submissions);
    expect(ruleFor('GET', '/api/submissions/12')).toBeNull();
    expect(ruleFor('GET', '/')).toBeNull();
    expect(ruleFor('GET', '/api/health')).toBeNull();
  });

  it('take the address the proxy saw, not one a client wrote before it', () => {
    expect(clientAddress('6.6.6.6, 203.0.113.7', '10.0.1.5')).toBe('203.0.113.7');
    expect(clientAddress(undefined, '10.0.1.5')).toBe('10.0.1.5');
    expect(clientAddress(' , ', undefined)).toBe('unknown');
  });
});

describe('the body of an installation', () => {
  it('is the three keys, and nothing else', () => {
    expect(readInstall({ id: 'dev.x.ntfy', version: '1.2.0', event: 'install' })).toEqual({
      id: 'dev.x.ntfy',
      version: '1.2.0',
      event: 'install',
    });
    expect(readInstall({ id: 'dev.x.ntfy', version: '1.2.0', event: 'update' })?.event).toBe('update');
    expect(readInstall({ id: 'dev.x.ntfy', version: '1.2.0', event: 'install', installId: 'abc' })).toBeNull();
    expect(readInstall({ id: 'dev.x.ntfy', version: '1.2.0', event: 'uninstall' })).toBeNull();
    expect(readInstall({ id: '../etc', version: '1.2.0', event: 'install' })).toBeNull();
    expect(readInstall({ id: 'dev.x.ntfy', version: 'x'.repeat(65), event: 'install' })).toBeNull();
    expect(readInstall(null)).toBeNull();
    expect(readInstall([])).toBeNull();
  });
});
