import type { Tree } from '../types';

// The frame of the app: status bar, welcome screen, modals, toasts, updates.
export default {
  /** The window when it has nothing else to show. */
  app: {
    /** `{error}`: what the backend answered, as received. */
    cantStart: 'Impossible de démarrer : {error}',
    noAgent: 'Aucun agent dans ce projet.',
    newAgent: 'Nouvel agent',
  },
  welcome: {
    intro: 'Ajoute un projet pour y lancer des agents Claude Code, suivre leurs questions et ouvrir des terminaux.',
    /** `{command}`: the command that installs it, in code style. */
    claudeMissing: 'Claude Code est introuvable sur ce poste. Installe-le ({command}) ou indique son chemin dans les réglages.',
    addProject: 'Ajouter un projet',
  },
  /** The window that adds a project. */
  newProject: {
    title: 'Nouveau projet',
    folderTitle: 'Dossier du projet',
    pathPlaceholder: 'C:\\chemin\\vers\\le\\projet',
    browse: 'Parcourir…',
    tabPreview: "Aperçu de l'onglet",
    untitled: 'nouveau-projet',
    colorN: 'Couleur {n}',
    firstAgent: 'Créer un premier agent',
    firstAgentDesc: 'Ouvre directement une conversation dans ce projet',
    worktree: 'Un worktree git par agent',
    worktreeDesc: 'Isole le travail de chaque agent et permet de voir ses fichiers modifiés séparément',
    create: 'Créer le projet',
    creating: 'Création…',
    /** What is known of the folder chosen. */
    git: {
      choose: 'Choisis le dossier du projet',
      missing: 'Dossier introuvable',
      repoClean: 'Dépôt git détecté · branche {branch} · propre',
      repoDirty: {
        one: 'Dépôt git détecté · branche {branch} · {count} fichier modifié',
        other: 'Dépôt git détecté · branche {branch} · {count} fichiers modifiés',
      },
      none: 'Aucun dépôt git · un dépôt sera initialisé',
    },
  },
  /** The update downloaded, its notes once installed, and the toasts about it. */
  update: {
    ready: 'Escouade {version} est prête',
    notes: 'Nouveautés d’Escouade {version}',
    saveFirst: {
      one: 'Enregistre d’abord tes fichiers : {count} fichier n’est pas enregistré.',
      other: 'Enregistre d’abord tes fichiers : {count} fichiers ne sont pas enregistrés.',
    },
    working: {
      one: '{count} agent travaille ou attend ta réponse : son tour en cours sera interrompu. S’il travaille sur un ticket, il reprend de lui-même ; sinon, il attend ton prochain message.',
      other:
        '{count} agents travaillent ou attendent ta réponse : leur tour en cours sera interrompu. Les agents de ticket reprennent d’eux-mêmes ; les autres attendent ton prochain message.',
    },
    restartNow: 'Redémarrer maintenant',
    restarting: 'Redémarrage…',
    failed: 'La mise à jour vers {version} n’a pas pu s’installer.',
    notReady: 'La mise à jour vers {version} n’est plus prête : Escouade te la proposera de nouveau une fois téléchargée.',
    installed: 'Escouade {version} est installée.',
    seeNotes: 'Voir les nouveautés',
    /** `{error}`: what the updater answered, as received. */
    downloadFailed: 'Téléchargement de la mise à jour impossible : {error}',
    checkFailed: 'Vérification des mises à jour impossible : {error}',
  },
  /** The question asked when the window is closed with files unsaved. */
  quit: {
    title: 'Quitter Escouade ?',
    unsaved: {
      one: '{count} fichier n’est pas enregistré dans l’éditeur : leurs modifications seront perdues.',
      other: '{count} fichiers ne sont pas enregistrés dans l’éditeur : leurs modifications seront perdues.',
    },
    confirm: 'Quitter quand même',
  },
  /** A file the editor was asked to open, outside the folder of its source. */
  openFile: {
    outsideProject: '{name} est en dehors du dossier du projet.',
    outsideAgent: '{name} est en dehors du dossier de cet agent.',
  },
  /** What an agent has spent. */
  spend: {
    estimateHint: 'Estimation (tarifs publics) pendant que Claude travaille ; coût exact à la fin du tour',
    /** `{pct}`: written as a percentage; `{used}` and `{size}`: counts of tokens. */
    context: 'Contexte : {pct} de la fenêtre du modèle ({used} tokens sur {size})',
  },
  /** The bar at the bottom of the window. */
  status: {
    active: { one: '{count} actif', other: '{count} actifs' },
    waiting: '{n} en attente',
    done: { one: '{count} terminé', other: '{count} terminés' },
    /** `{key}`: the shortcut. */
    nextWaiting: 'Aller au prochain agent en attente ou à voir ({key})',
    procsTitle: 'Processus Claude en cours (avec les outils et serveurs MCP qu’ils lancent)',
    /** `{memory}` and `{cpu}`: the amounts, in code style. */
    procs: '{instances} Claude · {memory} · {cpu} CPU',
    session: 'Session 5 h',
    week: 'Hebdo',
    resetsAt: 'Réinitialisation : {date}',
    sessionUnavailable: 'Quota de session indisponible',
    weekUnavailable: 'Quota hebdomadaire indisponible',
    reset: 'reset {countdown}',
    today: "Aujourd'hui",
    /** The branch against its remote. */
    sync: {
      tracked: 'Suit {upstream} : {behind} à tirer, {ahead} à pousser',
      gone: "La branche suivie {upstream} n'existe plus sur le dépôt distant",
      unpublished: 'Branche pas encore publiée sur le dépôt distant',
      lastFetch: 'Dernier fetch : {when}',
      never: 'jamais',
      publish: 'Publier la branche',
      now: 'maintenant',
      goneShort: 'distante supprimée',
      unpublishedShort: 'non publiée',
    },
    restartIn: 'Redémarrage dans {seconds} s',
    updateReady: 'Mise à jour {version} prête · Redémarrer',
    updating: 'Mise à jour {version}…',
    sound: 'Son des notifications',
    soundOn: 'On',
    soundOff: 'Off',
    /** `{key}`: the shortcut. */
    settings: 'Réglages ({key})',
  },
} as const satisfies Tree;
