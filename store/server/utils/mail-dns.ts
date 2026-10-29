// SPF, DKIM and DMARC for store@escouade.dev (docs/STORE.md §8.1): the store's notices leave by Scaleway's
// Transactional Email; without these records, the mailboxes file them as spam or refuse them. The records are set at
// Gandi by the user; scripts/check-mail-dns.ts reads them and says what is missing. Plain TypeScript, no import.

/** What Scaleway's Transactional Email asks SPF to include. */
export const SCALEWAY_SPF = 'include:_spf.tem.scaleway.com';

export interface MailDns {
  /** The TXT records of the domain. */
  root: string[];
  /** The TXT records of `<selector>._domainkey.<domain>` (the selector is Scaleway's project id), or null if not asked. */
  dkim: string[] | null;
  /** The TXT records of `_dmarc.<domain>`. */
  dmarc: string[];
}

/** What is wrong with the records, one line each, with what to set; empty when all holds. */
export function judgeMailDns(dns: MailDns): string[] {
  const problems: string[] = [];
  const spf = dns.root.filter((t) => /^v=spf1(\s|$)/i.test(t.trim()));
  if (spf.length === 0) problems.push(`SPF: no record; set "v=spf1 include:_mailcust.gandi.net ${SCALEWAY_SPF} ~all"`);
  else if (spf.length > 1) problems.push('SPF: more than one v=spf1 record (receivers then ignore them all); merge them into one');
  else if (!spf[0]!.split(/\s+/).includes(SCALEWAY_SPF)) problems.push(`SPF: add ${SCALEWAY_SPF} before its last term, in "${spf[0]}"`);
  if (dns.dkim === null) problems.push('DKIM: give the selector (--dkim <project id>), as Scaleway shows it for the domain');
  else if (!dns.dkim.some((t) => /(^|;)\s*p=[A-Za-z0-9+/=]{100,}/.test(t.replace(/\s/g, ' '))))
    problems.push('DKIM: no public key at <selector>._domainkey; copy the TXT record Scaleway gives for the domain');
  if (!dns.dmarc.some((t) => /^v=DMARC1;/i.test(t.trim())))
    problems.push('DMARC: no record at _dmarc; set "v=DMARC1; p=none; rua=mailto:guillaume@gagnaire.dev" to start');
  return problems;
}
