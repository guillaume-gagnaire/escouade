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
    empty: 'Aucun agent.',
    createAgent: 'Créer un agent',
    archived: 'Archivés ({n})',
    resumeTitle: 'Arrêté par la limite d’usage : reprise automatique {when}',
    resumes: 'Reprise {when}',
    settingUp: 'Préparation…',
    settingUpTitle: 'Préparation du worktree : {step}',
    tokens: '{tokens} tok',
    files: { one: '{count} fich.', other: '{count} fich.' },
    /** The remote control link of an agent, told whole in each state. */
    remote: {
      connected: 'Remote control : connecté (accessible depuis claude.ai et l’app Claude)',
      connecting: 'Remote control : connexion… (accessible depuis claude.ai et l’app Claude)',
      waiting: 'Remote control : en attente de connexion (accessible depuis claude.ai et l’app Claude)',
    },
    terminals: 'Terminaux',
    newTerminal: 'Nouveau terminal',
    noTerminals: 'Aucun terminal ouvert',
    exited: 'terminé',
    noShell: 'Aucun shell détecté (PowerShell 7, Git Bash, WSL). Vérifie les réglages.',
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
      openTerminal: 'Ouvrir un terminal',
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
      'Le processus Claude est arrêté et la conversation est retirée de l’application (la session Claude Code reste sur le disque).',
    deleteWorktree: 'Supprimer aussi le worktree et la branche {branch}',
  },

  // The title bar: the projects' tabs and the window's buttons.
  titleBar: {
    overviewTitle: 'Tous les agents de tous les projets ({shortcut})',
    attention: 'À voir : {names}',
    delta: 'Modifications git non commitées',
    waiting: 'Agents en attente de réponse',
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
  dropdown: { label: '{caption} : {shown}' },

  // Closing a project.
  project: {
    closeTitle: 'Fermer « {name} » ?',
    closeBody:
      "Le projet et ses agents sont retirés de l'application (conversations comprises). Les fichiers et les worktrees sur le disque ne sont pas touchés.",
    closeConfirm: 'Fermer le projet',
  },
} as const satisfies Tree;
