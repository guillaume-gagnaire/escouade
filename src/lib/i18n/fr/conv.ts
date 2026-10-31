import type { Tree } from '../types';

// The conversation with an agent.
export default {
  /** The words that tell what an agent is doing (the header of its conversation). */
  status: { running: 'En cours', waiting: 'Question', idle: 'Prêt', done: 'Terminé', error: 'Erreur' },

  /** The header of the conversation: name, test button, figures, layout. */
  header: {
    renameTitle: "Renommer l'agent",
    renameHint: 'Double-clic pour renommer',
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
    filesClose: 'Fermer les fichiers',
    layout: 'Disposition ({key})',
    layoutClassic: 'Disposition classique',
    layoutSplit: 'Conversation et fichiers côte à côte',
    /** « Vue <harness> »: `{engine}`, the agent's harness (Claude Code, Codex…). */
    toTerminal: 'Vue {engine}',
    toTerminalHint: 'Faire tourner l’agent dans l’interface de {engine} elle-même, dans un terminal',
    toRich: 'Vue Escouade',
    toRichHint: 'Revenir à la conversation d’Escouade (la session continue)',
    /** The two views of the agent, in segments: the one shown named, the other one an icon. */
    view: 'Vue de l’agent',
    rich: 'Escouade',
    /** « … »: the agent's menu, the one of its card. */
    more: 'Actions de l’agent',
    /** The sandbox of a harness (Codex), beside the agent's path; `{meaning}`: what its mode lets it do. */
    sandbox: 'Bac à sable',
    sandboxLabel: 'Bac à sable. {meaning}',
    sandboxDefault: 'Ses commandes tournent dans un bac à sable.',
  },

  /** A message refused for want of an account of the agent's harness (choice 19): `{harness}`, its name. */
  noAccount: {
    body: 'Cet agent ne démarre pas tant qu’aucun compte {harness} n’est ajouté : celui déjà connecté sur cette machine, ou un nouveau. Ton message reste dans le champ.',
  },

  /** « Vue Claude Code »: the agent in Claude Code's own interface. */
  terminal: {
    /** `{engine}`: the agent's harness, everywhere in this group; `{speaker}`: its agents' name (Claude). */
    starting: 'Démarrage de {engine}…',
    exited: '{engine} s’est arrêté.',
    /** It stopped as soon as it started (ESC-106): the reason is in its output, above. `{code}`: its exit code. */
    exitedAtOnce: '{engine} s’est arrêté dès son lancement (code {code}). Regarde sa sortie ci-dessus, puis relance-le.',
    exitedAtOnceNoCode: '{engine} s’est arrêté dès son lancement. Regarde sa sortie ci-dessus, puis relance-le.',
    /** It found no conversation to resume: the interface starts again, on a new session. */
    sessionLost: 'Session {speaker} introuvable : l’interface repart en nouvelle session.',
    restart: 'Relancer',
    failed: '{engine} n’a pas démarré : {error}',
    imageTooBig: '{name} dépasse {max} : {engine} ne la recevra pas.',
    /** The amber band above the terminal while Codex or Copilot CLI asks whether to trust the agent's folder. */
    trust: {
      title: '{engine} demande s’il peut faire confiance au dossier de l’agent.',
      codex: 'Réponds dans le terminal : sans confiance, il s’arrête. Escouade a déjà approuvé ses hooks pour ce seul process.',
      copilot: 'Réponds dans le terminal : Escouade ne répond pas à ta place et ne tape rien tant que la question est posée.',
      cursor:
        'Réponds dans le terminal (« Trust this workspace ») : Escouade ne répond pas à ta place et ne tape rien tant que la question est posée.',
    },
    /**
     * The band of Cursor's view (ESC-49): what Escouade reads of its transcript, what it does not know, and the two
     * conversations Cursor keeps apart. `{title}`: `limitedTitle`, in bold.
     */
    limited:
      '{title} : Escouade voit tes messages, les réponses de Cursor et la fin de chaque tour, mais pas ses permissions ni ses questions en attente, ni le résultat de ses outils, ni les tokens : regarde le terminal. Cette conversation est celle de Cursor : Escouade la relie à celle de l’agent.',
    limitedTitle: 'Suivi limité',
    /** The green band of OpenCode's view (choice 37): its conversation stays up to date. `{title}`: `keepsTitle`, in bold. */
    keeps:
      '{title} : la conversation, la carte, les notifications et le coût restent à jour pendant que tu travailles ici. Revenir à la vue Escouade n’arrête que l’interface.',
    keepsTitle: 'Escouade garde le fil',
  },

  /** An agent whose process started without Escouade’s tools, which its next start gives (ESC-91). */
  escouadeNext: {
    title: 'Outils d’Escouade au prochain démarrage',
    text: 'Son process a démarré sans les outils d’Escouade, que « Les agents peuvent piloter Escouade » lui donne maintenant : relance-le pour qu’il les ait. Sa session est reprise et sa conversation reste.',
    /** A turn, a question or the setup of its worktree under way: it ends by itself. */
    busy: 'Il pourra être relancé quand il aura fini ce qu’il fait.',
    /**
     * Nothing of that, but something runs in the background (a command, a monitor, a subagent, a workflow): restarting it
     * would stop it, so « Relancer » is refused, and a command or a dev server does not end by itself.
     */
    busyBackground:
      'Il a encore des tâches de fond (commande, surveillance, sous-agent, workflow) : le relancer les arrêterait. Une commande ou un serveur de dev ne finit pas seul : arrête-la d’abord, depuis sa vue (« Arrêter »), ou attends qu’elle ait fini.',
    restart: 'Relancer',
  },

  loadError: 'Impossible de charger la conversation : {error}',
  empty: {
    title: 'Agent prêt',
    /** `{cwd}`: the folder the agent works in, in code style. */
    hint: "Décris la tâche à confier à {engine}. L'agent travaille dans {cwd}.",
    /** `{engine}`: the agent's harness. */
    engineMissing: '{engine} est introuvable sur ce poste : installe-le ou indique son chemin dans Réglages › Harness (⚙).',
  },
  /** The button above the messages drawn, and what a screen reader hears once more are drawn. */
  older: {
    show: { one: 'Afficher le précédent', other: 'Afficher les {n} précédents' },
    drawn: { one: '{n} message précédent affiché', other: '{n} messages précédents affichés' },
  },
  working: '{engine} travaille…',
  newMessages: 'Nouveaux messages',
  archived: { text: 'Agent archivé.', restore: 'Restaurer' },
  stopped: "L'agent s'est arrêté. Envoie un message pour relancer {engine} sur la même session.",

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
    queuedHint: '{engine} en tient compte dès sa prochaine étape',
    remote: 'depuis claude.ai',
    remoteHint: 'Envoyé depuis claude.ai ou l’app Claude (remote control)',
    saidAloud: 'Dit à voix haute',
  },

  /** A message held before it goes (`outbox.svelte.ts`). `{s}`: seconds left. */
  held: {
    delay: 'Envoi dans {s} s',
    turnEnd: 'Attend la fin du tour',
    turnEndHint: '{engine} le recevra au début de son prochain tour',
    sendNow: 'Envoyer maintenant',
    edit: 'Modifier',
    cancel: 'Annuler',
  },

  /** The files an agent sent (Claude Code's SendUserFile, ESC-171): its caption, its images, its other files. */
  sent: {
    /** The card's label: `{engine}`, who sent them (« Claude »). */
    by: 'Envoyé par {engine}',
    images: { one: '{count} image', other: '{count} images' },
    files: { one: '{count} fichier', other: '{count} fichiers' },
    reveal: 'Afficher dans le dossier',
    revealName: 'Afficher {name} dans le dossier',
    openName: 'Ouvrir {name}',
    missing: 'Fichier introuvable',
    missingHint: 'Effacé ou déplacé depuis son envoi',
    fenced: 'Fichier d’un projet privé, non affiché',
    big: 'Image trop lourde pour l’aperçu',
    loading: 'Chargement…',
    /** A thumbnail, as a button: it opens the viewer. */
    view: 'Voir {name} en grand',
    /** The viewer of the images. */
    viewer: 'Images envoyées',
    prev: 'Image précédente',
    next: 'Image suivante',
    close: 'Fermer (Échap)',
    counter: '{n} sur {count}',
    /** An image of a message, by a path the agent wrote, that cannot be shown. */
    imageMissing: 'Image introuvable : {src}',
    /** An image of the web in a message: not loaded until asked (its address could carry a secret out). */
    remoteOf: 'image de {host}',
    remoteShow: 'Afficher l’image',
  },

  /** What Claude Code passed on to Claude by itself (a background task, a subagent, another session). */
  event: {
    taskCompleted: 'Tâche de fond terminée',
    taskFailed: 'Tâche de fond en échec',
    taskStopped: 'Tâche de fond arrêtée',
    /** The status the CLI reports, when it is none of those. */
    taskStatus: 'Tâche de fond {status}',
    notification: 'Notification',
    wake: 'Réveil',
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
    more: { one: 'Afficher la suite ({count} caractère)', other: 'Afficher la suite ({count} caractères)' },
  },
  tools: {
    tasks: { one: '{count} tâche', other: '{count} tâches' },
    interrupted: 'interrompu',
    error: 'erreur',
    lines: { one: '{n} ligne', other: '{n} lignes' },
    results: { one: '{n} résultat', other: '{n} résultats' },
    noResults: 'aucun résultat',
    done: 'terminé',
    /** The task tools of Claude Code: the line of a task made, and of one changed (`status`: below). */
    taskCreate: 'Tâche : {subject}',
    taskUpdate: 'Tâche #{id} : {status}',
    taskStatus: { pending: 'à faire', inProgress: 'en cours', completed: 'terminée', deleted: 'supprimée', changed: 'modifiée' },
  },
  patch: { more: { one: '… {n} ligne de plus', other: '… {n} lignes de plus' } },

  /** The end of a turn: its card, or the line that separates two turns. */
  turn: {
    failed: "Le tour s'est terminé en erreur",
    resumeAt: 'Reprise automatique {when}',
    cancelResume: 'Annuler la reprise',
    signInTitle: 'Connexion à refaire',
    signIn:
      'Le compte {account} doit se reconnecter : Claude Code n’a pas pu rafraîchir sa session. L’agent reprendra seul après la connexion.',
    signInAction: 'Se reconnecter',
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
    /** Before the first message of an agent whose worktree waits for it (ESC-156). */
    later: 'Le worktree sera créé au premier message, au nom de l’agent.',
  },

  /** The code blocks of a reply (`lib/markdown.ts`): the language they have none of, and the copy button once pressed. */
  markdown: { plainText: 'texte', copied: 'Copié ✓' },
} as const satisfies Tree;
