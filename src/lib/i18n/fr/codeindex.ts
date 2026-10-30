import type { Tree } from '../types';

// « Index du code » (ESC-254): its setting in the project's settings and in « Nouveau projet », the model of the search by
// meaning, and what it saved in the statistics.
export default {
  what: 'Les agents trouvent le code plus vite : en un appel, ils ont un symbole, ses appelants (justes même quand le nom a des homonymes), ses appelés et ce qu’un changement toucherait.',
  local:
    'Tout reste sur cette machine : l’index est calculé ici et gardé dans le dossier d’Escouade, jamais dans le dépôt ; rien n’est envoyé en ligne.',
  privateNote:
    'Projet privé : son index ne sert qu’à ses propres agents ; ceux des autres projets n’y ont jamais accès. Les fichiers ignorés par git ne sont pas lus.',
  excluded: {
    one: 'Laissé hors de l’index : {list} (un autre projet dans son dossier).',
    other: 'Laissés hors de l’index : {list} (d’autres projets dans son dossier).',
  },
  settings: {
    title: 'Index du code',
    on: 'Indexer ce projet',
    onDesc:
      'Allumé, le projet est lu une fois, puis tenu à jour aux commits, aux changements de branche et aux fins de tour. Le couper efface l’index.',
    reading: 'Lecture du projet…',
    waiting: 'En attente de lecture, après les autres projets…',
    follows: {
      on: 'Suit le réglage de l’app (allumé).',
      off: 'Suit le réglage de l’app (coupé).',
    },
    chosen: 'Choisi pour ce projet.',
    follow: 'Suivre le réglage de l’app',
    followed: 'Ce projet suit de nouveau le réglage de l’app.',
    seeAll: 'Voir le réglage',
    status: '{files} fichiers · {symbols} symboles · {size} · lu {when}',
    turnedOn: 'Index du code allumé : {files} fichiers lus.',
    turnedOff: 'Index du code coupé : son index est effacé.',
  },
  vectors: {
    label: 'Recherche par le sens',
    desc: 'En plus des mots, la mémoire partagée et les docs se cherchent par le sens, avec un petit modèle multilingue ({model}, licence {license}) qui tourne sur cette machine. Un seul téléchargement de {size}, pour tous les projets ; ensuite, rien ne sort de la machine.',
    download: 'Télécharger ({size})',
    resume: 'Reprendre ({pct} %)',
    cancel: 'Annuler',
    downloaded: 'Téléchargé',
    delete: 'Effacer',
    deleteTitle: 'Effacer le modèle ({size}) : la recherche se fait par les mots seuls jusqu’au prochain téléchargement.',
    progress: 'Téléchargement du modèle',
  },
  wizard: {
    title: 'Indexer le code',
  },
  all: {
    title: 'Tous les projets',
    on: 'Index du code sur tous les projets',
    onDesc:
      'Chaque projet qui n’a pas fait son propre choix suit cette case. Allumée, ils sont lus un par un, en arrière-plan ; coupée, leur index est effacé. Un projet allumé ou coupé dans ses réglages garde son choix.',
    fence:
      'La clôture des projets privés est respectée : l’index d’un projet privé ne sert qu’à ses propres agents. Le modèle de la recherche par le sens ne se télécharge pas pour autant.',
    followersNone: 'Aucun projet ne la suit : chacun a fait son propre choix.',
    followers: {
      one: 'Suivie par {count} projet sur {total}.',
      other: 'Suivie par {count} projets sur {total}.',
    },
    turnedOnNone: 'Index du code sur tous les projets allumé : aucun projet ne le suit pour l’instant.',
    turnedOn: {
      one: 'Index du code sur tous les projets allumé : {count} projet sera lu, en arrière-plan.',
      other: 'Index du code sur tous les projets allumé : {count} projets seront lus, un par un, en arrière-plan.',
    },
    turnedOffNone: 'Index du code sur tous les projets coupé.',
    turnedOff: {
      one: 'Index du code sur tous les projets coupé : l’index de {count} projet est effacé.',
      other: 'Index du code sur tous les projets coupé : l’index de {count} projets est effacé.',
    },
    confirmTitle: {
      one: 'Couper l’index du code de {count} projet ?',
      other: 'Couper l’index du code de {count} projets ?',
    },
    confirmBody: {
      one: 'Le projet qui suit ce réglage perd son index, effacé de cette machine. Ceux qui ont fait leur propre choix gardent le leur.',
      other:
        'Les {count} projets qui suivent ce réglage perdent leur index, effacé de cette machine. Ceux qui ont fait leur propre choix gardent le leur.',
    },
    confirm: 'Couper et effacer',
  },
  stats: {
    title: 'Index du code',
    note: 'Les réponses que l’index a données aux agents, et leur poids en tokens. La lecture de fichiers évitée est une estimation haute, pas une économie mesurée.',
    project: 'Projet',
    calls: 'Réponses',
    tokens: 'Tokens des réponses',
    avoided: 'Lecture de fichiers évitée (estimation haute)',
    avoidedTip:
      'Les fichiers d’où viennent les réponses de l’index, moins ces réponses, comptés comme si l’agent les avait lus entiers. Ce n’est pas une économie mesurée : avec de vrais agents, un agent qui cherche avec grep lit bien moins que cela, et le coût total change peu.',
    none: 'Aucune réponse de l’index sur la période.',
  },
} as const satisfies Tree;
