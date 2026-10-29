import { describe, expect, it } from 'vitest';
import { judgeMailDns } from '../../server/utils/mail-dns';

// SPF, DKIM and DMARC of store@escouade.dev (docs/STORE.md §8.1), as scripts/check-mail-dns.ts judges them.
const KEY = `v=DKIM1; k=rsa; p=${'A'.repeat(300)}`;

describe('the records of the sender domain', () => {
  it('hold with Scaleway in SPF, a DKIM key and a DMARC record', () => {
    expect(
      judgeMailDns({
        root: ['google-site-verification=x', 'v=spf1 include:_mailcust.gandi.net include:_spf.tem.scaleway.com ~all'],
        dkim: [KEY],
        dmarc: ['v=DMARC1; p=none'],
      }),
    ).toEqual([]);
  });

  it('say what to set, as escouade.dev stands on 2026-10-29', () => {
    const problems = judgeMailDns({ root: ['v=spf1 include:_mailcust.gandi.net ?all'], dkim: [], dmarc: [] });
    expect(problems).toEqual([
      'SPF: add include:_spf.tem.scaleway.com before its last term, in "v=spf1 include:_mailcust.gandi.net ?all"',
      'DKIM: no public key at <selector>._domainkey; copy the TXT record Scaleway gives for the domain',
      expect.stringContaining('DMARC: no record at _dmarc'),
    ]);
  });

  it('refuse two SPF records, which receivers ignore together', () => {
    expect(
      judgeMailDns({
        root: ['v=spf1 include:_mailcust.gandi.net ?all', 'v=spf1 include:_spf.tem.scaleway.com ~all'],
        dkim: [KEY],
        dmarc: ['v=DMARC1; p=none'],
      }),
    ).toEqual([expect.stringContaining('more than one')]);
    expect(judgeMailDns({ root: [], dkim: null, dmarc: ['v=DMARC1; p=none'] })).toEqual([
      expect.stringContaining('SPF: no record'),
      expect.stringContaining('DKIM: give the selector'),
    ]);
  });
});
