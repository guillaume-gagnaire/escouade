import type { Tree } from '../types';

// The pull requests of an ordinary agent (ESC-64): « Ouvrir une PR » from the header of its conversation, the menu of
// its card and the branch picker; then their state on its card and in the header, and the two corrections handed to the
// agent on a click. `{word}`: the code host's word, « PR » or « MR » (GitLab), always feminine in French (« la PR »).
export default {
  /** The items of the menus and the button of the header. */
  open: 'Ouvrir une {word}…',
  openButton: 'Ouvrir une {word}',
  openHint: 'Pousse la branche {branch} et ouvre sa {word}, titre et description proposés.',
  /** The item of the menus once one is open. */
  see: 'Voir la {word} #{number}',
  /** On the card when the plugin that serves its code host does not answer (switched off, stopped): its pull requests
   * read last stay. The tooltip says why. */
  down: 'Hébergeur arrêté : {word} plus suivies',
  downHint: '{why}. Les {word} lues en dernier restent ; Réglages › Plugins le rallume.',

  modal: {
    title: 'Ouvrir une {word} sur {host}',
    /** Under the title: the agent and where its branch goes. */
    scope: '{agent} · {head} → {base}',
    titleLabel: 'Titre',
    bodyLabel: 'Description',
    baseLabel: 'Branche cible',
    draft: 'En brouillon',
    proposing: 'Rédaction du titre et de la description…',
    fallback: 'Le modèle des textes rédigés n’a pas répondu : titre et description tirés des commits.',
    uncommitted: {
      one: '{count} fichier non commité du worktree ne sera pas dans la {word}.',
      other: '{count} fichiers non commités du worktree ne seront pas dans la {word}.',
    },
    regenerate: 'Proposer à nouveau',
    submit: 'Pousser et ouvrir',
    opening: 'Ouverture…',
    opened: '{word} #{number} ouverte.',
  },

  /** What a pull request is, in its tooltip: its state, its CI, its reviews. */
  state: {
    open: 'ouverte',
    draft: 'brouillon',
    merged: 'mergée',
    closed: 'fermée',
  },
  checks: {
    pending: 'CI en cours',
    success: 'CI verte',
    failure: 'CI en échec',
    skipped: 'CI ignorée',
    none: 'pas de CI',
  },
  approvals: { one: '{count} approbation', other: '{count} approbations' },
  changesRequested: 'changements demandés',
  threads: { one: '{count} fil ouvert', other: '{count} fils ouverts' },
  conflict: 'conflit avec {base}',
  /** The tooltip's first line. */
  tipHead: '{word} #{number} · {title}',
  /** The number's label, for a screen reader: what a click does. */
  chipLabel: 'Ouvrir la {word} #{number} dans le navigateur ({state})',
  /** The group under the card. */
  group: 'Pull requests de {name}',

  /** The corrections, on a click only. */
  fixChecks: 'L’agent corrige',
  fixChecksHint: 'Envoie à {agent} les checks en échec de la {word} #{number}, avec la fin de leurs journaux.',
  fixReviews: 'L’agent corrige les retours',
  fixReviewsHint: 'Envoie à {agent} les demandes de changement et les fils non résolus de la {word} #{number}.',
  fixSent: 'Envoyé à {agent}.',
  /** Beside the number on the card: where it goes. */
  to: '→ {base}',
  seeCi: 'Voir la CI de la {word} #{number} dans le panneau',

  /**
   * The project's « Pull requests » view (ESC-65), beside the Kanban: its open pull requests, three filters, and an agent
   * launched on one. Read only.
   */
  list: {
    /** The sidebar's button, its word: « PR », « MR » on GitLab. */
    tab: '{word}',
    tabHint: 'Les {word} ouvertes du projet',
    title: 'Pull requests',
    titleMr: 'Merge requests',
    open: { one: '{count} ouverte', other: '{count} ouvertes' },
    filterLabel: 'Quelles {word}',
    filter: {
      mine: 'Les miennes',
      review: 'Relecture demandée',
      all: 'Toutes',
    },
    readAt: 'Lu à {time}',
    refresh: 'Rafraîchir',
    empty: {
      all: 'Aucune {word} ouverte.',
      mine: 'Aucune {word} ouverte par toi.',
      review: 'Personne ne t’a demandé de relecture.',
    },
    emptyHint: 'Les {word} ouvertes du dépôt, par toi ou par d’autres, s’affichent ici.',
    showAll: 'Toutes les {word}',
    /** The host did not say whose account it read them with. */
    noMe: 'L’hébergeur ne dit pas ton identifiant : « Les miennes » et « Relecture demandée » restent vides.',
    draft: 'Brouillon',
    fork: 'fork {owner}',
    forkHint: 'Depuis le fork {repo}, branche {branch}',
    agentHint: 'L’agent {agent} y travaille',
    conflict: 'Conflit',
    changesRequested: 'Changements demandés',
    yourReview: 'Ta relecture',
    noCi: 'Pas de CI',
    /** A row, for a screen reader. */
    row: '{word} #{number} · {title}',
    launch: 'Lancer un agent',
    launchHint: 'Lancer un agent sur cette {word} : un worktree sur sa branche {branch} (Entrée)',
    launching: 'Lancement…',
    openAgent: 'Ouvrir l’agent',
    openAgentHint: 'Ouvrir {agent}, qui travaille sur cette {word}',
  },

  /** « Faire relire par un agent » (ESC-66): a review written by an agent, kept, edited or dropped comment by comment, then published at once. */
  review: {
    ask: 'Faire relire',
    askHint: 'Un agent lit le diff de cette {word} et rédige une relecture. Rien n’est publié sans toi.',
    askLong: 'Faire relire par un agent',
    show: 'Voir la relecture',
    showHint: 'Ouvrir la relecture de cette {word}',
    mark: {
      reading: 'Relecture en cours',
      failed: 'Relecture en échec',
      published: 'Relecture publiée',
      ready: 'Relecture à publier',
      open: { one: 'Relecture : {count} à trancher', other: 'Relecture : {count} à trancher' },
    },
    ready: 'Relecture de la {word} #{number} prête.',
    failedToast: 'La relecture de la {word} #{number} a échoué.',
    open: 'Voir',
    back: 'Revenir aux {word}',
    by: 'Relu par {who}',
    reading: 'L’agent relit la {word} #{number}…',
    readingHint: 'Tu peux quitter cette vue : la relecture continue et rien n’est publié.',
    failed: 'La relecture n’a pas abouti.',
    discard: 'Écarter la relecture',
    cancel: 'Annuler',
    summary: 'Texte de la relecture',
    excerpt: 'Le diff de {path} à la ligne {line}',
    noteLabel: 'Commentaire sur {path}, ligne {line}',
    outside: 'Hors du diff : partira dans le texte de la relecture',
    leftOut: {
      one: '{count} fichier n’a pas été lu : trop long ou généré.',
      other: '{count} fichiers n’ont pas été lus : trop longs ou générés.',
    },
    comments: { one: '{count} commentaire', other: '{count} commentaires' },
    none: 'L’agent n’a rien à signaler ligne par ligne.',
    keep: 'Garder',
    edit: 'Corriger',
    drop: 'Écarter',
    takeBack: 'Reprendre',
    kept: 'Gardé',
    edited: 'Corrigé et gardé',
    dropped: 'Écarté',
    editLabel: 'Corriger le commentaire',
    saveKeep: 'Garder ce texte',
    backToAgent: 'Revenir au texte de l’agent',
    toDecide: { one: '{count} à trancher', other: '{count} à trancher' },
    keptCount: { one: '{count} gardé', other: '{count} gardés' },
    droppedCount: { one: '{count} écarté', other: '{count} écartés' },
    promise: 'rien ne part avant ton clic',
    verdict: 'Ce que dit la relecture',
    verdicts: { commented: 'Commenter', changesRequested: 'Demander des changements', approved: 'Approuver' },
    ownHint: 'L’hébergeur ne le permet pas sur ta propre {word}.',
    publish: 'Publier sur {host}',
    publishing: 'Publication…',
    published: 'Relecture publiée sur la {word} #{number}.',
    publishedAt: 'Publiée sur {host} à {time}.',
    block: {
      open: { one: 'Encore {count} commentaire à garder ou à écarter.', other: 'Encore {count} commentaires à garder ou à écarter.' },
      text: 'Écris le texte de la relecture pour demander des changements.',
      nothing: 'Rien à publier.',
    },
    moved: {
      title: 'La {word} a bougé depuis la relecture',
      body: 'De nouveaux commits sont arrivés sur cette {word} : l’agent ne les a pas lus. Un commentaire dont la ligne a quitté le diff partira dans le texte de la relecture.',
      confirm: 'Publier quand même',
    },
    again: {
      button: 'Relire à nouveau',
      title: 'Relire à nouveau ?',
      body: 'Une nouvelle relecture remplace celle-ci, avec ce que tu as gardé, corrigé ou écarté.',
      confirm: 'Relire à nouveau',
    },
  },

  /** The side panel's « PR et CI » tab (ESC-305): read only, but « Relancer les échecs » on a click. */
  view: {
    loading: 'Lecture de la {word}…',
    retry: 'Réessayer',
    openInBrowser: 'Ouvrir dans le navigateur',
    /** Under its title: who opened it, and when (« il y a 2 h »). */
    opened: 'créée {ago}',
    by: 'par {author}',
    conflict: 'Conflit avec {base}',
    others: 'Ses autres {word}',
    ci: {
      title: 'CI',
      failed: { one: '{count} échec sur {total}', other: '{count} échecs sur {total}' },
      running: { one: '{count} en cours sur {total}', other: '{count} en cours sur {total}' },
      passed: { one: '{count} vérification réussie', other: '{count} vérifications réussies' },
      cancelled: { one: '{count} annulée', other: '{count} annulées' },
      none: 'Aucune vérification sur le dernier commit.',
      phase: {
        queued: 'en attente',
        running: 'en cours',
        success: 'réussie',
        failure: 'échouée',
        cancelled: 'annulée',
        skipped: 'ignorée',
      },
      rerun: 'Relancer les échecs',
      rerunHint: 'Relance sur l’hébergeur les jobs en échec du dernier commit.',
      rerunning: 'Relance…',
      rerunDone: { one: '{count} exécution relancée.', other: '{count} exécutions relancées.' },
      rerunNone: 'Aucun job à relancer : l’hébergeur ne relance que ses propres jobs.',
      log: 'Journal de {name}',
      logShow: 'Voir le journal',
      logLoading: 'Lecture du journal…',
      logNone: 'L’hébergeur ne garde pas le journal de cette vérification.',
      logTail: 'La fin du journal',
      onHost: 'Voir sur l’hébergeur',
    },
    reviews: {
      title: 'Revues et commentaires',
      none: 'Aucune revue ni commentaire.',
      approved: 'a approuvé',
      changesRequested: 'demande des changements',
      commented: 'a commenté',
      resolved: 'résolu',
      readOnly: 'En lecture : Escouade ne publie rien en ton nom.',
    },
    deployments: {
      title: 'Déploiements',
      open: 'Ouvrir {url}',
      state: {
        success: 'déployé',
        failure: 'échoué',
        error: 'en erreur',
        pending: 'en attente',
        queued: 'en file',
        inProgress: 'en cours',
        inactive: 'remplacé',
      },
    },
    noPr: {
      title: 'Pas de {word} sur cette branche',
      settings: 'Réglages › Services connectés',
    },
    project: {
      title: '{word} ouvertes du projet',
      none: 'Aucune {word} ouverte.',
      loading: 'Lecture des {word} ouvertes du projet…',
      ofAgent: 'Ouvrir l’agent {agent}',
    },
    failure: {
      refused: 'Jeton refusé par l’hébergeur : reconnecte son compte.',
      rateLimit: 'Limite d’API de l’hébergeur atteinte.',
      rateLimitUntil: 'Limite d’API de l’hébergeur atteinte, jusqu’à {time}.',
      unreachable: 'Hébergeur injoignable.',
    },
  },
} as const satisfies Tree;
