import type { Lang } from './language';

// The authors' terms of the store (docs/STORE.md, §2.4): accepted at the first submission, versioned. A change of
// substance makes a new version, which the authors accept again at their next submission.

export const TERMS_VERSION = 1;
export const TERMS_DATE = '2026-10-29';

export interface Terms {
  title: string;
  version: string;
  lead: string;
  sections: { title: string; items: string[] }[];
}

const fr: Terms = {
  title: 'Les conditions des auteurs',
  version: `Version ${TERMS_VERSION}, du 29 octobre 2026`,
  lead: 'Ces conditions valent pour qui publie un plugin sur store.escouade.dev, édité par GAGNAIRE DEVELOPPEMENT. Elles sont acceptées à la première soumission ; une nouvelle version se réaccepte à la soumission suivante.',
  sections: [
    {
      title: 'Ce que tu publies',
      items: [
        'Tu as les droits sur ce que tu publies : le code, les textes, les images et les noms.',
        'Ton plugin est sous une licence libre reconnue par l’OSI ou la FSF, dite par son identifiant SPDX dans le manifeste et dans le fichier de licence du dépôt.',
        'Le store ne vend rien : pas de vente ni de paiement dans un plugin par Escouade.',
        'Une version publiée ne se modifie pas et son numéro ne se réutilise pas. Tu peux la retirer, retirer le plugin de la liste, le transférer ou le supprimer depuis ton compte.',
      ],
    },
    {
      title: 'Ce qui est interdit',
      items: [
        'Le code malveillant, et tout ce qui contourne les permissions d’Escouade.',
        'La collecte de données que les permissions du plugin ne déclarent pas.',
        'Le minage, et tout usage des machines des utilisateurs à d’autres fins que ce que le plugin annonce.',
        'Le contenu illicite, et l’usurpation d’un nom, d’une marque ou d’un auteur.',
      ],
    },
    {
      title: 'Ce qu’Escouade peut faire',
      items: [
        'Refuser une soumission, retirer un plugin de la liste, retirer une version ou révoquer son badge, à tout moment. Escouade dit pourquoi, sauf quand le dire mettrait des utilisateurs en danger.',
        'Garder un plugin supprimé 30 jours, et son identifiant réservé à ton compte ensuite : personne ne publie autre chose sous le même nom.',
      ],
    },
    {
      title: 'Le badge « Vérifié par Escouade »',
      items: [
        '« Vérifié par Escouade » veut dire qu’une version précise, celle de son empreinte, a été relue et signée. Ce n’est pas une garantie : Escouade ne répond pas d’un plugin tiers, vérifié ou non.',
        'Une nouvelle version n’a pas le badge tant qu’elle n’a pas été relue à son tour.',
      ],
    },
    {
      title: 'Ton compte et tes données',
      items: [
        'Tu te connectes avec GitHub ; ce que le store garde de toi, pourquoi et combien de temps est dans la politique de confidentialité d’escouade.dev, section « Le store ».',
        'Tu peux exporter tes données et supprimer ton compte depuis la page de ton compte.',
      ],
    },
    {
      title: 'La loi',
      items: ['Ces conditions sont régies par la loi française. Contact : guillaume@gagnaire.dev.'],
    },
  ],
};

const en: Terms = {
  title: 'The authors’ terms',
  version: `Version ${TERMS_VERSION}, of 29 October 2026`,
  lead: 'These terms apply to whoever publishes a plugin on store.escouade.dev, published by GAGNAIRE DEVELOPPEMENT. They are accepted at the first submission; a new version is accepted again at the next one.',
  sections: [
    {
      title: 'What you publish',
      items: [
        'You hold the rights to what you publish: the code, the texts, the images and the names.',
        'Your plugin is under an open-source licence recognised by the OSI or the FSF, given by its SPDX identifier in the manifest and in the repository’s licence file.',
        'The store sells nothing: no sale and no payment inside a plugin through Escouade.',
        'A published version is never changed and its number is never reused. You may yank it, unlist the plugin, transfer it or delete it from your account.',
      ],
    },
    {
      title: 'What is forbidden',
      items: [
        'Malicious code, and anything that gets around Escouade’s permissions.',
        'Collecting data the plugin’s permissions do not declare.',
        'Mining, and any use of the users’ machines for other ends than what the plugin announces.',
        'Unlawful content, and impersonating a name, a brand or an author.',
      ],
    },
    {
      title: 'What Escouade may do',
      items: [
        'Refuse a submission, unlist a plugin, yank a version or revoke its badge, at any time. Escouade says why, unless saying it would put users at risk.',
        'Keep a deleted plugin 30 days, and its identifier reserved to your account afterwards: nobody publishes something else under the same name.',
      ],
    },
    {
      title: 'The “Verified by Escouade” badge',
      items: [
        '“Verified by Escouade” means that one precise version, the one of its hash, was reviewed and signed. It is not a guarantee: Escouade is not liable for a third-party plugin, verified or not.',
        'A new version has no badge until it is reviewed in turn.',
      ],
    },
    {
      title: 'Your account and your data',
      items: [
        'You sign in with GitHub; what the store keeps of you, why and how long is in escouade.dev’s privacy policy, section “The store”.',
        'You can export your data and delete your account from your account’s page.',
      ],
    },
    {
      title: 'The law',
      items: ['These terms are governed by French law. Contact: guillaume@gagnaire.dev.'],
    },
  ],
};

export const TERMS: Record<Lang, Terms> = { fr, en };
