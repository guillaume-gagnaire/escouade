import type { Tree } from '../types';

// The message field, and the cards that wait for an answer (permissions, questions).
export default {
  /** The efforts offered (`lib/models.ts`): the label of a chip, and what it means. */
  effort: {
    low: { label: 'Bas', title: 'Réponses rapides, peu de réflexion' },
    medium: { label: 'Moyen', title: 'Équilibré' },
    high: { label: 'Élevé', title: 'Réflexion approfondie' },
    xhigh: { label: 'Très élevé', title: 'Réflexion très approfondie' },
    max: { label: 'Max', title: 'Réflexion maximale, plus de tokens' },
  },
  /** The permission modes offered (`lib/models.ts`). */
  mode: {
    auto: { label: 'Auto', title: 'Un classifieur approuve les actions sûres et demande pour le reste' },
    default: { label: 'Demander', title: 'Claude demande ton accord avant chaque action sensible' },
    plan: { label: 'Plan', title: 'Claude analyse et propose un plan sans rien modifier' },
    acceptEdits: { label: 'Édits auto', title: 'Les modifications de fichiers sont acceptées sans demander' },
    bypassPermissions: { label: 'Bypass', title: 'Aucune demande de permission (à réserver aux environnements sûrs)' },
    autoUnavailableDetail: 'indisponible avec Haiku',
    autoUnavailableTitle: "Le mode Auto n'est pas disponible avec Haiku",
    bypassToast: 'Mode Bypass : Claude agira sans aucune demande de permission.',
  },

  /** Files attached to a message (`lib/attachments.ts`). */
  attachments: {
    kilobytes: '{n} Ko',
    unreadable: "{name} n'a pas pu être lu",
    unsupported:
      '« {name} » ne peut pas être joint : les fichiers acceptés sont les images (PNG, JPEG, GIF, WebP), les PDF et les fichiers texte.',
    tooBig: '{name} dépasse {max}',
    notText: "{name} n'est pas un fichier texte : il ne peut pas être joint.",
    totalTooBig: "{name} n'est pas joint : les fichiers d'un message sont limités à {max} en tout.",
    remove: 'Retirer {name}',
  },
  attach: {
    label: 'Joindre un fichier',
    title: 'Joindre une image, un PDF ou un fichier texte (ou colle / glisse-le)',
  },

  placeholder: {
    denyPermission: 'Explique à Claude quoi faire à la place (refuse la demande)…',
    answerQuestion: 'Réponds à la question ou écris une réponse libre…',
    sendTo: 'Envoyer un message à {name}…',
    describe: 'Décris la tâche à confier à Claude…',
  },

  /** The bar under the field: the menus, stop. */
  bar: {
    effort: 'Effort',
    effortTitle: 'Effort de réflexion',
    effortUnsupported: "Haiku ne gère pas l'effort",
    mode: 'Mode',
    interrupt: 'Interrompre ({key})',
    stop: 'Stop',
  },

  send: {
    label: 'Envoyer',
    /** `{enter}`: the Enter key as written on this keyboard. */
    titleQueued: 'Claude en tiendra compte dès sa prochaine étape · {enter} pour envoyer',
    title: '{enter} pour envoyer · {newline} pour aller à la ligne',
    alreadyWaiting: 'Un message attend déjà la fin de la préparation.',
    filesKept: "Les fichiers joints n'accompagnent pas une réponse : ils restent prêts pour ton prochain message.",
  },

  /** The card of a request for permission, pending or answered. */
  permission: {
    title: 'Claude demande une autorisation',
    titlePlan: 'Claude propose un plan',
    /** What Claude is told when the plan is refused. */
    keepPlanningMessage: 'Continue à planifier : le plan ne me convient pas encore.',
    allow: 'Autoriser',
    approvePlan: 'Approuver le plan',
    always: 'Toujours autoriser',
    approvePlanAndEdits: 'Approuver et accepter les édits',
    deny: 'Refuser',
    keepPlanning: 'Continuer à planifier',
    denyHint: 'Pour refuser en expliquant quoi faire à la place, écris-le dans le champ ci-dessous.',
    denied: 'Refusé',
    allowed: 'Autorisé',
    alwaysAllowed: 'Toujours autorisé',
    cancelled: 'Demande annulée',
  },

  /** The card of a question Claude asks, pending or answered. */
  question: {
    waiting: 'Claude attend ta réponse',
    orWrite: 'ou réponds librement dans le champ ci-dessous',
    submit: 'Valider',
    unanswered: 'Question restée sans réponse',
  },
} as const satisfies Tree;
