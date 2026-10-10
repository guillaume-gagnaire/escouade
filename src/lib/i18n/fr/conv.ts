import type { Tree } from '../types';

// The conversation with an agent.
export default {
  /** The words that tell what an agent is doing (the header of its conversation). */
  status: { running: 'En cours', waiting: 'Question', idle: 'Prêt', done: 'Terminé', error: 'Erreur' },

  /** The header of the conversation: name, test button, figures, layout. */
  header: {
    renameTitle: "Renommer l'agent",
    renameHint: 'Double-clic pour renommer',
    editorHint: 'Parcourir et éditer les fichiers de cet agent',
    test: 'Tester',
    testIsolaHint: 'Lance les services isola de ce worktree et ouvre la fonctionnalité dans le navigateur',
    testHint: 'Lance le worktree de cet agent et ouvre la fonctionnalité dans le navigateur',
    prepare: 'Préparer le lancement',
    prepareHint: "Demande à l'agent comment lancer son worktree, sur ses propres ports",
    context: 'Contexte',
    contextNow: 'Contexte actuel',
    tokens: 'Tokens',
    duration: 'Durée',
    filesHint: 'Voir les fichiers non commités',
    layout: 'Disposition ({key})',
    layoutClassic: 'Disposition classique',
    layoutSplit: 'Conversation et fichiers côte à côte',
  },

  loadError: 'Impossible de charger la conversation : {error}',
  empty: {
    title: 'Agent prêt',
    /** `{cwd}`: the folder the agent works in, in code style. */
    hint: "Décris la tâche à confier à Claude. L'agent travaille dans {cwd}.",
    claudeMissing: 'Claude Code est introuvable sur ce poste : installe-le ou indique son chemin dans les réglages (⚙).',
  },
  /** The button above the messages drawn, and what a screen reader hears once more are drawn. */
  older: {
    show: { one: 'Afficher le précédent', other: 'Afficher les {count} précédents' },
    drawn: { one: '{count} message précédent affiché', other: '{count} messages précédents affichés' },
  },
  working: 'Claude travaille…',
  newMessages: 'Nouveaux messages',
  archived: { text: 'Agent archivé.', restore: 'Restaurer' },
  stopped: "L'agent s'est arrêté. Envoie un message pour relancer Claude sur la même session.",

  /** « Rechercher dans les conversations » (Ctrl+K). */
  search: {
    title: 'Rechercher dans les conversations',
    hint: 'Cherche dans les messages, les commandes et les fichiers des conversations de tes agents.',
    searching: 'Recherche…',
    none: 'Aucun message ne correspond.',
    capped: 'Les {n} premiers résultats : précise ta recherche.',
    timedOut: {
      one: '{n} résultat, recherche arrêtée au bout de 5 s : précise ta recherche.',
      other: '{n} résultats, recherche arrêtée au bout de 5 s : précise ta recherche.',
    },
    results: { one: '{n} résultat', other: '{n} résultats' },
    agentGone: 'Cet agent n’existe plus.',
    scope: 'Où chercher',
    thisProject: 'Ce projet',
    allProjects: 'Tous les projets',
    archivedAgents: 'Agents archivés',
    resultsLabel: 'Résultats',
    archived: 'archivé',
    keyChoose: 'choisir',
    keyOpen: 'ouvrir',
    keyClose: 'fermer',
  },

  user: {
    images: { one: '{count} image', other: '{count} images' },
    waiting: 'En attente de la préparation…',
    queued: 'transmis pendant le tour',
    queuedHint: 'Claude en tient compte dès sa prochaine étape',
    remote: 'depuis claude.ai',
    remoteHint: 'Envoyé depuis claude.ai ou l’app Claude (remote control)',
  },

  /** What Claude Code passed on to Claude by itself (a background task, a subagent, another session). */
  event: {
    taskCompleted: 'Tâche de fond terminée',
    taskFailed: 'Tâche de fond en échec',
    taskStopped: 'Tâche de fond arrêtée',
    /** The status the CLI reports, when it is none of those. */
    taskStatus: 'Tâche de fond {status}',
    notification: 'Notification',
    fromClaudeCode: 'Message de Claude Code ({source})',
    handbackNamed: 'Rapport du sous-agent « {label} »',
    handback: 'Rapport d’un sous-agent',
    messageNamed: 'Message de « {label} »',
    message: 'Message d’une autre session Claude',
  },

  thinking: { title: 'Réflexion' },

  /** A tool call (its row) and the summary of its result. */
  tool: {
    openFile: 'Ouvrir {path} dans l’éditeur',
    running: 'en cours',
    subTools: { one: '{count} outil', other: '{count} outils' },
  },
  tools: {
    tasks: { one: '{count} tâche', other: '{count} tâches' },
    interrupted: 'interrompu',
    error: 'erreur',
    lines: { one: '{n} ligne', other: '{n} lignes' },
    results: { one: '{n} résultat', other: '{n} résultats' },
    noResults: 'aucun résultat',
    done: 'terminé',
  },
  patch: { more: { one: '… {n} ligne de plus', other: '… {n} lignes de plus' } },

  /** The end of a turn: its card, or the line that separates two turns. */
  turn: {
    failed: "Le tour s'est terminé en erreur",
    resumeAt: 'Reprise automatique {when}',
    cancelResume: 'Annuler la reprise',
    done: 'Tâche terminée',
    tokens: '{tokens} tokens',
    filesEdited: { one: '{count} fichier modifié', other: '{count} fichiers modifiés' },
    editedFiles: 'Fichiers modifiés',
    interrupted: 'Interrompu',
  },

  /** The setup of an agent's new worktree under way. `{step}`: the step running, in code style. */
  setup: {
    running: 'Préparation du worktree · {step} — tes messages partiront une fois terminée.',
    showOutput: 'Voir la sortie',
    outputLabel: 'Sortie de {step}',
    noOutput: 'Pas encore de sortie.',
  },

  /** The code blocks of a reply (`lib/markdown.ts`): the language they have none of, and the copy button once pressed. */
  markdown: { plainText: 'texte', copied: 'Copié ✓' },
} as const satisfies Tree;
