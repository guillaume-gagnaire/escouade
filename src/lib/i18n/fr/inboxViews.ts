// La boîte « À toi » (ESC-513, docs/INBOX.md § 5, § 10.2, § 11) : ce qu’Escouade y verse de lui-même, des vues sur
// ce qui attend déjà ailleurs, et le résumé de ses notifications.
export default {
  // La sorte d’une vue, en tête de sa ligne.
  kind: {
    permission: 'Autorisation',
    question: 'Question',
    plan: 'Plan',
    ticketReview: 'Ticket à valider',
    ticketBlocked: 'Ticket bloqué',
    decision: 'Décision proposée',
    docs: 'Docs à partager',
    review: 'Relecture',
    handoff: 'Agent passé',
    teamAccept: 'Ticket à accepter',
  },
  /** Ce qu’une demande bloquante dit de son agent. */
  stopped: 'l’agent s’est arrêté',
  /** The tooltip of the box's tile: the views it counts besides the requests and the agents stopped. */
  tileViews: {
    one: '{count} autre chose à régler (ticket, décision, docs, relecture)',
    other: '{count} autres choses à régler (tickets, décisions, docs, relectures)',
  },
  /** Le titre d’une vue qui n’en a pas. */
  docsTitle: { one: '{count} fichier des docs à partager', other: '{count} fichiers des docs à partager' },
  planTitle: 'Approuver le plan',
  prTitle: '{word} #{number} · {title}',
  since: 'depuis {since}',
  until: 'jusqu’à {when}',
  partial: 'sans tous ses critères',
  conflict: 'conflit',
  /** A ticket of its own target (ESC-552), on its line. */
  to: 'vers {branch}',
  // Les gestes, ceux de la source.
  act: {
    allow: 'Autoriser',
    always: 'Toujours autoriser',
    deny: 'Refuser',
    answer: 'Répondre',
    inConversation: 'Répondre dans la conversation',
    approvePlan: 'Approuver le plan',
    approve: 'Valider',
    reject: 'Renvoyer à l’agent',
    rejectSend: 'Renvoyer',
    rejectLabel: 'Ce qu’il faut changer',
    rejectPlaceholder: 'Ce qui ne va pas, ce qu’il faut refaire…',
    resume: 'Reprendre',
    resolve: 'L’agent résout',
    dismiss: 'Annuler',
    retarget: 'Viser {branch}',
    accept: 'Accepter',
    share: 'Partager',
    openPr: 'Ouvrir la {word}',
    agent: 'Aller à l’agent',
    board: 'Voir dans le Kanban',
    docs: 'Ouvrir dans Docs',
    test: '▶ Tester',
    take: 'Prendre',
  },
  denyLabel: 'Un mot pour l’agent (facultatif)',
  answerLabel: 'Ta réponse',
  /** « Plus tard » et ses heures (§ 8.4). */
  later: {
    button: 'Plus tard',
    back: 'Remettre dans la boîte',
    blocking: 'Ton agent s’est arrêté pour cette demande : elle attend ta réponse, elle ne se reporte pas.',
  },
  /** Sous les gestes d’une vue : ce qu’ils font. */
  note: {
    request: 'Ta réponse part à {agent}, comme depuis sa conversation.',
    ticket: 'Les mêmes gestes que sa carte du Kanban : valider le merge selon les réglages du projet.',
    decision: 'Acceptée, elle prend son numéro dans le registre des décisions, comme depuis Docs.',
    docs: 'Partager publie les changements des docs, comme le bouton « Partager » de l’onglet Docs.',
    review: 'La relecture se fait chez l’hébergeur : Escouade l’a lue dans la dernière liste des PR.',
    teamAccept:
      'Son texte vient de {name} : accepté, le pilote auto peut le lancer avec tes droits et tes comptes. Lis-le d’abord dans le Kanban ; changé ensuite par un collègue, il revient ici.',
    handoff:
      'Escouade récupère la branche {branch}, fait un worktree, applique le travail en cours, puis l’agent repart ici sur ton harness et ton compte, avec un dossier de passation.',
  },
  /** An agent a teammate passes (TS-17). */
  handoffTitle: '{name} te passe {key} · {title}',
  /** A ticket assigned to the user with a teammate's text (ESC-645). */
  teamAcceptTitle: '{key} · {title} · texte de {name}',
  settled: 'Réglé ailleurs : la boîte s’est mise à jour.',
  /** Ce que l’Overseer recommande d’une demande qu’il te laisse (ESC-646) : jamais une réponse. */
  advice: {
    allow: 'L’Overseer te recommande d’autoriser',
    deny: 'L’Overseer te recommande de refuser',
    none: 'L’Overseer te laisse la réponse',
    note: 'C’est toi qui réponds : il n’accorde que des lectures dans le projet de l’agent.',
  },
  /** Ce que le chef d’escouade en a dit (ESC-557) : pris en charge, ou coché comme réglé. */
  mark: {
    group: 'Le chef s’en occupe',
    claimed: '{name} s’en occupe · {when}',
    settled: 'Réglé par {name} · {when}',
    settledLine: 'Réglé par {name} : {note}',
    note: '« {note} »',
    reopen: 'Remettre dans la boîte',
    claimedNote: 'Ça ne compte plus comme t’attendant. Tu peux toujours la régler toi-même.',
    settledNote: 'Elle est sortie de ta boîte ; elle y revient d’elle-même si sa source change encore.',
    confirmTitle: '{name} est en train de la vérifier',
    confirmBody: 'Sa note : « {note} ». La régler toi-même quand même ?',
  },
  /** Le réglage du résumé, dans Réglages › Notifications. */
  summary: {
    title: 'Boîte « À toi »',
    label: 'Résumé de la boîte « À toi »',
    labelDesc:
      'Jamais une alerte par demande : un résumé de ce que tes agents attendent de toi sans s’arrêter, ni carillon ni clignotement. Les demandes de permission et les questions gardent leurs notifications.',
    every30: 'Un résumé, toutes les 30 min au plus',
    hourly: 'Un résumé par heure',
    never: 'Jamais (le compteur seul)',
  },
} as const;
