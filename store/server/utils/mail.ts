import nodemailer from 'nodemailer';
import type { Env } from './config';

// The store's e-mails, through SMTP_URL (Scaleway Transactional Email, from MAIL_FROM). Without them the address is
// still offered, the link is not sent, and the page says so (docs/STORE.md, §2.2).

export interface Mail {
  to: string;
  subject: string;
  text: string;
}

export const mailOn = (env: Env) => !!(env.SMTP_URL ?? '').trim() && !!(env.MAIL_FROM ?? '').trim();

/** Sends `mail`; false if the mail is off or the server refused it (the reason goes to the log, not to the page). */
export async function sendMail(env: Env, mail: Mail): Promise<boolean> {
  if (!mailOn(env)) return false;
  try {
    const transport = nodemailer.createTransport(env.SMTP_URL!.trim());
    await transport.sendMail({ from: env.MAIL_FROM!.trim(), ...mail });
    return true;
  } catch (e) {
    console.error('[store] mail not sent:', e instanceof Error ? e.message : e);
    return false;
  }
}

/** The e-mail that checks an address: the link, its 24 hours, and what to do if it was not asked for. */
export function emailCheckMail(lang: 'fr' | 'en', to: string, link: string): Mail {
  return lang === 'fr'
    ? {
        to,
        subject: 'Vérifie ton adresse sur Escouade Store',
        text: `Bonjour,\n\nPour recevoir les avis d’Escouade Store à cette adresse (rejets, revues faites, signalements), suis ce lien dans les 24 heures :\n\n${link}\n\nSi tu ne l’as pas demandé, ignore ce message : l’adresse ne sera pas gardée.\n\nEscouade Store — https://store.escouade.dev\n`,
      }
    : {
        to,
        subject: 'Check your address on Escouade Store',
        text: `Hello,\n\nTo get Escouade Store's notices at this address (rejections, reviews done, reports), follow this link within 24 hours:\n\n${link}\n\nIf you did not ask for it, ignore this message: the address will not be kept.\n\nEscouade Store — https://store.escouade.dev\n`,
      };
}
