import type { Lang } from './language';

// The store's texts, in its two languages. The legal pages are the site's: one privacy policy and one legal notice
// for the site, the app and the store (docs/STORE.md, §2.4); the authors' terms are the store's own (/terms).

const SITE = 'https://escouade.dev';

export const LINKS = {
  fr: {
    site: `${SITE}/`,
    docs: `${SITE}/docs/`,
    privacy: `${SITE}/privacy/`,
    legal: `${SITE}/legal/`,
  },
  en: {
    site: `${SITE}/en/`,
    docs: `${SITE}/en/docs/`,
    privacy: `${SITE}/en/privacy/`,
    legal: `${SITE}/en/legal/`,
  },
} as const;

const fr = {
  title: 'Escouade Store',
  description: 'Des plugins pour Escouade : harness, thèmes, notifications, règles d’autorisation. Relus, signés, installés depuis l’app.',
  header: { home: 'Accueil du store', docs: 'Documentation' },
  footer: {
    privacy: 'Confidentialité',
    legal: 'Mentions légales',
    terms: 'Conditions des auteurs',
    docs: 'Documentation',
    other: 'English',
  },
  seal: 'Vérifié par Escouade',
  home: {
    title: 'Des plugins pour Escouade.',
    more: 'Relus, signés, installés depuis l’app.',
    lead: 'Des harness, des thèmes, des notifications et des règles d’autorisation, publiés par leurs auteurs depuis un dépôt public. Ceux qui portent le sceau ont été relus version par version.',
    opening: 'Le store ouvre avec les plugins d’Escouade 2.2.',
    openingMore: 'Les premiers paraîtront ici, chacun avec son auteur, ses permissions et son sceau s’il a été relu.',
    trust: ['Licence libre exigée', 'Chaque version figée par son empreinte'],
    publishTitle: 'Tu as écrit un plugin ? Publie-le depuis son dépôt.',
    publishLead:
      'Une étiquette suffit. Le store fige le commit, lit le manifeste, la licence et le README, et te dit champ par champ ce qui manque.',
    guide: 'Lire la documentation',
    steps: [
      ['Connecte-toi avec GitHub', ' et installe l’App « Escouade Store » sur ton dépôt public : elle lit, elle n’écrit jamais.'],
      ['Choisis une étiquette.', ' Avant, vérifie en local : '],
      ['Ton plugin paraît aussitôt', ', « non vérifié ». Demande la revue : une fois relue et signée, la version porte le sceau.'],
    ],
  },
  error: { missing: 'Cette page n’existe pas.', broken: 'Le store n’a pas pu afficher cette page.', back: 'Revenir à l’accueil' },
};

type Text = typeof fr;

const en: Text = {
  title: 'Escouade Store',
  description: 'Plugins for Escouade: harnesses, themes, notifications, permission rules. Reviewed, signed, installed from the app.',
  header: { home: 'Store home', docs: 'Documentation' },
  footer: { privacy: 'Privacy', legal: 'Legal notice', terms: 'Authors’ terms', docs: 'Documentation', other: 'Français' },
  seal: 'Verified by Escouade',
  home: {
    title: 'Plugins for Escouade.',
    more: 'Reviewed, signed, installed from the app.',
    lead: 'Harnesses, themes, notifications and permission rules, published by their authors from a public repository. Those that carry the seal were reviewed version by version.',
    opening: 'The store opens with Escouade 2.2’s plugins.',
    openingMore: 'The first ones will appear here, each with its author, its permissions and its seal once reviewed.',
    trust: ['Open-source licence required', 'Each version frozen by its hash'],
    publishTitle: 'Wrote a plugin? Publish it from its repository.',
    publishLead:
      'A tag is enough. The store freezes the commit, reads the manifest, the licence and the README, and tells you field by field what is missing.',
    guide: 'Read the documentation',
    steps: [
      ['Sign in with GitHub', ' and install the “Escouade Store” App on your public repository: it reads, it never writes.'],
      ['Pick a tag.', ' Check it locally first: '],
      ['Your plugin appears at once', ', “not verified”. Ask for the review: once read and signed, the version carries the seal.'],
    ],
  },
  error: { missing: 'This page does not exist.', broken: 'The store could not show this page.', back: 'Back to the home page' },
};

export const TEXTS: Record<Lang, Text> = { fr, en };
