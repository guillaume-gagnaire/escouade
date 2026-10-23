import type { Tree } from '../types';

// The sidebar, the title bar, the side panel and the menus.
export default {
  // The sidebar of a project: its agents, its terminals, its folder.
  sidebar: {
    /** What an agent is doing (glossary: « En cours » is « Running » for an agent). */
    status: { running: 'En cours', waiting: 'Question', idle: 'Prêt', done: 'Terminé', error: 'Erreur' },
    viewLabel: 'Vue du projet',
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
    worktreeLater: '⎇ au premier message',
    worktreeLaterTitle: 'Le worktree sera créé au premier message, au nom de l’agent',
    tokens: '{tokens} tok',
    files: { one: '{count} fich.', other: '{count} fich.' },
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
    noRepo: 'Pas de dépôt git',
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

  // The side panel's two tabs.
  sidePanel: { uncommitted: 'Non commités', history: 'Historique' },

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
