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
    default: { label: 'Demander', title: '{engine} demande ton accord avant chaque action sensible' },
    plan: { label: 'Plan', title: '{engine} analyse et propose un plan sans rien modifier' },
    acceptEdits: { label: 'Édits auto', title: 'Les modifications de fichiers sont acceptées sans demander' },
    bypassPermissions: { label: 'Bypass', title: 'Aucune demande de permission (à réserver aux environnements sûrs)' },
    autoUnavailableDetail: 'indisponible avec ce modèle',
    autoUnavailableTitle: "Le mode Auto n'est pas disponible avec ce modèle, d'après le Claude Code de ce compte",
    /** A mode its harness lacks, greyed in its place (choice 17): `{harness}`, `{mode}`. */
    missing: '{harness} n’a pas de mode {mode}.',
    missingAuto: '{harness} n’a pas de mode Auto : rien ne décide à ta place.',
    bypassToast: 'Mode Bypass : {engine} agira sans aucune demande de permission.',
  },

  /** Files attached to a message (`lib/attachments.ts`). */
  attachments: {
    kilobytes: '{n} Ko',
    unreadable: "{name} n'a pas pu être lu",
    unsupported:
      '« {name} » ne peut pas être joint : les fichiers acceptés sont les images (PNG, JPEG, GIF, WebP), les PDF et les fichiers texte.',
    tooBig: '{name} dépasse {max}',
    notText: "{name} n'est pas un fichier texte : il ne peut pas être joint.",
    remove: 'Retirer {name}',
    /** The name of the bar that shows a copy going on. */
    copying: 'Copie de {name}',
    /** A file past the size the settings warn at: copied to the disk, or kept where it is. */
    warnCopy: 'Gros fichier : {size}, copié sur le disque',
    warnOrigin: 'Gros fichier : {size}, joint là où il est, sans copie',
    /** The mark of a chip whose file is the original one, attached where it lies (nothing was copied). */
    original: 'Fichier d’origine, lu là où il est',
  },
  attach: {
    label: 'Joindre un fichier',
    title: 'Joindre un fichier (ou colle / glisse-le)',
  },
  /** Files dragged over the composer: the whole field takes them. */
  drop: {
    title: 'Dépose pour joindre à ton message',
    detail: 'Images, PDF et textes partent dans le message ; le reste, par son chemin',
  },

  placeholder: {
    denyPermission: 'Explique à {engine} quoi faire à la place (refuse la demande)…',
    answerQuestion: 'Réponds à la question ou écris une réponse libre…',
    sendTo: 'Envoyer un message à {name}…',
    describe: 'Décris la tâche à confier à {engine}…',
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
    titleQueued: '{engine} en tiendra compte dès sa prochaine étape · {enter} pour envoyer',
    /** `{s}`: seconds before it goes. */
    titleHeld: 'Envoyé à {engine} dans {s} s, annulable d’ici là · {enter} pour envoyer',
    titleTurnEnd: '{engine} le recevra à la fin de son tour, annulable d’ici là · {enter} pour envoyer',
    title: '{enter} pour envoyer · {newline} pour aller à la ligne',
    alreadyWaiting: 'Un message attend déjà la fin de la préparation.',
    filesKept: "Les fichiers joints n'accompagnent pas une réponse : ils restent prêts pour ton prochain message.",
  },

  /** The card of a request for permission, pending or answered. */
  permission: {
    title: '{engine} demande une autorisation',
    titlePlan: '{engine} propose un plan',
    /** What Claude is told when the plan is refused. */
    keepPlanningMessage: 'Continue à planifier : le plan ne me convient pas encore.',
    allow: 'Autoriser',
    approvePlan: 'Approuver le plan',
    /** After « Approuver le plan »: the mode its work resumes in (ESC-277), `{mode}` its label (« Auto »). */
    resumesIn: 'Le travail reprendra en mode {mode}',
    always: 'Toujours autoriser',
    /** Allowed for the rest of the session, the same request (Codex: `acceptForSession`). */
    forSession: 'Pour cette session',
    sessionAllowed: 'Autorisé pour cette session',
    /** Allowed for good, but for this agent alone (Cursor CLI: its rule kept in the agent's own config folder). */
    forAgent: 'Pour cet agent',
    agentAllowed: 'Autorisé pour cet agent',
    approvePlanAndEdits: 'Approuver et accepter les édits',
    deny: 'Refuser',
    keepPlanning: 'Continuer à planifier',
    denyHint: 'Pour refuser en expliquant quoi faire à la place, écris-le dans le champ ci-dessous.',
    denied: 'Refusé',
    allowed: 'Autorisé',
    alwaysAllowed: 'Toujours autorisé',
    /** Answered through the MCP server, not by the user in the window: « par <author> », then « via Escouade ». */
    by: 'par',
    via: 'via Escouade',
    cancelled: 'Demande annulée',
  },

  /** The card of a question Claude asks, pending or answered. */
  question: {
    waiting: '{engine} attend ta réponse',
    asked: '{engine} t’a posé une question',
    orWrite: 'ou réponds librement dans le champ ci-dessous',
    saidAnswer: 'Réponse dite : « {text} »',
    submit: 'Valider',
    unanswered: 'Question restée sans réponse',
    /** Answered through the MCP server, not by the user in the window (see `permission.by`). */
    answered: 'Répondu',
    /** A form of an MCP server (Copilot: `elicitation`): its fields to fill. */
    form: 'Formulaire de {source}',
    formNoSource: 'Formulaire',
    required: 'obligatoire',
    fieldPlaceholder: 'Ta réponse',
    numberPlaceholder: 'Un nombre',
  },

  /** What a model costs beside the others, beside its name in the menu (Copilot). */
  model: {
    multiplier: '×{n}',
    multiplierTitle: 'Coûte {n} fois le tarif de base',
    /** Beside the stand-in model while the account's are being read (Codex, ESC-186). */
    reading: 'liste en cours de lecture',
    /** Beside a model the account's config names, its harness not listing it (Codex's config.toml, ESC-186). */
    fromConfig: 'de config.toml',
    price: {
      low: 'coût bas',
      medium: 'coût moyen',
      high: 'coût élevé',
      veryHigh: 'coût très élevé',
    },
  },
} as const satisfies Tree;
