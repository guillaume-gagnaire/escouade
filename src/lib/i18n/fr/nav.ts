import type { Tree } from '../types';

// The sidebar, the title bar, the side panel and the menus.
export default {
  // The sidebar of a project: its agents, its terminals, its folder.
  sidebar: {
    /** What an agent is doing (glossary: « En cours » is « Running » for an agent). */
    status: { running: 'En cours', waiting: 'Question', idle: 'Prêt', done: 'Terminé', error: 'Erreur' },
    viewLabel: 'Vue du projet',
    /** Its right edge (ESC-204), dragged as the rail's. */
    width: 'Largeur de la barre des agents',
    widthTip: 'Tirer pour élargir · double clic : largeur par défaut',
    toReview: '{count} à tester',
    newAgent: 'Nouvel agent',
    newAgentTitle: 'Nouvel agent ({shortcut})',
    /** The menu beside « Nouvel agent », from two harnesses found on: one entry each. */
    newAgentOn: 'Nouvel agent sur {engine}',
    /** The same menu: an agent the other way round from « Un worktree par agent ». */
    newAgentNoWorktree: 'Nouvel agent sans worktree',
    newAgentInWorktree: 'Nouvel agent dans un worktree',
    /** The same menu (ESC-160): an agent whose worktree starts from a branch chosen in the picker. */
    newAgentFrom: 'Nouvel agent depuis une autre branche…',
    /** The brand of an agent's harness (its card, the overview, its header), once agents run on two. */
    engineMark: 'Harness : {engine}',
    empty: 'Aucun agent.',
    createAgent: 'Créer un agent',
    archived: 'Archivés ({n})',
    resumeTitle: 'Arrêté par la limite d’usage : reprise automatique {when}',
    resumes: 'Reprise {when}',
    settingUp: 'Préparation…',
    settingUpTitle: 'Préparation du worktree : {step}',
    /** An agent whose worktree waits for its first message (ESC-156): its card, and what waits for it in its menu. */
    worktreeLater: 'worktree au premier message',
    worktreeLaterTitle: 'Le worktree sera créé au premier message, au nom de l’agent',
    /** The line of a card after a turn: what it changed (2.0 design). */
    changed: { one: '{count} fichier modifié', other: '{count} fichiers modifiés' },
    /** What an agent of a harness counted in credits (Copilot CLI) has spent, on its card. */
    credits: { one: '{n} crédit', other: '{n} crédits' },
    /** The line of the card of an agent whose turn is over while its work goes on: « Tour fini · 1 sous-agent, … ». */
    over: 'Tour fini',
    goes: {
      subagents: { one: '{count} sous-agent', other: '{count} sous-agents' },
      background: { one: '{count} tâche de fond', other: '{count} tâches de fond' },
      driven: { one: '{count} agent piloté', other: '{count} agents pilotés' },
    },
    /** The meta line of a card: a ticket's loop, a ticket to test, an agent never written to. */
    loop: 'boucle {iteration}/{max}',
    toTest: 'à tester',
    noMessage: 'Aucun message',
    /** The choices of the question an agent waits on, answered from its card (2.0 design). */
    answers: 'Réponses à {name}',
    step: 'Question {n} sur {count}',
    inConversation: 'Répondre dans la conversation',
    answered: '{name} : « {choice} »',
    /** The menu « Harness » beside « Nouvel agent » (choices 1, 15, 16 of the 2.0 design). */
    harness: 'Harness',
    projectHarness: 'Celui du projet',
    noAccount: 'Aucun compte',
    addAccount: 'Ajouter un compte…',
    harnessMissing: 'Introuvable : voir Réglages › Harness',
    worktree: 'Worktree',
    newAgentNoWorktreeDetail: 'Dans le dossier du projet, sur sa branche courante',
    newAgentInWorktreeDetail: 'Sur une branche à lui, dans un dossier à part',
    defaultHarness: 'Harness par défaut du projet…',
    /** The remote control link of an agent, told whole in each state. */
    remote: {
      connected: 'Remote control : connecté (accessible depuis claude.ai et l’app Claude)',
      connecting: 'Remote control : connexion… (accessible depuis claude.ai et l’app Claude)',
      waiting: 'Remote control : en attente de connexion (accessible depuis claude.ai et l’app Claude)',
      /** An agent in Claude Code’s view with the remote control on: no interface runs (not started yet, or ended): not reachable. */
      noInterface: 'Remote control : aucune interface lancée, pas joignable pour l’instant (ouvre l’agent pour la lancer)',
      /** Its interface was started without the remote control, which its next launch has. */
      nextLaunch: 'Remote control : au prochain lancement de son interface Claude Code',
      /** Its interface runs but Claude Code did not open the remote control (no claude.ai subscription, network, proxy or organization policy). */
      unavailable:
        'Remote control : pas ouvert par Claude Code (compte sans abonnement claude.ai, réseau, proxy ou règle de l’organisation)',
      /** Off for Escouade, but its interface, still open, stays linked to claude.ai (it is off: no source follows). */
      stillOn:
        'Remote control encore actif dans l’interface ouverte, jusqu’à son prochain lancement (sa commande /remote-control le coupe)',
      /** `{state}`: one of the states above; `{source}`: where the remote control comes from (below). */
      withSource: '{state} · {source}',
      source: {
        hand: 'activé à la main',
        project: 'activé par le réglage du projet',
        global: 'activé par le réglage de tous les agents',
      },
    },
    terminals: 'Terminaux',
    newTerminal: 'Nouveau terminal',
    noTerminals: 'Aucun terminal ouvert',
    exited: 'terminé',
    noShell: 'Aucun shell détecté (PowerShell 7, Git Bash, WSL). Vérifie les réglages.',
    /** In the menu of the « + » of the terminals: opens the list of the agents whose worktree can hold one. */
    inWorktree: 'Dans le worktree de…',
    modified: '~{n} modifiés',
    added: '+{n} ajoutés',
    deleted: '−{n} supprimés',
    /** The tooltip of the footer's changes. */
    changesTitle: '{m} modifiés, {a} ajoutés, {d} supprimés (non commités)',
    noRepo: 'Pas de dépôt git',
    /** The project's folder in the footer: it opens the editor on it. */
    browseFolder: 'Parcourir {path}',
    // The menu of an agent's card.
    menu: {
      duplicate: 'Dupliquer la conversation',
      waitTurn: 'Attends la fin de son tour.',
      restore: 'Restaurer',
      archive: 'Archiver',
      archiveHint: 'garde la conversation',
      prepareLaunch: 'Préparer le lancement',
      // The project's launch commands, to run in its worktree.
      launch: 'Lancer…',
      openTerminal: 'Ouvrir un terminal',
      /** `{engine}`: the agent's harness. */
      toTerminal: 'Vue {engine}',
      toRich: 'Vue Escouade',
      remoteOff: 'Désactiver le remote control',
      remoteOn: 'Activer le remote control',
      remoteOnHint: 'claude.ai, mobile',
      openRemote: 'Ouvrir sur claude.ai',
      copyRemoteLink: 'Copier le lien claude.ai',
      delete: 'Supprimer…',
    },
    archiveTitle: 'Archiver {name} ?',
    archiveBody: 'Son ticket {key} repartira « À faire ».',
    archiveConfirm: 'Archiver',
    deleteTitle: "Supprimer l'agent « {name} » ?",
    deleteBody:
      'Le processus de {engine} est arrêté et la conversation est retirée de l’application (la session {harness} reste sur le disque).',
    deleteWorktree: 'Supprimer aussi le worktree et la branche {branch}',
  },

  // The title bar: the projects' tabs and the window's buttons.
  titleBar: {
    overviewTitle: 'Tous les agents de tous les projets ({shortcut})',
    attention: 'À voir : {names}',
    delta: 'Modifications git non commitées',
    waiting: 'Agents en attente de réponse',
    /** The lock of a private project (« Projet privé »), on its tab and in the settings. */
    private: 'Projet privé : invisible pour les agents des autres projets',
    addProject: 'Ajouter un projet',
    /** The active project's name in the title bar opens its menu. */
    projectMenu: 'Menu du projet {name}',
    search: 'Rechercher',
    /** `{key}`: the shortcut. */
    searchLabel: 'Rechercher dans les conversations ({key})',
    openEditor: 'Ouvrir l’éditeur du projet',
    openEditorTitle: 'Parcourir et éditer les fichiers du projet',
    stats: 'Stats',
    minimize: 'Réduire',
    maximize: 'Agrandir',
    restore: 'Restaurer',
    closeWindow: "Fermer (l'app reste dans la zone de notification)",
    // The menu of a project's tab.
    menu: {
      rename: 'Renommer…',
      renameTitle: 'Renommer le projet',
      worktreeOff: 'Désactiver le worktree par agent',
      worktreeOn: 'Activer le worktree par agent',
      worktreeHint: 'nouveaux agents',
      settings: 'Réglages du projet…',
      openFolder: 'Ouvrir le dossier',
      close: 'Fermer le projet…',
    },
  },

  // The projects' rail (ESC-158): a tile per open project, on the left.
  rail: {
    label: 'Projets',
    list: 'Projets ouverts',
    overview: 'Vue d’ensemble',
    overviewTip: 'Tous les agents de tous les projets',
    add: 'Ajouter un projet',
    addTip: 'Ou rouvrir un projet fermé',
    stats: 'Stats',
    statsTip: 'Coût, tokens et agents de tous les projets',
    settings: 'Réglages',
    /** `{key}`: the shortcut. */
    settingsLabel: 'Réglages ({key})',
    /** The lines of a tile's tooltip. */
    waiting: { one: '{count} agent attend ta réponse', other: '{count} agents attendent ta réponse' },
    running: { one: '{count} agent travaille', other: '{count} agents travaillent' },
    delta: 'Δ {count} · modifications non commitées',
    private: 'Projet privé',
    idle: 'Rien en cours, rien de modifié',
    /** What a tile says to a screen reader, after the name. */
    waitingShort: '{count} en attente',
    runningShort: '{count} au travail',
    privateShort: 'privé',
    /** The pills of the projects out of view whose agents wait: `{names}`, the projects. */
    moreUp: '{names} : {count} en attente, plus haut',
    moreDown: '{names} : {count} en attente, plus bas',
    up: 'Monter',
    down: 'Descendre',
    upKey: 'Alt ↑',
    downKey: 'Alt ↓',
    closeDetail: 'Son historique est gardé : « + » le rouvre',
    /** The rail's right edge (ESC-168). */
    width: 'Largeur de la barre des projets',
    widthTip: 'Tirer pour élargir · double clic : largeur par défaut',
    // The menu of « + », with closed projects.
    addMenu: {
      new: 'Ajouter un projet…',
      newDetail: 'Un dossier, un dépôt à cloner ou un projet vide, par l’assistant',
      closed: 'Projets fermés · leur historique est gardé',
      closedOn: 'Fermé le {date}',
      reopen: 'Rouvrir',
      all: 'Tous les projets fermés…',
      allDetail: 'Réglages › Projets',
    },
  },

  // The side panel's two tabs.
  sidePanel: {
    uncommitted: 'Non commités',
    history: 'Historique',
    // The code hosts' tab (ESC-305): their word (« PR », « MR » on GitLab), and its dot when the CI of the agent's PR failed.
    hosts: '{word} et CI',
    hostsFailed: 'La CI de la {word} #{number} a échoué',
    /** Its × (ESC-204), and how it comes back: by the header of the conversation, as it was opened. */
    close: 'Fermer le panneau',
    reopen: 'Pour le rouvrir : « Fichiers » dans l’en-tête de la conversation',
    reopenSplit: 'Pour le rouvrir : la disposition côte à côte, en haut à droite de la conversation',
  },

  // The context menu's row of colors: « Couleur 3 ».
  menu: { swatch: '{label} {n}' },

  // The picker whose menu opens upwards: « Modèle : Opus ».
  dropdown: {
    label: '{caption} : {shown}',
    search: 'Rechercher',
    none: 'Aucun résultat',
    favorite: 'Ajouter aux favoris',
    unfavorite: 'Retirer des favoris',
    // The headings of the models' menu: the favorites first, then a group by family, « Autres » for those of none.
    favorites: 'Favoris',
    others: 'Autres',
  },

  // Closing a project.
  project: {
    closeTitle: 'Fermer « {name} » ?',
    closeBody:
      "Le projet quitte l'application : ses agents, ses terminaux et ses lancements s'arrêtent. Son historique (conversations, tickets, réglages) est gardé : rouvrir ce dossier le retrouve. Les fichiers et les worktrees sur le disque ne sont pas touchés.",
    closeConfirm: 'Fermer le projet',
    /** The box of the question, unchecked: closing keeps the history unless it is checked. */
    closeErase: 'Effacer aussi son historique (conversations, tickets, réglages)',
  },
} as const satisfies Tree;
