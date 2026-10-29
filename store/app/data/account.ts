import type { Lang } from './language';

// The texts of the author's account (docs/STORE.md, §2.2; the preview's #account), in the store's two languages.

const fr = {
  signIn: 'Se connecter avec GitHub',
  signOut: 'Se déconnecter',
  mine: 'Mes plugins',
  columns: ['Plugin', 'État', 'Dernière version', 'Installations', '30 jours'],
  guest: {
    title: 'Ton compte d’auteur',
    lead: 'Connecte-toi avec GitHub pour publier et gérer tes plugins. Le store garde ton identifiant GitHub, ton login, ton nom et ton avatar ; l’App « Escouade Store », installée sur les dépôts que tu choisis, les lit sans jamais écrire.',
    cookie: 'Un seul cookie, celui de ta session, le temps de rester connecté (30 jours). Aucune mesure d’audience.',
    off: 'La connexion n’est pas encore ouverte sur ce store.',
  },
  plugins: {
    none: 'Tu n’as encore publié aucun plugin.',
    soon: 'La soumission depuis un dépôt arrive bientôt : une étiquette suffira, le store te dira champ par champ ce qui manque.',
  },
  app: {
    title: 'L’App sur tes dépôts',
    none: 'L’App « Escouade Store » n’est installée sur aucun de tes dépôts. Installe-la sur celui de ton plugin : elle lit le code et les étiquettes, elle n’écrit jamais.',
    all: 'tous ses dépôts',
    repos: (n: number) => (n === 0 ? 'aucun dépôt' : n === 1 ? '1 dépôt' : `${n} dépôts`),
    org: 'organisation',
    suspended: 'Suspendue',
    install: 'Installer l’App sur un dépôt',
    change: 'Changer les dépôts',
  },
  installsNote:
    'Les installations ne comptent que celles dont l’utilisateur a accepté les statistiques d’Escouade : elles sont en dessous de la réalité.',
  email: {
    title: 'Avis par e-mail',
    label: 'Adresse',
    verified: 'vérifiée',
    placeholder: 'toi@exemple.dev',
    send: 'Envoyer le lien',
    change: 'Changer d’adresse',
    remove: 'Retirer l’adresse',
    pending: (e: string) => `Lien envoyé à ${e}, à suivre dans les 24 heures.`,
    about: 'Rejets, revues faites, signalements. Facultatif : sans adresse, ils restent ici.',
    off: 'L’envoi des e-mails n’est pas encore en place sur le store : le lien ne part pas pour l’instant.',
  },
  data: {
    title: 'Mes données',
    about: 'Ton identifiant GitHub, ton nom et ton avatar viennent de GitHub ; rien d’autre sans toi.',
    export: 'Exporter',
    privacy: 'Ce que le store garde, et pourquoi',
    terms: 'Les conditions des auteurs',
    delete: 'Supprimer le compte',
    confirm: (login: string) =>
      `Tape ton login, ${login}, pour confirmer. Tes sessions, ton adresse et le lien à tes installations partent aussitôt ; le journal garde la suppression un an.`,
    confirmLabel: 'Ton login',
    deleteNow: 'Supprimer définitivement',
  },
  notices: {
    'signin-cancelled': ['warn', 'Connexion annulée sur GitHub.'],
    'signin-failed': ['bad', 'La connexion n’a pas abouti. Réessaie depuis ce bouton.'],
    installed: ['ok', 'L’App est installée. Les dépôts choisis apparaissent ci-dessous.'],
    'email-sent': ['ok', 'Lien envoyé. Suis-le dans les 24 heures pour garder cette adresse.'],
    'email-not-sent': [
      'warn',
      'Le lien n’est pas parti : l’envoi des e-mails n’est pas encore en place. L’adresse n’est pas gardée tant qu’il n’est pas suivi.',
    ],
    'email-invalid': ['bad', 'Cette adresse ne semble pas valide.'],
    'email-same': ['warn', 'C’est déjà ton adresse vérifiée.'],
    'email-too-many': ['bad', 'Trop de liens demandés en une heure. Réessaie plus tard.'],
    'email-verified': ['ok', 'Adresse vérifiée : les avis y partiront.'],
    'email-expired': ['bad', 'Ce lien ne vaut plus (déjà suivi, ou plus de 24 heures). Demandes-en un autre.'],
    'email-removed': ['ok', 'Adresse retirée : les avis restent ici.'],
    'delete-unconfirmed': ['bad', 'Le login tapé ne correspond pas : rien n’a été supprimé.'],
    deleted: ['ok', 'Ton compte est supprimé. Merci d’être passé.'],
  } as Record<string, [string, string]>,
};

