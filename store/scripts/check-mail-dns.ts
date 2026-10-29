// Reads the SPF, DKIM and DMARC records of the store's sender domain and says what is missing (server/utils/mail-dns.ts):
//
//   npx tsx store/scripts/check-mail-dns.ts [--domain escouade.dev] [--dkim <selector>]
//
// The selector is Scaleway's project id, shown with the DKIM record in the domain's page of Transactional Email.
// Exit 1 while something is missing.

import { Resolver } from 'node:dns/promises';
import { judgeMailDns } from '../server/utils/mail-dns';

const arg = (name: string) => {
  const i = process.argv.indexOf(`--${name}`);
  return i > 0 ? process.argv[i + 1] : undefined;
};
const domain = arg('domain') ?? 'escouade.dev';
const selector = arg('dkim');
const resolver = new Resolver();
resolver.setServers(['1.1.1.1', '8.8.8.8']);
const txt = (name: string) =>
  resolver.resolveTxt(name).then(
    (r) => r.map((parts) => parts.join('')),
    () => [] as string[],
  );

const problems = judgeMailDns({
  root: await txt(domain),
  dkim: selector ? await txt(`${selector}._domainkey.${domain}`) : null,
  dmarc: await txt(`_dmarc.${domain}`),
});
for (const p of problems) console.log(`✗ ${p}`);
if (!problems.length) console.log(`✓ SPF, DKIM and DMARC hold for ${domain}`);
process.exit(problems.length ? 1 : 0);
