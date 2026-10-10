import type { Tree } from '../types';

// The settings window and the settings of a project.
export default {
  // The tabs of the window: their name and what they set (under the title, in the header).
  tabs: {
    app: { label: 'Application', desc: 'Langue de l’interface et des textes rédigés par Claude' },
    claude: { label: 'Claude Code', desc: 'Exécutable, modèle et permissions par défaut' },
    accounts: { label: 'Comptes Claude', desc: 'Les comptes sur lesquels partent les agents, enregistrés à chaque changement' },
    notifications: { label: 'Notifications', desc: 'Alertes visuelles et sonores' },
    projects: { label: 'Projets', desc: 'Réglages propres à chaque projet' },
    board: { label: 'Kanban', desc: 'Pilote auto et tickets validés' },
    integrations: { label: 'Intégrations', desc: 'Jira, Trello et GitHub Issues' },
    terminals: { label: 'Terminaux', desc: 'Shells disponibles dans les terminaux intégrés' },
    network: {
      label: 'Réseau',
      desc: 'Proxy HTTP(S) et certificats TLS pour Claude Code, les intégrations et les mises à jour',
    },
    about: { label: 'À propos', desc: 'Version et données locales' },
  },

  // The window itself.
  modal: {
    changed: 'Modifié, pas encore enregistré',
    saved: 'Réglages enregistrés',
    noProject: 'Aucun projet ouvert.',
  },

  // What keeps the draft from being saved.
  problem: {
    projectName: 'Le projet « {name} » doit garder un nom.',
    command: 'Chaque commande de lancement de « {name} » demande un nom et une ligne de commande.',
  },

  // « Remplir automatiquement »: what is said when Claude has read the project.
  suggest: {
    nothing: "Claude n'a trouvé aucune commande à lancer pour ce projet.",
    proposed: {
      one: "1 commande proposée : relis-la avant d'enregistrer.",
      other: "{count} commandes proposées : relis-les avant d'enregistrer.",
    },
    /** One command was left out, for what it would hide. */
    proposedLeftOne: {
      one: "1 commande proposée, 1 écartée (caractères invisibles ou trop longue) : relis-la avant d'enregistrer.",
      other: "{count} commandes proposées, 1 écartée (caractères invisibles ou trop longue) : relis-les avant d'enregistrer.",
    },
    /** Several were left out: `refused` of them. */
    proposedLeftMany: {
      one: "1 commande proposée, {refused} écartées (caractères invisibles ou trop longues) : relis-la avant d'enregistrer.",
      other: "{count} commandes proposées, {refused} écartées (caractères invisibles ou trop longues) : relis-les avant d'enregistrer.",
    },
  },

  // A shell, and what is asked of a folder, in a project's commands.
  fields: {
    shell: 'Shell',
    /** A shell the draft names that this machine does not have: « bash (introuvable) ». */
    shellNotFound: '{name} (introuvable)',
    subfolder: 'Sous-dossier',
    addCommand: '+ Ajouter une commande',
  },

  // The « Application » tab.
  app: {
    language: 'Langue',
    uiLanguage: 'Langue de l’interface',
    /** The system's language, named in the interface's: « Système (Français) ». */
    system: 'Système ({lang})',
    claudeLanguage: 'Langue des textes rédigés par Claude',
    sameAsUi: 'Comme l’interface (par défaut)',
    claudeLanguageHelp:
      'Messages de commit et descriptions de pull request proposés, commentaires publiés dans Jira, Trello et GitHub, consignes données aux agents (par défaut, le message de commit proposé reprend la langue des derniers commits du dépôt).',
    /** A language named in the interface's (the choices name each in itself: `LANG_NAMES`). */
    langName: { fr: 'Français', en: 'Anglais' },
  },

  // The « Claude Code » tab.
  claude: {
    executable: 'Exécutable',
    path: "Chemin de l'exécutable",
    pathHint: 'vide = détection automatique',
    pathFound: 'claude (trouvé dans le PATH)',
    pathMissing: 'introuvable — indique le chemin de claude.exe',
    newAgents: 'Nouveaux agents',
    defaultModel: 'Modèle par défaut',
    defaultEffort: 'Effort par défaut',
    defaultMode: 'Mode de permission par défaut',
    processes: 'Processus',
    autoResume: 'Reprise automatique après la limite d’usage',
    autoResumeDesc: '« continue » envoyé une fois le quota réinitialisé',
    idleStop: 'Arrêter les processus Claude inactifs après',
    idleStopHint: 'minutes, 0 = jamais',
    idleStopLabel: 'Arrêter les processus Claude inactifs après (minutes)',
    todoTools: 'Les agents tiennent une liste de tâches',
    todoToolsDesc:
      'Donne aux agents l’outil de liste de tâches de Claude Code : Escouade peut alors montrer leur avancée dans le bandeau « Plan ». S’applique aux agents lancés ensuite.',
  },

  // The « Notifications » tab.
  notifications: {
    notifyFor: 'Me prévenir pour',
    /** Under the types of notification; the system names the place the app shows in (the Dock, the taskbar). */
    noteMac: 'Désactivé : ni notification système ni carillon pour ce type ; l’onglet, la carte et le Dock signalent toujours l’agent.',
    noteOther:
      'Désactivé : ni notification système ni carillon pour ce type ; l’onglet, la carte et la barre des tâches signalent toujours l’agent.',
    questions: 'Questions et autorisations',
    done: 'Tâches terminées',
    errors: 'Erreurs',
    tickets: 'Tickets (prêt à tester, bloqué)',
    channels: 'Canaux',
    /** `os`: « macOS » or « Windows ». */
    system: "Notifications {os} quand l'app n'est pas au premier plan",
    systemLabel: 'Notifications système',
    sound: 'Son',
    soundOn: 'Son activé',
    soundOnDesc: 'Question de Claude, fin de tour',
    soundTest: 'Tester le son',
    soundTestButton: '▶ Tester',
  },

  // The « Terminaux » tab.
  terminals: {
    paths: 'Chemins',
    detected: 'Détectés : {list}',
    none: 'aucun',
    auto: 'vide = auto',
    wsl: 'Distribution WSL',
    wslHint: 'vide = distribution par défaut',
  },

  // The « Réseau » tab.
  network: {
    proxy: 'Proxy',
    proxyNote:
      "Le proxy est transmis aux processus Claude Code, à la lecture des quotas, aux intégrations et aux mises à jour. Il s'applique aux agents au prochain (re)démarrage de leur processus.",
    proxyUrl: 'Proxy HTTP(S)',
    proxyUrlHint: 'ex. http://utilisateur:motdepasse@proxy:3128',
    proxyUrlNone: 'aucun',
    exclusions: 'Exclusions',
    exclusionsHint: 'NO_PROXY, séparées par des virgules',
    proxyTerminals: 'Appliquer aussi le proxy aux terminaux intégrés',
    certificates: 'Certificats',
    certificatesNote:
      "Les intégrations, les quotas et les mises à jour font confiance aux certificats reconnus et à ceux installés sur ce système (celui d'un proxy d'entreprise, le plus souvent). Claude Code, lui, a sa propre liste.",
    insecureTls: 'Ignorer la vérification des certificats TLS',
    insecureTlsDesc:
      "Pour un proxy qui déchiffre le trafic avec un certificat non reconnu (erreur « UnknownIssuer »). S'applique aux intégrations, aux quotas, aux mises à jour, et aux processus Claude Code (au prochain démarrage de leur processus) avec les commandes que lancent les agents (npm, node…). À réserver à un réseau de confiance : une connexion interceptée, jetons compris, ne serait plus détectée.",
  },

  // The « À propos » tab.
  about: {
    claudeFound: 'Claude Code détecté',
    claudeMissing: 'Claude Code introuvable',
    checking: 'Recherche…',
    check: 'Rechercher une mise à jour',
    noUpdate: 'Aucune mise à jour disponible.',
    autoUpdate: 'Installer les mises à jour automatiquement',
    autoUpdateDesc: 'Escouade redémarre d’elle-même quand aucun agent ne travaille et que tout est enregistré.',
    localData: 'Données locales',
  },

  // The « Projets » tab: one project's own settings.
  project: {
    identity: 'Identité',
    nameLabel: 'Nom du projet',
    colorN: 'Couleur {n}',
    worktreePerAgent: 'Un worktree par agent',
    worktreePerAgentDesc: 'Isole les modifications de chaque nouvel agent dans sa propre branche.',
    copy: 'Fichiers copiés dans les worktrees',
    copyDesc: 'Seuls ceux que git ignore, jamais commités ; un motif par ligne : .env* à la racine, **/.env* partout.',
    commit: 'Commit',
    commitDesc: 'Direct : Escouade propose un message, tu le relis et tu commites toi-même.',
    commitModes: {
      agent: "Rédigé par l'agent",
      direct: 'Direct, avec un message proposé',
    },
    worktrees: 'Worktrees',
    worktreesNote:
      "Variables disponibles : ESCOUADE_PROJECT_DIR (le projet), ESCOUADE_WORKTREE_DIR, ESCOUADE_BRANCH, et les ports réservés d'un ticket (ESCOUADE_PORT_BASE, ESCOUADE_PORT_END). Un échec est signalé dans la conversation de l'agent.",
    fill: 'Remplir automatiquement',
    fillButton: '✦ Remplir automatiquement',
    filling: 'Claude lit le projet…',
    fillWorktreesDesc:
      "Claude lit le projet (manifestes, lockfiles, README…) sans rien modifier et propose les commandes. Relis-les avant d'enregistrer.",
    fillLaunchDesc:
      "Claude lit le projet (manifestes, scripts, docker-compose, README…) sans rien modifier et propose les commandes à lancer. Relis-les avant d'enregistrer.",
    proposedWorktrees: 'Commandes de worktree proposées',
    proposedWorktreesNote:
      "Proposition de Claude : relis chaque commande en entier. Elles remplacent les deux listes et, une fois enregistrées, tournent seules : la préparation à l'ouverture de chaque nouveau worktree, le démontage avant sa suppression.",
    proposedLaunch: 'Commandes de lancement proposées',
    proposedLaunchNote: 'Proposition de Claude : relis chaque commande en entier. Elles remplacent celles de la liste.',
    ignore: 'Ignorer',
    replace: 'Remplacer les commandes',
    noCommands: 'Aucune commande.',
    worktreeRoot: 'la racine du worktree',
    projectFolder: 'le dossier du projet',
    setupTitle: "À l'ouverture d'un worktree",
    setupDesc: "Dans l'ordre, avant le premier message de son agent : dépendances, code généré… Les messages attendent la fin.",
    teardownTitle: 'Avant sa suppression',
    teardownDesc: 'Ce que la préparation a créé hors du worktree (base de données, conteneurs…) ; souvent rien.',
    launch: 'Lancement',
    launchNote:
      'Chaque commande tourne dans son propre terminal, en lecture seule. Lance-les depuis la section « Lancement » de la barre latérale.',
    commandN: 'Commande {n}',
    namePlaceholder: 'ex. Front',
    commandPlaceholder: 'ex. npm run dev',
    subfolderHint: '(vide = dossier du projet)',
    subfolderPlaceholder: 'ex. apps/web',
    noCommandsYet: "Aucune commande pour l'instant.",
    danger: 'Zone sensible',
    close: 'Fermer le projet',
    closeDesc:
      "Le projet et ses agents sont retirés de l'application, conversations comprises. Les fichiers et les worktrees sur le disque ne sont pas touchés.",
    closeButton: 'Fermer le projet…',
  },

  // The commands run in a project's worktrees (their setup, their teardown), in order.
  steps: {
    setup: {
      group: 'Commande de préparation {n}',
      add: 'Ajouter une commande de préparation',
    },
    teardown: {
      group: 'Commande de démontage {n}',
      add: 'Ajouter une commande de démontage',
    },
    commandPlaceholder: 'ex. npm ci',
    subfolderTitle: 'Sous-dossier du worktree (vide : sa racine)',
    subfolderPlaceholder: 'sous-dossier',
    /** `keys`: the shortcut, as the system writes it. */
    up: 'Monter ({keys})',
    upN: 'Monter la commande {n}',
    down: 'Descendre ({keys})',
    downN: 'Descendre la commande {n}',
    deleteN: 'Supprimer la commande {n}',
  },
} as const satisfies Tree;