type Text = typeof fr;

const en: Text = {
  signIn: 'Sign in with GitHub',
  signOut: 'Sign out',
  mine: 'My plugins',
  columns: ['Plugin', 'State', 'Latest version', 'Installs', '30 days'],
  guest: {
    title: 'Your author account',
    lead: 'Sign in with GitHub to publish and manage your plugins. The store keeps your GitHub id, login, name and avatar; the “Escouade Store” App, installed on the repositories you choose, reads them and never writes.',
    cookie: 'One cookie only, your session’s, to keep you signed in (30 days). No audience measurement.',
    off: 'Signing in is not open yet on this store.',
  },
  plugins: {
    none: 'You have not published any plugin yet.',
    soon: 'Submitting from a repository comes soon: a tag will be enough, and the store will tell you field by field what is missing.',
  },
  app: {
    title: 'The App on your repositories',
    none: 'The “Escouade Store” App is not installed on any of your repositories. Install it on your plugin’s: it reads the code and the tags, it never writes.',
    all: 'all its repositories',
    repos: (n: number) => (n === 0 ? 'no repository' : n === 1 ? '1 repository' : `${n} repositories`),
    org: 'organization',
    suspended: 'Suspended',
    install: 'Install the App on a repository',
    change: 'Change the repositories',
  },
  installsNote: 'Installs only count users who accepted Escouade’s statistics: they are below the real figure.',
  email: {
    title: 'E-mail notices',
    label: 'Address',
    verified: 'verified',
    placeholder: 'you@example.dev',
    send: 'Send the link',
    change: 'Change the address',
    remove: 'Remove the address',
    pending: (e: string) => `Link sent to ${e}, to follow within 24 hours.`,
    about: 'Rejections, reviews done, reports. Optional: without an address, they stay here.',
    off: 'Sending e-mails is not set up on the store yet: the link does not leave for now.',
  },
  data: {
    title: 'My data',
    about: 'Your GitHub id, name and avatar come from GitHub; nothing else without you.',
    export: 'Export',
    privacy: 'What the store keeps, and why',
    terms: 'The authors’ terms',
    delete: 'Delete the account',
    confirm: (login: string) =>
      `Type your login, ${login}, to confirm. Your sessions, your address and the link to your installations go at once; the journal keeps the deletion one year.`,
    confirmLabel: 'Your login',
    deleteNow: 'Delete for good',
  },
  notices: {
    'signin-cancelled': ['warn', 'Sign-in cancelled on GitHub.'],
    'signin-failed': ['bad', 'Signing in did not go through. Try again from this button.'],
    installed: ['ok', 'The App is installed. The repositories you chose show below.'],
    'email-sent': ['ok', 'Link sent. Follow it within 24 hours to keep this address.'],
    'email-not-sent': ['warn', 'The link did not leave: sending e-mails is not set up yet. The address is not kept until it is followed.'],
    'email-invalid': ['bad', 'This address does not look valid.'],
    'email-same': ['warn', 'This is already your verified address.'],
    'email-too-many': ['bad', 'Too many links asked for in an hour. Try again later.'],
    'email-verified': ['ok', 'Address verified: the notices will go there.'],
    'email-expired': ['bad', 'This link is no longer valid (already followed, or older than 24 hours). Ask for another one.'],
    'email-removed': ['ok', 'Address removed: the notices stay here.'],
    'delete-unconfirmed': ['bad', 'The login typed does not match: nothing was deleted.'],
    deleted: ['ok', 'Your account is deleted. Thanks for coming by.'],
  },
};

export const ACCOUNT_TEXTS: Record<Lang, Text> = { fr, en };

/** The hue of an author's initials (the preview's avatars), one of twelve, from their GitHub id: no image from GitHub. */
export const avatarHue = (githubId: number) => (Math.abs(githubId) % 12) * 30;

/** The initials of a login, as the preview draws them: the first letters of its first two words (lea-martin: LM). */
export function initials(login: string): string {
  const words = login.split(/[^a-z0-9]+/i).filter(Boolean);
  const two = words.length > 1 ? words[0]![0]! + words[1]![0]! : (words[0] ?? '').slice(0, 2);
  return (two || '?').toUpperCase();
}
