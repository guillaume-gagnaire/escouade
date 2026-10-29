import { and, eq, gte, sql } from 'drizzle-orm';
import { reports } from '../db/schema';
import { cleanEmail, note } from './accounts';
import type { Db } from './db';
import type { Mail } from './mail';

// « Signaler » (docs/STORE.md §4.6): open to anyone, with an e-mail address and without a GitHub account, as the
// DSA's notice and action asks (decided by the chief of the squad on 2026-10-29). A report goes to the review queue
// (STORE-8) and its sender gets an acknowledgement; the author sees that one is open, never who sent it. What limits
// the abuse without an account: the address, the statement of good faith, and a cap per address and per hour.

export const REASONS = ['malicious', 'misleading', 'permissions', 'license', 'illegal', 'other'] as const;
export type Reason = (typeof REASONS)[number];
export const DETAILS_MIN = 20;
export const DETAILS_MAX = 4000;
/** Reports a same address may send in an hour, and the store in all in an hour. */
export const PER_ADDRESS_HOUR = 5;
export const PER_HOUR = 200;

export type Filed = { id: number; email: string } | { error: 'reason' | 'details' | 'email' | 'faith' | 'too-many' };

/** Files a report from its form; `exists` says whether the plugin has a page (Escouade's own included). */
export async function fileReport(
  db: Db,
  form: Record<string, unknown>,
  ctx: { pluginId: string; version: string | null; lang: 'fr' | 'en' },
  now = new Date(),
): Promise<Filed> {
  const reason = String(form.reason ?? '');
  if (!(REASONS as readonly string[]).includes(reason)) return { error: 'reason' };
  const details = String(form.details ?? '')
    .replace(/\r\n?/g, '\n')
    .trim();
  if (details.length < DETAILS_MIN || details.length > DETAILS_MAX) return { error: 'details' };
  const email = cleanEmail(form.email);
  if (!email) return { error: 'email' };
  if (form.faith !== 'on' && form.faith !== 'yes') return { error: 'faith' };
  const hour = new Date(now.getTime() - 3600_000);
  const [mine] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(reports)
    .where(and(eq(reports.email, email), gte(reports.createdAt, hour)));
  const [everyone] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(reports)
    .where(gte(reports.createdAt, hour));
  if ((mine?.n ?? 0) >= PER_ADDRESS_HOUR || (everyone?.n ?? 0) >= PER_HOUR) return { error: 'too-many' };
  const [row] = await db
    .insert(reports)
    .values({ pluginId: ctx.pluginId, version: ctx.version, reason, details, email, lang: ctx.lang, createdAt: now })
    .returning({ id: reports.id });
  await note(db, 'report.filed', { accountId: null }, { report: row!.id, plugin: ctx.pluginId, version: ctx.version, reason });
  return { id: row!.id, email };
}

const REASON_WORDS: Record<Reason, { fr: string; en: string }> = {
  malicious: { fr: 'Malveillant', en: 'Malicious' },
  misleading: { fr: 'Ne fait pas ce qu’il dit', en: 'Does not do what it says' },
  permissions: { fr: 'Permissions trompeuses', en: 'Misleading permissions' },
  license: { fr: 'Licence', en: 'License' },
  illegal: { fr: 'Contenu illicite', en: 'Illegal content' },
  other: { fr: 'Autre', en: 'Other' },
};

/** The acknowledgement of a report: its number, what happens next, nothing of the plugin's author. */
export function reportMail(lang: 'fr' | 'en', to: string, what: { id: number; plugin: string; reason: Reason }): Mail {
  return lang === 'fr'
    ? {
        to,
        subject: `Escouade Store : signalement n° ${what.id} reçu`,
        text: `Bonjour,\n\nTon signalement de ${what.plugin} (${REASON_WORDS[what.reason].fr}) est reçu sous le n° ${what.id}. Il va dans la file de revue d’Escouade : une personne le lit, et te répond à cette adresse quand une décision est prise. L’auteur du plugin sait qu’un signalement est ouvert, jamais qui l’a fait.\n\nEscouade Store — https://store.escouade.dev\n`,
      }
    : {
        to,
        subject: `Escouade Store: report no. ${what.id} received`,
        text: `Hello,\n\nYour report of ${what.plugin} (${REASON_WORDS[what.reason].en}) is received as no. ${what.id}. It goes to Escouade's review queue: a person reads it, and answers you at this address once a decision is made. The plugin's author knows that a report is open, never who sent it.\n\nEscouade Store — https://store.escouade.dev\n`,
      };
}

/** The answer to a report once the review closed it (`upheld`: acted on; `dismissed`: nothing to do), with its words. */
export function reportClosedMail(
  lang: 'fr' | 'en',
  to: string,
  what: { id: number; plugin: string; upheld: boolean; answer: string },
): Mail {
  const answer = what.answer.trim() ? `\n\n${what.answer.trim()}` : '';
  return lang === 'fr'
    ? {
        to,
        subject: `Escouade Store : signalement n° ${what.id} traité`,
        text: `Bonjour,\n\nTon signalement n° ${what.id} de ${what.plugin} est lu. ${what.upheld ? 'Il est retenu : Escouade agit sur le plugin.' : 'Il n’appelle pas d’action sur le plugin.'}${answer}\n\nMerci de l’avoir fait.\n\nEscouade Store — https://store.escouade.dev\n`,
      }
    : {
        to,
        subject: `Escouade Store: report no. ${what.id} handled`,
        text: `Hello,\n\nYour report no. ${what.id} of ${what.plugin} was read. ${what.upheld ? 'It is upheld: Escouade acts on the plugin.' : 'It calls for no action on the plugin.'}${answer}\n\nThank you for sending it.\n\nEscouade Store — https://store.escouade.dev\n`,
      };
}
