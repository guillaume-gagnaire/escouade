import type { Tree } from '../types';

// The Kanban: its columns, tickets and criteria.
export default {
  // The four columns (glossary): their names and what one says while it holds nothing.
  columns: { todo: 'À faire', doing: 'En cours', review: 'À tester', pr: 'En revue', done: 'Terminé' },
  empty: {
    todo: "Ajoute un ticket : un agent le prendra dès qu'une place se libère.",
    doing: 'Aucun agent en boucle',
    review: 'Rien à tester',
    pr: 'Aucune PR en revue',
    done: 'Aucun ticket terminé',
  },

  column: {
    newTicket: 'Nouveau ticket',
    /** The column « En revue » (ESC-59): its pull requests are read now rather than at the next minute. */
    refresh: 'Rafraîchir',
    refreshHint: 'Relire maintenant les PR en revue sur leur hébergeur',
  },

  /** A card « En revue » (ESC-59): its pull request as its host last told it. `{word}`: « PR », « MR » on GitLab. */
  pr: {
    /** Its state, in a word. */
    state: { draft: 'brouillon', open: 'ouverte', approved: 'approuvée', changes: 'changements demandés', closed: 'fermée' },
    /** Its CI, with how many jobs it has. */
    ci: {
      success: { one: 'CI verte · {count} job', other: 'CI verte · {count} jobs' },
      failure: '{failed}/{count} jobs en échec',
      pending: { one: 'CI en cours · {count} job', other: 'CI en cours · {count} jobs' },
      skipped: 'CI ignorée',
    },
    /** The host could not be read: said, the ticket stays where it is. */
    error: '{error} · depuis {since}',
    /** Merging from Escouade (ESC-62): « Merger » approved and green, « Merger quand c'est vert » while its CI runs. */
    merge: {
      now: 'Merger ({strategy})',
      nowTitle: '{word} #{number} approuvée et verte : merger dans {target} en {strategy}',
      whenGreen: 'Merger quand c’est vert',
      whenGreenTitle: 'Par l’auto-merge de l’hébergeur quand le dépôt l’autorise, sinon Escouade merge à la fin de la CI',
      hostOn: 'Auto-merge de {host} activé : mergée dès que la CI est verte',
      escouadeWaits: 'Escouade la merge ({strategy}) dès qu’elle est approuvée et verte, tant qu’elle tourne',
      escouadeTold: 'Pas d’auto-merge sur {host} pour ce dépôt : Escouade attend la fin de la CI et merge elle-même, tant qu’elle tourne',
      stopWaiting: 'Ne plus attendre',
      merged: '{word} #{number} mergée',
      none: 'Le dépôt n’autorise aucune des façons de merger d’Escouade',
      strategies: { merge: 'merge commit', squash: 'squash', rebase: 'rebase', semiLinear: 'semi-linéaire' },
    },
    finishAnyway: 'Terminer quand même',
    backToTodo: 'Remettre à faire',
    /** Its CI failed (ESC-60): its agent gets the failed jobs and the end of their logs. */
    fixCi: "L'agent corrige",
    fixCiTitle: 'Renvoyer à son agent les jobs en échec et la fin de leurs journaux ; il pousse, la {word} se met à jour',
    /** A CI of another kind gives a status and an address alone. */
    noLog: { one: '{names} : CI tierce, sans journal', other: '{names} : CI tierces, sans journal' },
    loopsSpent: 'Boucles épuisées ({count}/{max}) : le pilote auto ne la renvoie plus',
    /** What its agent does on its pull request (ESC-60, ESC-61). */
    fixingKind: {
      ci: "L'agent corrige la CI…",
      reviews: "L'agent corrige les retours…",
      merge: "L'agent met à jour la branche…",
      rebase: "L'agent rebase la branche sur {target}…",
    },
    /** New review comments (ESC-61): never given to its agent without a click. */
    feedback: { one: '1 retour de relecture', other: '{count} retours de relecture' },
    fixReviews: "L'agent corrige les retours",
    fixReviewsTitle:
      'Donner à son agent les commentaires non résolus arrivés depuis le dernier push (fichier, ligne, extrait du diff, auteur, texte) ; il pousse ses corrections',
    /** In conflict with its target (ESC-61). */
    updateBranch: "L'agent met à jour la branche",
    updateBranchTitle: {
      merge: 'Merger {target} dans la branche ; ses conflits vont à son agent, puis Escouade pousse',
      rebase: 'Rebaser la branche sur {target} ; ses conflits vont à son agent, puis Escouade pousse (--force-with-lease)',
    },
  },

  /** The CI of a branch without a pull request (ESC-63): a project's target in its board's header, a ticket's pushed branch on its card. */
  ci: {
    /** The pill: `{branch}` and, red, since when. */
    pill: {
      failure: '{branch} ✗ depuis {since}',
      success: '{branch} ✓',
      pending: '{branch} · CI en cours',
      none: '{branch} · aucune CI',
    },
    pillTitle: 'CI de {branch} : {state}. Cliquer pour voir les jobs, leurs journaux et les relancer',
    /** Its state in words, in the pill's tooltip. */
    state: {
      failure: { one: '{failed}/{count} job en échec', other: '{failed}/{count} jobs en échec' },
      success: { one: '{count} job réussi', other: '{count} jobs réussis' },
      pending: { one: '{count} job en cours', other: '{count} jobs en cours' },
      none: 'aucun job',
    },
    /** A reading that failed: the last state stays. */
    error: '{error} · depuis {since}',
    /** « Pause quand la CI de la cible est rouge »: the band under the board's header. */
    paused: 'Pilote auto en pause : la CI de {branch} est rouge depuis {since}',
    pausedShow: 'Voir les jobs',
    /** The window of a CI's jobs. */
    title: 'CI de {branch}',
    titlePr: 'CI de la {word} #{number}',
    loading: 'Lecture des jobs…',
    empty: 'Aucun job sur le dernier commit de {branch}.',
    refresh: 'Rafraîchir',
    rerun: 'Relancer les jobs en échec',
    rerunHint: 'Relance sur {host} les jobs en échec du dernier commit. Rien ne part sans ce clic.',
    rerunning: 'Relance…',
    rerunDone: { one: '{count} exécution relancée.', other: '{count} exécutions relancées.' },
    rerunNone: 'Aucun job à relancer : {host} ne relance que ses propres jobs.',
    noRerun: '{host} ne relance pas les jobs depuis Escouade : relance-les sur son site.',
    /** A job's log, in reading only, like a terminal. */
    log: 'Journal',
    logOf: 'Journal de {name}',
    logHint: 'Lecture seule · la fin du journal',
    logLoading: 'Lecture du journal…',
    logEmpty: 'Journal vide.',
    noLog: 'CI tierce : un statut et une adresse, pas de journal',
    onHost: 'Ouvrir sur {host}',
    send: 'Envoyer à un agent',
    sendHint: 'Donner ce job et la fin de son journal à un agent du projet, en message',
    sendNone: 'Aucun agent actif dans ce projet : crée-en un pour lui envoyer ce journal.',
    sent: 'Journal de {name} envoyé à {agent}.',
  },

  // The header of the board.
  header: {
    sub: {
      one: '{count} ticket · {looping} en boucle',
      other: '{count} tickets · {looping} en boucle',
    },
    noEngine: 'Aucun harness trouvé — aucun ticket ne démarre',
    importTitle: 'Importer des tickets de Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues ou Azure Boards',
    settingsTitle: 'Réglages du Kanban — {summary}',
    afterApproval: 'Après validation :',
    autopilotTitle: "Les tickets « À faire » partent seuls dès qu'une place se libère",
    autopilot: 'Pilote auto',
    autopilotOff: 'Pilote auto · off',
    resumeNow: 'Reprendre maintenant',
  },

  // A ticket's card.
  card: {
    openExternal: 'Ouvrir {key} dans {service}',
    syncError: 'Synchro avec {service} : {error}',
    resync: 'Resynchroniser',
    loop: 'Boucle {iteration}/{max}',
    loops: { one: '{count} boucle', other: '{count} boucles' },
    partial: 'Objectif partiel',
    // Its own model or effort, which come before the Kanban's for its agent (the label of the chip, and its tooltip).
    ownModel: 'Modèle de ce ticket : {model} (avant celui du Kanban)',
    ownEffort: 'Effort de ce ticket : {effort} (avant celui du Kanban)',
    ownBoth: 'Modèle de ce ticket : {model}, effort {effort} (avant celui du Kanban)',
    // Its own account (ESC-153): its agents start on it, and stay there.
    ownAccount: 'Compte de ce ticket : {account} (ses agents y restent)',
    // A ticket set on an account, its agent waiting for that account's quota: it never goes elsewhere.
    waitsAccount: 'En attente du quota de {account}, reprise à {at}',
    doneHeading: 'Ce qui a été fait',
    progressLabel: 'Avancement',
    moreSteps: { one: '{count} autre élément', other: '{count} autres éléments' },
    criteriaLabel: 'Critères',
    met: 'atteint',
    images: { one: '{count} image jointe', other: '{count} images jointes' },
    files: { one: '{count} fichier joint', other: '{count} fichiers joints' },
    questionWaiting: 'Question en attente de ta réponse',
    resumes: 'Reprise {when}',
    settingUp: 'Prépare le worktree · {step}',
    thinking: 'Réfléchit',
    costLabel: 'Coût du ticket',
    launch: 'Lancer',
    launchTitle: 'Lancer {key} ?',
    launchAnyway: 'Lancer quand même',
    stopTests: 'Arrêter',
    test: 'Tester',
    resume: 'Reprendre',
    agentResolves: "L'agent résout",
    sendBack: 'Renvoyer',
    prioritize: 'Passer en tête',
    openAgent: "Ouvrir l'agent",
    removeTitle: 'Supprimer {key} ?',
    removeBody: 'Le ticket et sa description sont supprimés.',
    removeBodyImported: 'Le ticket et sa description sont supprimés. Il ne sera plus importé depuis {service}.',
    // A ticket with images: they are files of its folder, which go with it.
    removeBodyImages: {
      one: 'Le ticket, sa description et son image sont supprimés.',
      other: 'Le ticket, sa description et ses {count} images sont supprimés.',
    },
    removeBodyImportedImages: {
      one: 'Le ticket, sa description et son image sont supprimés. Il ne sera plus importé depuis {service}.',
      other: 'Le ticket, sa description et ses {count} images sont supprimés. Il ne sera plus importé depuis {service}.',
    },
    // A ticket with files (alone or with images): `count` is all its attachments, which go with it.
    removeBodyFiles: {
      one: 'Le ticket, sa description et sa pièce jointe sont supprimés.',
      other: 'Le ticket, sa description et ses {count} pièces jointes sont supprimés.',
    },
    removeBodyImportedFiles: {
      one: 'Le ticket, sa description et sa pièce jointe sont supprimés. Il ne sera plus importé depuis {service}.',
      other: 'Le ticket, sa description et ses {count} pièces jointes sont supprimés. Il ne sera plus importé depuis {service}.',
    },
    removeRunningTitle: 'Supprimer le ticket {key} ?',
    removeRunningBody: 'Son agent est archivé, avec son worktree.',
    removeRunningCleanup:
      "Son agent est archivé et son worktree supprimé, sauf s'il garde des changements non commités ou des commits ni mergés ni poussés.",
    approveTitle: 'Valider {key} ?',
    approveBody: 'Le worktree de son agent est supprimé après le merge.',
  },

  // The form of a ticket, new or edited, in its window.
  form: {
    newTitle: 'Nouveau ticket',
    /** « Harness » of the ticket, from two harnesses found on: the project's by default, which the autopilot follows. */
    engine: 'Harness',
    engineProject: 'Celui du projet ({engine})',
    /** On its button, beside the brand of the project's harness, which says the rest. */
    engineOwn: 'Celui du projet',
    /** Under « Harness »: what it starts from, and that the autopilot follows it. */
    engineHelp: 'Celui du projet : {engine}. Le pilote auto suit ce choix.',
    // « Compte », « Modèle », « Effort » of the ticket (ESC-153): « Auto » says what it gives.
    options: {
      account: 'Compte',
      model: 'Modèle',
      effort: 'Effort',
      auto: 'Auto',
      autoIs: 'Auto · {what}',
      autoFromBoard: 'Auto · {what} (Kanban)',
      autoFromAccount: 'Auto · {what} (compte)',
      autoAccountDetail: 'Le compte courant du harness pour ce projet, au démarrage du ticket.',
      pinnedDetail: 'Ses agents partent sur ce compte et y restent : à sa limite, ils attendent sa remise à zéro.',
      noEffort: '{model} ne prend pas d’effort',
      noEffortHarness: 'Ce harness ne prend pas d’effort',
      accountReset: '« Compte » remis sur « Auto » : {account} ne sert pas ce ticket sur ce harness.',
      modelReset: '« Modèle » remis sur « Auto » : {account} n’a pas {model}.',
      effortReset: '« Effort » remis sur « Auto » : {model} ne prend pas l’effort {effort}.',
    },
    editTitle: 'Modifier {key}',
    addHint: '{key} pour ajouter',
    saveHint: '{key} pour enregistrer',
    discardTitle: 'Abandonner les modifications ?',
    discardBody: 'Ce que tu as saisi dans ce ticket ne sera pas enregistré.',
    discard: 'Abandonner',
    title: 'Titre du ticket',
    descriptionPlaceholder: 'Description (facultative)',
    description: 'Description',
    criteriaPlaceholder: "Critères d'acceptation, un par ligne",
    criteria: "Critères d'acceptation",
    maxLoops: 'Boucles max',
    after: 'Après',
    afterNone: 'Aucun',
    searchKey: 'Rechercher une clé',
    noMatch: 'Aucun ticket pour cette clé',
    // The button over the description that attaches any file (an image or not), as pasting and dropping do.
    attach: 'Joindre un fichier',
    attachTitle: 'Joindre un fichier (ou colle-le, glisse-le dans la fenêtre)',
    attachHelp:
      'Colle ou glisse un fichier dans la fenêtre : les images restent des images du ticket, les autres fichiers sont copiés avec lui.',
    // The files that are no image of the ticket, as chips under its images: copied at once, kept with the ticket.
    files: {
      label: 'Fichiers du ticket',
      tooMany: "{name} n'est pas ajouté : un ticket prend {max} fichiers au plus.",
    },
    // The images pasted or dropped in the window, under the description: the formats and limits of the chat.
    images: {
      label: 'Images du ticket',
      // What its chip says under its name.
      kind: 'Image',
      kindSize: 'Image · {size}',
      // Several images are named with their rank: screenshots are all called image.png.
      ranked: '{name} ({n}/{total})',
      remove: 'Retirer {name}',
      loading: 'Chargement de {name}…',
      missing: "{name} n'est plus dans le dossier du ticket : il en est retiré à l'enregistrement.",
      notImage: "« {name} » n'est pas une image : un ticket prend les images PNG, JPEG, GIF et WebP.",
      totalTooBig: "{name} n'est pas ajoutée : les images d'un ticket sont limitées à {max} en tout.",
      tooMany: "{name} n'est pas ajoutée : un ticket prend {max} images au plus.",
    },
  },

  // The form that sends a ticket back to its agent.
  reject: { comment: 'Ce qui ne va pas', submit: 'Renvoyer' },

  // The report an agent gives when it hands a ticket over: its criteria, its progress, its test launch.
  report: {
    title: 'Bilan des critères',
    /** The mark before a criterion, read by a screen reader. */
    met: 'Atteint',
    unmet: 'Pas encore atteint',
    criterion: 'Critère {n}',
    doneHeading: 'Ce qui a été fait',
    progressLabel: 'Avancement',
    testLaunch: 'Lancement de test',
    prepare: 'Préparation',
    process: 'processus {n}',
    test: 'Tester',
  },
} as const satisfies Tree;
