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

/** What the author's space tells its author (docs/STORE.md, §2.2 and §7): every change of a plugin, by e-mail too. */
export type AuthorEvent =
  | 'ready'
  | 'auto-published'
  | 'published'
  | 'yanked'
  | 'unlisted'
  | 'relisted'
  | 'transfer-offered'
  | 'transfer-asked'
  | 'transfer-refused'
  | 'transferred-in'
  | 'transferred-out'
  | 'deleted'
  | 'restored';

const dateFr = (d: Date) => d.toLocaleDateString('fr-FR', { day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' });
const dateEn = (d: Date) => d.toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' });

/** The e-mail of an author's event, in both languages; an action they did not do tells them their account may be in other hands. */
export function authorMail(
  to: string,
  event: AuthorEvent,
  w: { plugin: string; version?: string; other?: string; until?: Date; page: string },
): Mail {
  const v = w.version ? `v${w.version}` : '';
  const u = w.until ?? new Date(0);
  const o = w.other ?? '';
  const texts: Record<AuthorEvent, [string, string, string, string]> = {
    ready: [
      `${w.plugin} ${v} attend ta publication`,
      `Une nouvelle étiquette de ${w.plugin} a passé les contrôles : la ${v} attend ton clic pour paraître, non vérifiée. Si tu n'as pas poussé cette étiquette, ne la publie pas.`,
      `${w.plugin} ${v} waits for you to publish it`,
      `A new tag of ${w.plugin} passed the checks: ${v} waits for your click to appear, unverified. If you did not push this tag, do not publish it.`,
    ],
    'auto-published': [
      `${w.plugin} ${v} publiée seule`,
      `La ${v} de ${w.plugin}, trouvée sur une nouvelle étiquette, a passé les contrôles et paraît, non vérifiée, comme tu l'as demandé (« Publier les nouvelles étiquettes seul »).`,
      `${w.plugin} ${v} published on its own`,
      `${v} of ${w.plugin}, found on a new tag, passed the checks and is out, unverified, as you asked ("Publish new tags on their own").`,
    ],
    published: [
      `${w.plugin} ${v} publiée`,
      `La ${v} de ${w.plugin} est publiée, non vérifiée.`,
      `${w.plugin} ${v} published`,
      `${v} of ${w.plugin} is published, unverified.`,
    ],
    yanked: [
      `${w.plugin} ${v} retirée`,
      `La ${v} de ${w.plugin} n'est plus proposée à l'installation ni comme mise à jour. Ceux qui l'ont la gardent, avec la mention « retirée par l'auteur ».`,
      `${w.plugin} ${v} taken back`,
      `${v} of ${w.plugin} is no longer offered to install nor as an update. Those who have it keep it, marked "taken back by its author".`,
    ],
    unlisted: [
      `${w.plugin} retiré de la liste`,
      `${w.plugin} n'apparaît plus dans la recherche ni dans le catalogue ; sa page reste lisible, les installations gardent leurs versions, sans mises à jour.`,
      `${w.plugin} taken off the list`,
      `${w.plugin} no longer shows in the search nor in the catalog; its page stays readable, installations keep their versions, without updates.`,
    ],
    relisted: [
      `${w.plugin} remis dans la liste`,
      `${w.plugin} apparaît de nouveau dans la recherche et le catalogue.`,
      `${w.plugin} back on the list`,
      `${w.plugin} shows again in the search and the catalog.`,
    ],
    'transfer-offered': [
      `${o} te transfère ${w.plugin}`,
      `${o} te propose ${w.plugin} sur le store. Accepte ou refuse depuis ton compte ; accepté, ses publications sont gelées 7 jours.`,
      `${o} offers you ${w.plugin}`,
      `${o} offers you ${w.plugin} on the store. Accept or refuse from your account; once accepted, its publications freeze for 7 days.`,
    ],
    'transfer-asked': [
      `Transfert de ${w.plugin} à ${o} demandé`,
      `${w.plugin} est proposé à ${o} ; il reste à toi tant que ${o} ne l'a pas accepté, et tu peux annuler d'ici là.`,
      `Transfer of ${w.plugin} to ${o} asked`,
      `${w.plugin} is offered to ${o}; it stays yours until ${o} accepts it, and you may cancel until then.`,
    ],
    'transfer-refused': [
      `${o} a refusé ${w.plugin}`,
      `${o} a refusé le transfert de ${w.plugin} : il reste à toi.`,
      `${o} refused ${w.plugin}`,
      `${o} refused the transfer of ${w.plugin}: it stays yours.`,
    ],
    'transferred-in': [
      `${w.plugin} est à toi`,
      `${w.plugin} est maintenant à ton compte${o ? ` (de ${o})` : ''}. Ses publications sont gelées jusqu'au ${dateFr(u)}, le temps d'un signalement ; sa page dit le transfert et sa date.`,
      `${w.plugin} is yours`,
      `${w.plugin} is now your account's${o ? ` (from ${o})` : ''}. Its publications are frozen until ${dateEn(u)}, the time for a report; its page tells the transfer and its date.`,
    ],
    'transferred-out': [
      `${w.plugin} transféré à ${o}`,
      `${w.plugin} est maintenant au compte de ${o}. Ses publications sont gelées jusqu'au ${dateFr(u)}.`,
      `${w.plugin} transferred to ${o}`,
      `${w.plugin} now belongs to ${o}'s account. Its publications are frozen until ${dateEn(u)}.`,
    ],
    deleted: [
      `${w.plugin} supprimé`,
      `${w.plugin} a quitté le catalogue. Sa page et ses archives partent le ${dateFr(u)} ; d'ici là, tu peux annuler depuis ton compte. Son identifiant reste à ton compte.`,
      `${w.plugin} deleted`,
      `${w.plugin} left the catalog. Its page and archives go on ${dateEn(u)}; until then, you may undo it from your account. Its id stays your account's.`,
    ],
    restored: [
      `${w.plugin} rétabli`,
      `La suppression de ${w.plugin} est annulée.`,
      `${w.plugin} restored`,
      `The deletion of ${w.plugin} is undone.`,
    ],
  };
  const [sFr, fr, sEn, en] = texts[event];
  return {
    to,
    subject: `Escouade Store : ${sFr} / ${sEn}`,
    text: `Bonjour,\n\n${fr}\n\nSi ce n'était pas toi, ton compte GitHub est peut-être entre d'autres mains : change son mot de passe, puis vérifie ton compte sur le store.\n\nTon compte : ${w.page}\n\n---\n\nHello,\n\n${en}\n\nIf it was not you, your GitHub account may be in other hands: change its password, then check your account on the store.\n\nYour account: ${w.page}\n\nEscouade Store — https://store.escouade.dev\n`,
  };
}

/** What the review tells an author (STORE-8): its answer on a version, the end of a manual review, a withdrawal. */
export type ReviewEvent = 'verified' | 'changes' | 'refused' | 'released' | 'held-refused' | 'disabled' | 'unverified';

/** The e-mail of the review's answer, in both languages, with what the review said (`message`) when it said something. */
export function reviewMail(
  to: string,
  event: ReviewEvent,
  w: { plugin: string; version: string; message?: { fr: string; en: string } | string; page: string },
): Mail {
  const v = `v${w.version}`;
  const texts: Record<ReviewEvent, [string, string, string, string]> = {
    verified: [
      `${w.plugin} ${v} vérifiée par Escouade`,
      `La ${v} de ${w.plugin} est relue et signée : elle porte le sceau « Vérifié par Escouade » sur le store et dans l'app. Une nouvelle version repassera en revue, sur son diff.`,
      `${w.plugin} ${v} verified by Escouade`,
      `${v} of ${w.plugin} is reviewed and signed: it bears the seal "Verified by Escouade" on the store and in the app. A new version goes through the review again, on its diff.`,
    ],
    changes: [
      `${w.plugin} ${v} : changements demandés`,
      `La revue de la ${v} de ${w.plugin} demande des changements avant de la vérifier. Elle reste publiée, non vérifiée. Une nouvelle version qui en tient compte repassera en revue.`,
      `${w.plugin} ${v}: changes asked`,
      `The review of ${v} of ${w.plugin} asks for changes before verifying it. It stays published, unverified. A new version that takes them into account goes through the review again.`,
    ],
    refused: [
      `${w.plugin} ${v} : revue refusée`,
      `La revue de la ${v} de ${w.plugin} ne la vérifie pas. Elle reste publiée, non vérifiée.`,
      `${w.plugin} ${v}: review refused`,
      `The review of ${v} of ${w.plugin} does not verify it. It stays published, unverified.`,
    ],
    released: [
      `${w.plugin} ${v} peut paraître`,
      `La revue manuelle de la ${v} de ${w.plugin} (un nom proche d'un autre plugin) est faite : elle attend ton clic pour paraître, non vérifiée.`,
      `${w.plugin} ${v} may be published`,
      `The manual review of ${v} of ${w.plugin} (a name close to another plugin's) is done: it waits for your click to appear, unverified.`,
    ],
    'held-refused': [
      `${w.plugin} ${v} refusée`,
      `La revue manuelle de la ${v} de ${w.plugin} (un nom proche d'un autre plugin) refuse sa publication.`,
      `${w.plugin} ${v} refused`,
      `The manual review of ${v} of ${w.plugin} (a name close to another plugin's) refuses its publication.`,
    ],
    disabled: [
      `${w.plugin} ${v} retirée par Escouade`,
      `La ${v} de ${w.plugin} est désactivée par la liste de révocation d'Escouade : elle quitte le catalogue, et l'app l'arrête chez ceux qui l'ont.`,
      `${w.plugin} ${v} withdrawn by Escouade`,
      `${v} of ${w.plugin} is disabled by Escouade's revocation list: it leaves the catalog, and the app stops it for those who have it.`,
    ],
    unverified: [
      `${w.plugin} ${v} : sceau retiré`,
      `Le sceau « Vérifié par Escouade » de la ${v} de ${w.plugin} est retiré par la liste de révocation. Elle reste publiée, non vérifiée.`,
      `${w.plugin} ${v}: seal withdrawn`,
      `The seal "Verified by Escouade" of ${v} of ${w.plugin} is withdrawn by the revocation list. It stays published, unverified.`,
    ],
  };
  const [sFr, fr, sEn, en] = texts[event];
  const said = typeof w.message === 'string' ? { fr: w.message, en: w.message } : w.message;
  const quote = (s: string) => (s.trim() ? `\n\n${s.trim()}` : '');
  return {
    to,
    subject: `Escouade Store : ${sFr} / ${sEn}`,
    text: `Bonjour,\n\n${fr}${quote(said?.fr ?? '')}\n\nTon compte : ${w.page}\n\n---\n\nHello,\n\n${en}${quote(said?.en ?? '')}\n\nYour account: ${w.page}\n\nEscouade Store — https://store.escouade.dev\n`,
  };
}
