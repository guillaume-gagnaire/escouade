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

/** The rejection of a version, check by check, in both languages (the account has no language of its own). */
export function rejectionMail(to: string, what: { plugin: string; tag: string; page: string; fr: string; en: string }): Mail {
  return {
    to,
    subject: `Escouade Store : ${what.plugin} ${what.tag} refusée par les contrôles / rejected by the checks`,
    text: `Bonjour,\n\nLes contrôles de ${what.plugin} ${what.tag} ont échoué ; rien n'est publié.\n\n${what.fr}\n\nLa page de la soumission : ${what.page}\n\n---\n\nHello,\n\nThe checks of ${what.plugin} ${what.tag} failed; nothing is published.\n\n${what.en}\n\nThe submission's page: ${what.page}\n\nEscouade Store — https://store.escouade.dev\n`,
  };
}

/** A tag moved on the repository after its version was published: nothing changes on the store, the author is told. */
export function tagMovedMail(to: string, what: { plugin: string; tag: string; was: string; now: string }): Mail {
  return {
    to,
    subject: `Escouade Store : l'étiquette ${what.tag} de ${what.plugin} a bougé / the tag moved`,
    text: `Bonjour,\n\nL'étiquette ${what.tag} de ${what.plugin} nomme maintenant le commit ${what.now.slice(0, 7)}, et non plus ${what.was.slice(0, 7)}, celui que le store a figé à la publication. La version publiée ne change pas : elle reste celle de ${what.was.slice(0, 7)}, et sa page dit « étiquette déplacée sur le dépôt ». Si tu ne l'as pas déplacée toi-même, ton dépôt est peut-être entre d'autres mains. Pour publier ce code, pousse une nouvelle étiquette.\n\n---\n\nHello,\n\nThe tag ${what.tag} of ${what.plugin} now names the commit ${what.now.slice(0, 7)}, no longer ${what.was.slice(0, 7)}, the one the store froze at publication. The published version does not change: it stays that of ${what.was.slice(0, 7)}, and its page says "tag moved on the repository". If you did not move it yourself, your repository may be in other hands. To publish this code, push a new tag.\n\nEscouade Store — https://store.escouade.dev\n`,
  };
}
