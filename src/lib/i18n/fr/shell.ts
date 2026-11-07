import type { Tree } from '../types';

// The frame of the app: status bar, welcome screen, modals, toasts, updates.
export default {
  /** The window when it has nothing else to show. */
  app: {
    /** `{error}`: what the backend answered, as received. */
    cantStart: 'Impossible de démarrer : {error}',
    noAgent: 'Aucun agent dans ce projet.',
    newAgent: 'Nouvel agent',
    projectPrs: 'PR et CI du projet',
  },
  welcome: {
    intro: 'Ajoute un projet pour y lancer des agents, suivre leurs questions et ouvrir des terminaux.',
    /** No harness found: the first start lists each one with the command that installs it. */
    noEngine: 'Aucun harness trouvé',
    /** ESC-128: harnesses found, no account added yet. */
    noAccount: 'Aucun compte',
    noAccountDesc: 'Ajoute un compte : celui déjà connecté sur cette machine, ou un nouveau. Seuls les comptes ajoutés lancent des agents.',
    /** `{where}`: « Réglages › Harness », in bold. */
    noEngineDesc:
      'Escouade fait travailler des agents de code. Installe l’un d’eux, puis indique son chemin dans {where} s’il n’est pas dans le PATH.',
    harnessSettings: 'Réglages › Harness',
    harnessSettingsButton: 'Réglages › Harness…',
    /** `{harness}`: its name. */
    copy: 'Copier la commande de {harness}',
    copied: 'Commande copiée',
    /** The machine's own account of a harness found: signed in (as whom), or not. */
    machine: 'Connecté sur cette machine',
    machineAs: 'Connecté sur cette machine · {email}',
    notConnected: 'Installé, pas connecté',
    searchAgain: 'Chercher à nouveau',
    searched: 'Cherché dans le PATH',
    searchedWsl: 'Cherché dans le PATH et dans WSL ({distro})',
    /** The panel with no project, once a harness and an account are there. */
    title: 'Un premier projet',
    addProject: 'Ajouter un projet…',
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
    /** The folder was a project, closed with its history kept: adding it brings the history back. */
    history: {
      /** `{agents}`, `{tickets}`: the counts, each with its noun; `{date}`: its last activity. */
      found: 'Historique retrouvé : {agents}, {tickets}, dernière activité le {date}',
      fresh: "Partir d'un projet vierge",
      freshDesc: "L'historique reste dans Réglages › Projets, où l'effacer.",
      reopen: 'Rouvrir le projet',
      reopening: 'Réouverture…',
    },
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
  /** The page's process ended while it showed a project (ESC-290): loaded again without it. */
  pageCrash: {
    project: 'L’interface s’est arrêtée en affichant « {project} » : elle a été rechargée sans ce projet.',
    other: 'L’interface s’est arrêtée : elle a été rechargée.',
    reopen: 'Rouvrir',
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
    /** `{error}`: the cause the backend gives, already in the language of the interface (or what the call answered, as received). */
    downloadFailed: 'Téléchargement de la mise à jour impossible : {error}',
    checkFailed: 'Vérification des mises à jour impossible : {error}',
    /** The button of a failed download: the page of the release, where the installer can be taken from. */
    openInBrowser: 'Télécharger dans le navigateur',
  },
  /** The question of the first start: the usage statistics (`telemetry`). */
  telemetry: {
    title: 'Aider à améliorer Escouade ?',
    body: 'Escouade peut envoyer des statistiques d’usage, sans rien qui t’identifie : quelles fonctionnalités servent, avec quelle version d’Escouade et quel système. Jamais tes projets, ton code, tes conversations ni ton nom.',
    detail:
      'Elles partent vers un serveur d’Escouade hébergé en France (Aptabase, open source), sous un identifiant tiré au hasard, et sont gardées 25 mois. Tu peux changer d’avis à tout moment dans Réglages › À propos.',
    more: 'Ce qui est envoyé, en détail',
    yes: 'Partager les statistiques',
    no: 'Non merci',
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
    estimateHint: 'Estimation (tarifs publics) pendant le tour ; coût exact à sa fin',
    /** `{pct}`: written as a percentage; `{used}` and `{size}`: counts of tokens. */
    context: 'Contexte : {pct} de la fenêtre du modèle ({used} tokens sur {size})',
  },
  /** The bar at the bottom of the window. */
  status: {
    /** After the count, in bold: « 4 actifs ». */
    activeWord: { one: 'actif', other: 'actifs' },
    waitingWord: 'en attente',
    doneWord: { one: 'terminé', other: 'terminés' },
    /** A group of quota gauges, as its tooltip and a screen reader name it; with several accounts, the account shown. */
    harnessAccount: '{harness}, {account}',
    /** Beside a harness whose account answers as extra usage, billed apart. */
    overage: 'usage supplémentaire',
    /** `{key}`: the shortcut. */
    nextWaiting: 'Aller à la carte du prochain agent en attente ou à voir ({key})',
    procsTitle: 'Processus des agents en cours (avec les outils et serveurs MCP qu’ils lancent)',
    /** `{memory}` and `{cpu}`: the amounts, in code style. */
    procs: { one: '{count} agent · {memory} · {cpu} CPU', other: '{count} agents · {memory} · {cpu} CPU' },
    /** One line of the tooltip: `{name}`: the agent; `{memory}` and `{cpu}`: the amounts. */
    procRow: '{name} : {memory} · {cpu}',
    today: "Aujourd'hui",
    /** A harness without a quota (OpenCode): its account's cost of the day, in the place of the bars. */
    dayCostTip: '{harness} n’a pas de quota : son compte a coûté {cost} aujourd’hui, sans budget par jour.',
    dayBudgetTip:
      '{harness} n’a pas de quota : son compte a coûté {cost} aujourd’hui, sur un budget de {budget} par jour (atteint, le pilote auto attend minuit).',
    /** After it, the day's cost at midnight at the pace since midnight; `{amount}`: « 3,40 $ ». */
    dayProjectedTip: 'Projection à minuit : {amount}.',
    /** The first line of the branch button's tooltip while an agent open works in a folder of its own. */
    agentBranch: 'Branche de l’agent {agent}, dans son worktree',
    /** After the pull request of the branch shown: how many more it has. */
    morePrs: { one: '+{count} autre', other: '+{count} autres' },
    /** The branch against its remote. */
    sync: {
      tracked: 'Suit {upstream} : {behind} à tirer, {ahead} à pousser',
      gone: "La branche suivie {upstream} n'existe plus sur le dépôt distant",
      unpublished: 'Branche pas encore publiée sur le dépôt distant',
      lastFetch: 'Dernier fetch : {when}',
      never: 'jamais',
      goneShort: 'distante supprimée',
      unpublishedShort: 'non publiée',
      /** The commits to pull and to push, beside their arrows, to a screen reader. */
      behindShort: { one: '{count} commit à tirer', other: '{count} commits à tirer' },
      aheadShort: { one: '{count} commit à pousser', other: '{count} commits à pousser' },
    },
    restartIn: 'Redémarrage dans {seconds} s',
    /** `{restart}`: « Redémarrer », in bold. */
    updateReady: 'Mise à jour {version} prête · {restart}',
    restart: 'Redémarrer',
    updating: 'Mise à jour {version}…',
    sound: 'Son des notifications',
    /** `{state}`: On or Off. */
    soundTip: 'Son des notifications : {state}',
    soundOn: 'On',
    soundOff: 'Off',
  },
} as const satisfies Tree;
