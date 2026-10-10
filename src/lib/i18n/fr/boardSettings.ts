import type { Tree } from '../types';

// The settings of the Kanban, and the sentences its header and cards build from them (`lib/board.ts`).
export default {
  // What the header says about the free places.
  places: {
    free: { one: '{count} place libre', other: '{count} places libres' },
    full: 'Toutes les places sont prises',
    quota: 'Quota atteint — reprise à {time}',
  },

  // Why no ticket starts: the autopilot is paused by the quota or by a usage limit.
  pause: {
    limit: "Pilote auto en pause : limite d'usage atteinte (reprise vers {time})",
    week: 'Pilote auto en pause : quota hebdo à {pct} (reprise {when})',
    fiveHour: 'Pilote auto en pause : quota de 5 h à {pct} (reprise {when})',
  },

  // What validating a ticket does, in the header's button.
  summary: {
    merge: 'merge {strategy} → {target}',
    pr: 'PR → {target}',
    push: 'push ticket/*',
    keep: "laisser en l'état",
  },

  // The button that validates a ticket, by what validating does.
  approve: {
    merge: 'Valider et merger',
    pr: 'Valider + PR',
    push: 'Valider et pousser',
    keep: 'Valider',
  },

  // When a ticket « À faire » starts.
  wait: {
    targetBranch: 'En attente de la branche cible',
    after: '⏸ après {keys}',
    pause: 'En attente : pilote auto en pause',
    forced: 'Lancement demandé…',
    autopilotOff: 'Pilote auto désactivé',
    next: "Pris dès qu'une place se libère",
    place: "En attente d'une place ({busy}/{max})",
  },

  // What « Lancer » asks first on a ticket that waits for others.
  launchAnyway: {
    one: '{key} attend {keys}, pas encore terminé. Le lancer quand même ?',
    other: '{key} attend {keys}, pas encore terminés. Le lancer quand même ?',
  },

  cycle: '{dep} attend déjà {ticket} (directement ou non).',

  // The example of a generated commit message, in the settings.
  commit: {
    conventional: 'feat: limiter les tentatives de connexion [{key}]',
    plain: '{key} Limiter les tentatives de connexion',
  },

  // The tag of a ticket's agent in the sidebar and the overview.
  tag: {
    loop: '{key} · boucle {iteration}/{max}',
    review: '{key} · à tester',
  },

  // « 3 boucles · 0,42 $ » on a finished ticket.
  loops: { one: '{count} boucle · {cost}', other: '{count} boucles · {cost}' },

  // The Kanban tab of the project's settings.
  tab: {
    actionsTitle: 'Quand je valide un ticket « À tester »',
    actions: {
      merge: {
        label: 'Merger dans une branche',
        desc: "Fusionne le worktree de l'agent dans la branche cible, puis libère l'agent.",
      },
      pr: {
        label: 'Ouvrir une pull request',
        desc: 'Pousse ticket/<clé> et ouvre une PR vers la branche cible pour relecture.',
      },
      push: {
        label: 'Pousser la branche du ticket',
        desc: 'Commit et push sur ticket/<clé>, sans merge ni PR.',
      },
      keep: {
        label: "Laisser en l'état",
        desc: 'Les modifications restent non commitées dans le worktree.',
      },
    },
    targetBranch: 'Branche cible',
    strategy: 'Stratégie',
    strategies: { merge: 'Merge commit', squash: 'Squash', rebase: 'Rebase' },
    draftPr: 'PR en brouillon',

    beforeAfter: 'Avant et après',
    testCommand: 'Commande de tests',
    testCommandHint: 'lancée dans le worktree du ticket',
    testCommandPlaceholder: 'ex. npm test',
    testsFirst: 'Relancer les tests avant',
    testsFirstDesc: "Bloque l'action si un test échoue et renvoie le ticket à l'agent.",
    cleanup: 'Supprimer le worktree une fois validé',
    cleanupDesc:
      "Libère l'espace disque, après ses commandes de démontage. Après un merge, sa branche part aussi ; poussée ou proposée en PR, elle reste.",
    commitMessage: 'Message de commit généré',
    onConflict: 'En cas de conflit',
    conflicts: { ask: 'Me demander', agent: "L'agent résout", abort: 'Annuler' },

    autopilot: 'Pilote auto',
    autoAssign: 'Attribuer les tickets automatiquement',
    autoAssignDesc: 'Un agent libre prend le prochain ticket « À faire »',
    quotaPause: 'Pause au-delà du quota',
    quotaPauseHint: 'pour tous les projets',
    quotaPauseDesc: 'Aucun ticket ne démarre tant que la fenêtre de 5 h ou la fenêtre hebdomadaire dépasse ce seuil.',

    parallel: 'En parallèle',
    parallelDesc: 'Au-delà, les tickets « À faire » attendent une place libre',
    defaultModel: 'Par défaut ({model})',
    default: 'Par défaut',
    effort: 'Effort',
    mode: 'Mode',
  },
} as const satisfies Tree;
