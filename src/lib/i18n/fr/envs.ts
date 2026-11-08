import type { Tree } from '../types';

// The deployments (ESC-67): the environments a done ticket is in, on its card; the preview of a pull request, on a card
// « En revue »; the view « Environnements » of a project's board.
export default {
  /** On a done ticket's card. */
  card: {
    label: 'Environnements',
    deployed: '{name} : déployé',
    waiting: '{name} : pas encore déployé',
    /** « Ouvrir l'aperçu », beside « ▶ Tester » on a card « En revue ». */
    preview: 'Ouvrir l’aperçu',
    previewTitle: 'Ouvre l’aperçu de la {word} : {url}',
  },

  /** The tabs of the board's header. */
  tabs: { label: 'Vue du Kanban', tickets: 'Tickets', environments: 'Environnements' },

  view: {
    title: 'Environnements',
    sub: '{project} · {host}',
    read: 'lu {since}',
    refresh: 'Rafraîchir',
    refreshTitle: 'Relit les déploiements de l’hébergeur maintenant',
    waiting: 'Mergés, pas encore déployés',
    notInProd: {
      one: '{count} ticket pas encore en production',
      other: '{count} tickets pas encore en production',
    },
    /** What runs in an environment: its commit, when, by whom. */
    runs: '{sha} · {when}',
    by: 'par {author}',
    nothingRuns: 'Aucun déploiement réussi',
    failed: 'Dernier déploiement en échec · {sha}',
    failedNoSha: 'Dernier déploiement en échec',
    job: 'Voir le job',
    underWay: 'Déploiement en cours · {sha}',
    more: { one: '+ {count} autre', other: '+ {count} autres' },
    merged: 'mergé {since}',
    empty: 'Aucun ticket n’est arrivé ici.',
    waitingEmpty: 'Rien n’attend un déploiement.',
    production: 'production',
    /** The environments left out because nothing ever ran there (ESC-390). */
    neverRan: {
      one: '{names} n’est pas montré : aucun déploiement réussi.',
      other: '{names} ne sont pas montrés : aucun déploiement réussi.',
    },
    noneShown: 'Aucun environnement n’est montré : choisis-les dans le menu des environnements.',
    loading: 'Lecture des déploiements…',
    none: 'Aucun déploiement sur {host} pour ce dépôt. Ceux de la CI (GitHub Actions, GitLab CI, Bitbucket Pipelines) apparaissent ici d’eux-mêmes.',
    unsupported: '{host} n’a pas d’API de déploiements : Escouade ne suit que la CI de la branche cible.',
    error: 'Déploiements illisibles : {error}',
  },

  /** The menu of the environments shown (ESC-390), in the board's header. */
  menu: {
    button: { one: '{count} environnement sur {total}', other: '{count} environnements sur {total}' },
    short: '{count} sur {total}',
    title: 'Environnements montrés',
    tip: 'Choisir les environnements montrés et leur ordre',
    tickets: { one: '{count} ticket', other: '{count} tickets' },
    byYou: 'masqué par toi',
    neverRan: 'aucun déploiement réussi',
    move: 'Glisser pour changer l’ordre',
    foot: 'Glisse pour changer l’ordre, ou Alt+↑ et Alt+↓. Gardé pour ce projet, sur cette machine.',
  },
} as const satisfies Tree;
