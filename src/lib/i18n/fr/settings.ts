import type { Tree } from '../types';

// The settings window and the settings of a project.
export default {
  // The tabs of the window: their name and what they set (under the title, in the header).
  tabs: {
    app: { label: 'Application', desc: 'Thème, zoom, langue de l’interface et textes rédigés' },
    claude: { label: 'Harness', desc: 'Exécutables et versions de chaque harness, et ce qui vaut pour tous' },
    accounts: {
      label: 'Comptes',
      desc: 'Les comptes de chaque harness, les réglages de leurs nouveaux agents, et ce qui se passe à la limite',
    },
    plugins: { label: 'Plugins', desc: 'Ce qu’ils ajoutent à Escouade, d’où ils viennent et ce qu’ils peuvent faire' },
    mcp: { label: 'Serveur MCP', desc: 'Ce que Claude et les agents peuvent faire dans Escouade, et ce qu’ils ont fait' },
    notifications: { label: 'Notifications', desc: 'Alertes visuelles et sonores' },
    projects: { label: 'Projets', desc: 'Réglages propres à chaque projet' },
    board: { label: 'Kanban', desc: 'Pilote auto et tickets validés' },
    integrations: {
      label: 'Intégrations',
      desc: 'Sources du Kanban (Jira, Trello, GitHub, Linear, ClickUp, GitLab, Azure Boards) et hébergeurs de code',
    },
    terminals: { label: 'Terminaux', desc: 'Shells disponibles dans les terminaux intégrés' },
    network: {
      label: 'Réseau',
      desc: 'Proxy HTTP(S) et certificats TLS pour Claude Code, les intégrations et les mises à jour',
    },
    voice: { label: 'Voix', desc: 'Dictée et conversation vocale, sur ta machine ou en ligne' },
    about: { label: 'À propos', desc: 'Version et données locales' },
  },

  // The window itself.
  modal: {
    // The headings of the tabs, after the general ones.
    groupProject: 'Par projet',
    groupSystem: 'Système',
    changed: 'Modifié, pas encore enregistré',
    saved: 'Réglages enregistrés',
    noProject: 'Aucun projet ouvert.',
  },

  // What keeps the draft from being saved.
  problem: {
    projectName: 'Le projet « {name} » doit garder un nom.',
    command: 'Chaque commande de lancement de « {name} » demande un nom et une ligne de commande.',
    worktrees: 'Le dossier des worktrees ou un préfixe de branche de « {name} » n’est pas valide.',
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
    /** In Réglages › Projets › Lancement, after the icon « + ». */
    addLaunch: 'Ajouter une commande',
  },

  // The « Application » tab.
  app: {
    display: 'Affichage',
    theme: 'Thème',
    themeDesc: 'Système suit le thème de Windows, macOS ou Linux, et ses changements. Appliqué tout de suite, barre de titre comprise.',
    themeSystem: 'Système',
    themeLight: 'Clair',
    themeDark: 'Sombre',
    // « Couleurs » (PLG-7): Escouade's, or a plugin's theme.
    colors: 'Couleurs',
    colorsDesc:
      'Celles d’Escouade, ou celles d’un thème apporté par un plugin, en clair comme en sombre. Ce qu’un thème ne donne pas garde la couleur d’Escouade.',
    colorsEscouade: 'Escouade',
    colorsNone: 'Aucun thème installé. Un plugin peut en apporter un : ses couleurs et sa palette de projets.',
    colorsFind: 'Voir les thèmes',
    colorsGone: 'Le thème choisi n’est plus là : son plugin ({plugin}) est désactivé ou désinstallé. Escouade a repris ses couleurs.',
    colorsDropped: {
      one: 'Une valeur de ce thème est écartée : ce n’est pas un jeton ou une couleur permise.',
      other: '{count} valeurs de ce thème sont écartées : ce ne sont pas des jetons ou des couleurs permises.',
    },
    zoom: 'Zoom de l’interface',
    zoomDesc:
      'Le texte, les espacements et les icônes de toute l’application grandissent ensemble, terminaux et éditeur compris. Appliqué et enregistré tout de suite.',
    /** The shortcuts of the zoom, as written on this system. */
    zoomKeys: 'Aussi au clavier : {in} pour agrandir, {out} pour réduire, {reset} pour revenir à 100 %.',
    zoomTooBig: 'Trop grand pour la taille de la fenêtre',
    /** The zoom chosen is too big for the window as it is now. */
    zoomHeld: 'La fenêtre est trop petite pour {zoom} : l’interface est à {shown} tant qu’elle ne grandit pas.',
    /** A step of the zoom: « 125 % ». */
    percent: '{n} %',
    terminalFontSize: 'Taille du texte des terminaux',
    editorFontSize: 'Taille du texte de l’éditeur',
    fontSizeHint: 'en pixels, {n} par défaut',
    fontSizeDesc: 'De 9 à 24, avant le zoom de l’interface, qui s’y ajoute.',
    language: 'Langue',
    uiLanguage: 'Langue de l’interface',
    /** The system's language, named in the interface's: « Système (Français) ». */
    system: 'Système ({lang})',
    claudeLanguage: 'Langue des textes rédigés par les agents',
    sameAsUi: 'Comme l’interface (par défaut)',
    claudeLanguageHelp:
      'Messages de commit et descriptions de pull request proposés, commentaires publiés dans Jira, Trello, GitHub, Linear, ClickUp, GitLab et Azure Boards, consignes données aux agents (par défaut, le message de commit proposé reprend la langue des derniers commits du dépôt).',
    /** A language named in the interface's (the choices name each in itself: `LANG_NAMES`). */
    langName: { fr: 'Français', en: 'Anglais' },
    /** « Modèle des textes rédigés »: the harness the texts written by a model go to, among those found. */
    writing: 'Textes rédigés',
    writer: 'Modèle des textes rédigés',
    writerHelp:
      'Nom donné à un nouvel agent, messages de commit et descriptions de pull request proposés, commandes du worktree remplies automatiquement. Seuls les harness installés sont proposés.',
    /** For a harness whose models are those of the providers connected to it (OpenCode). */
    writerModel: 'Modèle',
    writerModelHelp:
      'Un modèle des fournisseurs connectés à OpenCode, toujours donné : sans lui, OpenCode en choisirait un lui-même. Sans aucun modèle, pas de texte rédigé.',
    writerModelDefault: 'Par défaut ({model})',
    writerModelNone: 'Par défaut (lu au premier texte)',
  },

  // The « Harness » tab (Claude Code, Codex, Copilot CLI, Cursor CLI, OpenCode: « Harness » in French too).
  claude: {
    /** The models of a harness that lists those of its providers (OpenCode). */
    models: {
      label: 'Modèles',
      desc: 'Ceux des fournisseurs connectés à {engine}. Les favoris (★) viennent en premier dans le menu du composer, puis les autres par fournisseur.',
      search: 'Rechercher un modèle',
      none: 'Aucun modèle lu pour l’instant : ils le sont au premier agent de ce harness.',
    },
    /** A harness not found on this machine. */
    states: {
      absent: 'Introuvable',
      /** Beside its name, before its version (2.0 design): « Prêt · 2.1.301 ». */
      ready: 'Prêt',
      notConnected: 'Aucun compte',
      tooOld: 'Trop ancienne',
      unsupported: 'Non prise en charge',
    },
    /** The group of what holds for every harness, after each one's. */
    allHarnesses: 'Tous les harness',
    allHarnessesNote:
      'Le modèle, l’effort et le mode des nouveaux agents se règlent compte par compte, dans Réglages › Comptes, où chaque compte se connecte.',
    /** Under « Remote control pour tous les agents », in the group of the harness that has it. */
    remoteOnly: '{harness} seulement : les autres harness n’ont pas de remote control.',
    install: 'Installation',
    installDesc: 'Introuvable sur ce poste : installe-le avec cette commande, puis indique son chemin ici s’il n’est pas dans le PATH.',
    executable: 'Exécutable',
    path: "Chemin de l'exécutable",
    pathHint: 'vide = détection automatique',
    pathFound: 'claude (trouvé dans le PATH)',
    pathMissing: 'introuvable — indique le chemin de claude.exe',
    /** Under the path: where its `claude` is. */
    located: 'Trouvé : {path}',
    locatedWsl: 'Trouvé dans WSL ({distro}) : {path}',
    nat: 'Le réseau de WSL est en NAT : le serveur MCP d’Escouade et le suivi de la vue Claude Code ne joignent pas l’app. Passe-le en mode miroir (networkingMode=mirrored sous [wsl2] dans %UserProfile%\\.wslconfig), puis lance wsl --shutdown.',
    notLocated:
      'Introuvable. Indique le chemin de claude.exe, ou de Claude Code installé dans WSL : ~/.local/bin/claude, /home/<utilisateur>/…, ou \\\\wsl.localhost\\<distribution>\\….',
    notLocatedHere: 'Introuvable. Indique le chemin de claude.',
    /** Another harness than Claude Code: its path is not set in this tab. */
    notLocatedOther: 'Introuvable.',
    /** The version found of the saved path, alone: no word of the tested versions (ESC-159). */
    version: 'Version',
    versionUnknown: 'Version inconnue : claude ne l’a pas donnée.',
    versionNotConnected: 'Aucun compte connecté : ajoute ou connecte un compte de ce harness dans l’onglet Comptes.',
    processes: 'Processus',
    autoResume: 'Reprise automatique après la limite d’usage',
    autoResumeDesc: '« continue » envoyé une fois le quota réinitialisé',
    idleStop: 'Arrêter les processus des agents inactifs après',
    idleStopHint: 'minutes, 0 = jamais',
    idleStopLabel: 'Arrêter les processus des agents inactifs après (minutes)',
    terminalView: 'Vue des nouveaux agents',
    terminalViewDesc:
      'Escouade : la conversation d’Escouade. Terminal du harness : l’interface du harness elle-même (celle de Claude Code…), dans un terminal. Chaque agent change de vue depuis son en-tête ; les agents de ticket, et ceux d’un harness sans vue terminal, restent dans la vue d’Escouade.',
    terminalViewEscouade: 'Escouade',
    terminalViewHarness: 'Terminal du harness',
    /** Escouade's update of Claude Code (ESC-133). */
    selfUpdate: 'Mettre à jour Claude Code automatiquement',
    selfUpdateDesc:
      'Escouade lance claude update à son démarrage puis chaque jour, à part de ses agents. Les process de Claude Code qu’il lance ne se mettent plus à jour seuls : leur arrêt pouvait couper une installation à mi-chemin.',
    selfUpdateStates: {
      waiting: 'Pas encore lancée depuis le démarrage d’Escouade.',
      running: 'Mise à jour en cours…',
      upToDate: 'À jour ({version}).',
      upToDateNoVersion: 'À jour.',
      updated: 'Mis à jour en {version} (depuis la {from}).',
      failed: 'Échec de la mise à jour : {line}',
      off: 'Coupée : Claude Code n’est plus mis à jour, ni par Escouade ni par lui-même. Lance claude update de temps en temps.',
      userOff: 'Coupée dans Claude Code lui-même ({line}) : Escouade ne le met pas à jour non plus.',
    },
    todoTools: 'Les agents tiennent une liste de tâches',
    todoToolsDesc:
      'Donne aux agents l’outil de liste de tâches de Claude Code : s’ils la tiennent, Escouade montre leur avancée dans le bandeau « Plan » (le modèle n’est pas obligé de s’en servir). S’applique aux agents lancés ensuite.',
    attachments: 'Pièces jointes',
    attachmentsNote:
      'Les images, les PDF et les textes dans leurs limites partent dans le message. Tout autre fichier, ou un fichier trop gros pour le message, est joint par son chemin : l’agent le lit avec ses outils.',
    attachMax: 'Taille maximale d’un fichier joint',
    attachMaxHint: 'Mo, 0 = aucune limite',
    attachMaxLabel: 'Taille maximale d’un fichier joint (Mo)',
    attachWarn: 'Avertir au-delà de',
    attachWarnHint: 'Mo, 0 = jamais',
    attachWarnLabel: 'Avertir pour un fichier joint de plus de (Mo)',
    attachOrigin: 'Joindre le fichier d’origine',
    attachOriginDesc:
      'Un fichier déposé est joint là où il est, sans copie : l’agent le lit à son emplacement. Désactivé, Escouade en garde toujours une copie. Les tickets et la vue Claude Code copient toujours.',
    attachBigText: 'Texte de plus de 256 Ko',
    attachBigTextDesc: 'Un fichier texte trop gros pour le message est joint par son chemin, ou refusé.',
    attachBigTextPath: 'Joint par son chemin',
    attachBigTextRefuse: 'Refusé',
    attachKeep: 'Purger les copies après',
    attachKeepHint: 'jours, 0 = tant que leur propriétaire existe',
    attachKeepLabel: 'Purger les copies après (jours)',
    /** A harness that does not tell what its turns cost (Codex): Escouade estimates it. */
    cost: 'Coût',
    costEstimated:
      'Estimé d’après les tokens et les tarifs publics : {engine} ne donne pas le coût de ses tours. Il est écrit « ≈ » partout.',
    /** What it cannot do, beside: joined by « · », the first one capitalised. */
    costNoPdf: 'pas de PDF joint',
    costNoBackground: 'pas de tâches de fond',
    /** The group of the remote control of every agent, after the MCP server’s. */
    remoteTitle: 'Remote control',
    remoteAll: 'Remote control pour tous les agents',
    remoteAllDesc:
      'Chaque agent de chaque projet, existant ou nouveau (agents des tickets compris), est accessible depuis claude.ai et l’app Claude. Escouade lance tout de suite le process claude de chacun, qui reste lancé. La session apparaît sur claude.ai sous le nom « projet · agent » (pour un agent de ticket, son nom vient du ticket). En vue Claude Code, une interface déjà ouverte ne le prend qu’à son prochain lancement. Pour mettre un agent à part : clic droit, « Désactiver le remote control ».',
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
    soundOnDesc: 'Question d’un agent, fin de tour',
    soundTest: 'Tester le son',
    soundTestButton: 'Tester le son',
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
    /** A download of an update that failed, kept in sight while it lasts. */
    updateTrouble: 'Mise à jour {version}',
    updateTroubleDesc: 'Téléchargement impossible : {error}',
    /** An update found, by « Rechercher une mise à jour » or the automatic check (ESC-250): downloading, then ready. */
    updateFound: 'Escouade {version} disponible',
    updateDownloading: 'Téléchargement…',
    updateReady: 'Escouade {version} prête',
    updateReadyDesc: 'Elle s’installe au redémarrage d’Escouade.',
    /** What a screen reader hears of it as it comes. */
    updateFoundLive: 'Escouade {version} disponible : téléchargement…',
    updateReadyLive: 'Escouade {version} prête : redémarre pour l’installer.',
    seeNotes: 'Voir les nouveautés',
    hideNotes: 'Masquer les nouveautés',
    credits: 'Crédits et licences',
    creditsDesc: 'Piper, eSpeak NG, ONNX Runtime, whisper.cpp, et la licence et le crédit de chaque voix',
    creditsOpen: 'Ouvrir',
    /** `{list}`: the harnesses found, by name. */
    enginesFound: 'Harness détectés : {list}',
    noEngine: 'Aucun harness trouvé',
    checking: 'Recherche…',
    check: 'Rechercher une mise à jour',
    noUpdate: 'Aucune mise à jour disponible.',
    autoUpdate: 'Installer les mises à jour automatiquement',
    autoUpdateDesc: 'Escouade redémarre d’elle-même quand aucun agent ne travaille et que tout est enregistré.',
    localData: 'Données locales',
    telemetry: 'Statistiques d’usage',
    telemetryDesc:
      'Les fonctionnalités utilisées, la version d’Escouade et le système, sous un identifiant tiré au hasard : rien qui t’identifie, ni tes projets ni ton code.',
    telemetryMore: 'En détail',
    site: 'Site web',
    siteDesc: 'escouade.dev : la documentation, les nouveautés et la feuille de route',
    siteOpen: 'Ouvrir',
    siteOpenLabel: 'Ouvrir le site',
    discord: 'Discord',
    discordDesc: 'Poser une question, signaler un problème, suivre ce qui arrive',
    discordJoin: 'Rejoindre',
    discordJoinLabel: 'Rejoindre le Discord',
    support: 'Soutenir Escouade',
    supportDesc: 'Escouade est gratuite et open source : un café aide à la faire avancer',
    supportOpen: 'Offrir un café',
  },

  // The « Projets » tab: one project's own settings.
  project: {
    identity: 'Identité',
    nameLabel: 'Nom du projet',
    engine: 'Harness par défaut',
    colorDesc: 'Elle teinte toute la fenêtre quand le projet est ouvert.',
    engineDesc: 'Celui des nouveaux agents et des tickets du projet, sauf s’ils en choisissent un autre.',
    colorN: 'Couleur {n}',
    /** ESC-167: what the project's tile on the rail shows. */
    lettersLabel: 'Lettres de la tuile',
    lettersDesc: 'Une à trois lettres ou chiffres, ou un emoji. Vide : les initiales du nom.',
    logoLabel: 'Logo',
    logoDesc: 'PNG, JPEG, WebP, SVG ou ICO, 1 Mo au plus. Il remplace les lettres ; Escouade en garde une copie.',
    logoChoose: 'Choisir une image…',
    logoRemove: 'Retirer',
    /** « Libellé sous la tuile » (ESC-168). */
    labelLabel: 'Libellé sous la tuile',
    labelDesc:
      'Le nom du projet, ou un texte plus court (12 caractères au plus). Élargie, la barre le montre à droite de la tuile, allumé ou non.',
    labelText: 'Texte du libellé',
    logoTitle: 'Logo du projet',
    logoFilter: 'Images',
    tilePreview: 'Tuile du projet',
    /** The group of the remote control of the project’s agents, after its identity. */
    remoteTitle: 'Remote control',
    remote: 'Remote control pour les agents de ce projet',
    remoteDesc:
      'Chaque agent de ce projet, existant ou nouveau (agents des tickets compris), est accessible depuis claude.ai et l’app Claude. Escouade lance tout de suite le process claude de chacun, qui reste lancé. La session apparaît sur claude.ai sous le nom « projet · agent » (pour un agent de ticket, son nom vient du ticket). En vue Claude Code, une interface déjà ouverte ne le prend qu’à son prochain lancement. Pour mettre un agent à part : clic droit, « Désactiver le remote control ».',
    /** Under the switch, shown on and disabled: the setting for all agents covers the project already. */
    remoteGlobal: 'Activé pour tous les projets dans Réglages › Harness',
    /** The group of « Projet privé », after the remote control. */
    privateTitle: 'Confidentialité',
    private: 'Projet privé',
    privateDesc:
      'Les agents des autres projets ne le voient pas par le serveur MCP d’Escouade : ni dans les listes, ni par son nom, ses tickets ou ses agents. Les siens ne voient que lui. Toi, Claude hors Escouade et les vues de l’app voyez tout.',
    /** Under « Projet privé »: what keeps its files from the other projects' agents, harness by harness (ESC-140). */
    fenceTitle: 'Ses fichiers',
    fenceDesc:
      'Les agents des autres projets ne lisent ni n’écrivent son dossier, ses worktrees, ses données dans le dossier d’Escouade ni ses sessions de harness. Les siens ne lisent rien de tout cela pour les autres projets, ni le reste du dossier d’Escouade. Un garde-fou est une règle du harness, qu’une commande peut contourner ; une barrière tient pour toutes ses commandes.',
    fenceKind: { guard: 'Garde-fou', barrier: 'Barrière' },
    fenceHow: {
      claude:
        'Refusés à ses outils de fichiers et aux commandes qu’il reconnaît (cat, sed…), pas à un script qui ouvre un fichier lui-même. Avec son bac à sable activé (macOS, Linux, WSL2), toutes ses commandes sont refusées aussi.',
      codex: 'Son bac à sable les refuse à toutes ses commandes, sauf en mode Bypass, qui n’en a pas.',
      opencode: 'Refusés à ses outils et aux commandes qui les nomment. Il n’a pas de bac à sable.',
      copilot:
        'Escouade refuse d’office ce qu’il demande sur ces chemins. Rien n’est garanti en mode Bypass, ni en vue Copilot CLI, où ses demandes ne passent pas par Escouade.',
      cursor:
        'Escouade refuse d’office ses commandes qui nomment ces chemins, et ce qu’il soumet sur eux, même en mode Bypass. Ses lectures et ses écritures ne demandent rien, même hors du dossier de l’agent : Cursor ne les fait pas passer par Escouade, qui ne peut donc pas les garder.',
      vibe: 'Refusés à ses outils de lecture et d’écriture par la config de l’agent, et Escouade refuse d’office ce qu’il demande sur eux. En mode Bypass, Vibe reste dans son agent ask pour que ces refus tiennent. Une commande de sa liste (cat, ls…) qui les lit sans demander n’est pas gardée.',
      agy: 'Refusés à la lecture (et donc à l’écriture) par les règles de l’agent, que son mode Bypass ne lève pas, et Escouade refuse d’office ce qu’il demande sur eux. Ses commandes qui les liraient ne sont pas gardées.',
      gemini:
        'Refusés à ses outils de fichiers et à ses commandes qui les nomment par les règles de l’agent (--policy), que son mode Bypass (yolo) ne lève pas, et Escouade refuse d’office ce qu’il demande sur eux. Un script qui ouvre un fichier lui-même n’est pas gardé ; pas encore essayé avec un compte.',
    },
    fenceNote:
      'Une barrière pour tous les harness demanderait un compte système par projet : Escouade ne le propose pas, les harness et leurs comptes n’y suivraient pas. Un agent déjà lancé prend ce réglage avant son prochain tour (son process repart, sa conversation reste) ; une vue terminal, à son prochain lancement.',
    worktreePerAgent: 'Un worktree par agent',
    worktreePerAgentDesc: 'Isole les modifications de chaque nouvel agent dans sa propre branche.',
    worktreeAtFirstMessage: 'Créer le worktree au premier message, au nom de l’agent',
    worktreeAtFirstMessageDesc:
      'Le premier message nomme l’agent, puis sa branche et son dossier prennent ce nom avant la préparation. Éteint, le worktree est créé tout de suite, en agent-N.',
    /** « Branche de départ des nouveaux worktrees » (ESC-160): where a new agent’s worktree starts, its base. */
    worktreeBase: 'Branche de départ des nouveaux worktrees',
    worktreeBaseDesc:
      'Le worktree d’un nouvel agent part de cette branche, qui est aussi sa base : « Intégrer », son merge et sa PR vont vers elle. « Nouvel agent » peut en choisir une autre pour un agent.',
    worktreeBases: {
      head: 'Branche actuelle du dossier',
      target: 'Branche cible du Kanban',
      /** Before a branch is chosen. */
      branch: 'Une branche…',
    },
    /** The chip of the branch named, `{branch}`: it opens the picker again. */
    worktreeBaseBranch: '{branch}',
    worktreeBaseChoose: 'Branche de départ : {branch}. Choisir une autre branche',
    /** What each choice means, under the chips. `{branch}`: the board’s target or the branch named. */
    worktreeBaseHeadAt: 'Ils partent de la branche sur laquelle est le dossier du projet au moment de leur création.',
    worktreeBaseTargetAt: 'Ils partent de {branch}, mise à jour depuis le distant si elle n’est qu’en retard.',
    worktreeBaseBranchAt:
      'Ils partent de {branch}, mise à jour depuis le distant si elle n’est qu’en retard. Une branche distante seule passe par la branche locale créée pour la suivre ; une branche disparue fait partir de la branche du dossier, l’agent le dit.',
    worktreeDir: 'Dossier des worktrees',
    worktreeDirDesc:
      'Relatif au dossier du projet (../app-worktrees, .worktrees), à ton dossier personnel (~/worktrees/app) ou complet. Vide : .claude/worktrees, comme claude --worktree. Les worktrees déjà créés restent où ils sont ; hors du projet, la vue Claude Code demande d’abord ta confiance pour chacun.',
    /** `{path}`: where the next one goes, `<nom>` standing for its name. */
    worktreeDirAt: 'Le prochain : {path}',
    worktreeDirItself: 'C’est le dossier du projet lui-même.',
    worktreeDirContains: 'Ce dossier contient celui du projet : choisis-en un à côté (../app-worktrees) ou dedans (.worktrees).',
    worktreeDirGit: 'Ce dossier est dans un dossier .git.',
    worktreeDirPartial: 'Écris un chemin relatif au dossier du projet, ou un chemin complet (C:\\…).',
    agentPrefix: 'Préfixe des branches des agents',
    agentPrefixDesc: 'Branche d’un nouvel agent : {prefix}<nom>. Vide : son nom seul.',
    ticketPrefix: 'Préfixe des branches des tickets',
    ticketPrefixDesc: 'Branche d’un ticket, celle de sa PR : {prefix}<clé>. Vide : sa clé seule.',
    /** `{at}`: the characters `@{`, which a text of the catalog cannot hold. */
    prefixInvalid:
      'Aucun nom de branche ne commence ainsi : ni espace, ni « .. », « ~ », « ^ », « : », « ? », « * », « [ », « \\ » ou « {at} », ni « - » ou « / » au début, ni « // », ni une partie qui commence par « . » ou finit par « .lock ».',
    /** Stands for the name of the next agent in `worktreeDirAt`. */
    nextName: '<nom>',
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
    /** The same, after its icon (2.0 design). */
    fillAuto: 'Remplir automatiquement',
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
    leftovers: 'Worktrees restés',
    leftoversDesc:
      'Supprime ceux des agents archivés, après leurs commandes de démontage, sauf ceux qui gardent des changements non commités ou des commits ni mergés ni poussés.',
    leftoversButton: 'Nettoyer',
    leftoversRunning: 'Nettoyage…',
    leftoversNone: 'Aucun worktree à supprimer.',
    leftoversRemoved: { one: '1 worktree supprimé.', other: '{count} worktrees supprimés.' },
    leftoversKept: { one: '1 gardé :', other: '{count} gardés :' },
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
      "Le projet quitte l'application : ses agents, ses terminaux et ses lancements s'arrêtent. Son historique (conversations, tickets, réglages) est gardé : rouvrir ce dossier le retrouve. Les fichiers et les worktrees sur le disque ne sont pas touchés.",
    closeButton: 'Fermer le projet…',
  },

  // The projects closed with their history kept (under the « Projets » tab).
  /** The « Projets » tab, for every project: what a project added starts with. */
  newProjects: {
    title: 'Nouveaux projets',
    private: 'Nouveaux projets privés',
    privateDesc: 'Chaque projet ajouté est privé dès le départ. Ceux qui existent gardent leur réglage.',
  },

  closed: {
    title: 'Projets fermés',
    note: 'Leur historique est gardé : ajouter un projet dans leur dossier le retrouve.',
    /** `{size}`: what the history takes on the disk; `{date}`: its last activity. */
    detail: '{size} · dernière activité le {date}',
    erase: "Effacer l'historique…",
    eraseTitle: "Effacer l'historique de « {name} » ?",
    eraseBody:
      'Ses conversations, ses tickets et ses réglages sont effacés pour de bon. Les fichiers et les worktrees sur le disque ne sont pas touchés.',
    eraseConfirm: "Effacer l'historique",
    /** Ouvre la fenêtre d'ajout sur le dossier du projet fermé et sur cet historique-là. */
    reopen: 'Rouvrir',
    /** Un fichier de projet fermé que l'application ne sait pas lire : il ne peut qu'être effacé, lui seul. */
    unreadable: 'Historique illisible',
    eraseFile: 'Effacer',
    /** `{file}` : le nom du fichier (`abc123.json`). */
    eraseFileTitle: 'Effacer le fichier « {file} » ?',
    eraseFileBody:
      "Ce fichier, que l'application ne peut pas lire, est effacé pour de bon. Les autres historiques, les fichiers et les worktrees sur le disque ne sont pas touchés.",
    eraseFileConfirm: 'Effacer le fichier',
  },

  // The commands run in a project's worktrees (their setup, their teardown), in order.
  steps: {
    /** The keys that move a command, as the system writes Alt (`alt`: « Alt+ », or macOS's sign). */
    upKeys: '{alt}↑',
    downKeys: '{alt}↓',
    altMac: '⌥',
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
  // The project’s .escouade.yml (ESC-146): what it sets, its commands waiting, the settings kept on this machine.
  shared: {
    title: '.escouade.yml',
    /** What the group says of the file, by its state. */
    none: 'Ce dépôt n’a pas de .escouade.yml. Exporte les réglages de ce projet pour les partager avec ton équipe : le fichier se commite avec le code.',
    read: {
      one: 'Réglages lus dans .escouade.yml (version {version}) : {count} réglage défini par le fichier.',
      other: 'Réglages lus dans .escouade.yml (version {version}) : {count} réglages définis par le fichier.',
    },
    error: '.escouade.yml illisible, rien n’en est appliqué : {text}',
    /** A warning of the file, `{line}` its line. */
    warningAt: 'Ligne {line} : {text}',
    warningsTitle: 'À vérifier dans le fichier',
    pendingTitle: 'Commandes en attente de ton accord',
    pendingDesc:
      'Ces commandes viennent de .escouade.yml. Escouade ne les lance pas tant que tu ne leur as pas fait confiance : lis-les en entier. Si le fichier les change, il te le redemandera.',
    trust: 'Faire confiance à ces commandes',
    trusted: 'Commandes approuvées : elles s’appliquent au projet.',
    /** The keys of the commands, as the list names them. */
    keys: {
      setup: 'Préparation des worktrees',
      teardown: 'Démontage des worktrees',
      launch: 'Lancement',
      test: 'Commande de tests',
    },
    export: 'Exporter vers .escouade.yml',
    exportDesc:
      'Écrit les réglages partageables de ce projet dans le fichier, à la racine du dépôt (commentaires et clés inconnues gardés). Rien de personnel : ni nom, ni couleur, ni compte, ni remote control, ni projet privé.',
    exported: '.escouade.yml écrit. Commite-le pour le partager.',
    /** The marks of a setting. */
    fromFile: 'Défini par .escouade.yml',
    fromFilePending: 'Défini par .escouade.yml, en attente de ton accord',
    local: 'Réglage local de cette machine : il passe devant .escouade.yml.',
    writeKey: 'Écrire dans .escouade.yml',
    useFile: 'Reprendre la valeur du fichier',
    written: 'Écrit dans .escouade.yml.',
    /** On the project’s board, while its commands wait. */
    banner: 'Les commandes de .escouade.yml attendent ton accord : elles ne tournent pas encore.',
    bannerButton: 'Les lire',
  },
} as const satisfies Tree;
