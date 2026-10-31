import type { Tree } from '../types';

// The accounts of every harness.
export default {
  /** The user's own account, as long as they have not renamed it (its name is saved in the language of its first save). */
  principal: 'Principal',

  // The « Comptes » tab of the settings: a group per harness.
  tab: {
    /** In the place of the gauges, for a harness without a quota (OpenCode): what the account spent today. */
    today: 'Aujourd’hui',
    list: 'Comptes',
    /** The accounts of one harness; `{harness}`: « Claude Code ». */
    group: 'Comptes {harness}',
    order:
      'Les nouveaux agents partent sur le premier compte actif de leur harness dont la 5 h est sous les seuils « Pause au-delà du quota » et « Changer de compte à » et dont la semaine tient son rythme, dans cet ordre.',
    name: 'Nom',
    folder: 'Dossier',
    executable: 'Exécutable',
    executableDefault: 'Celui des réglages',
    active: 'Actif',
    lastActive: 'Il faut au moins un compte actif par harness.',
    up: 'Monter {name}',
    down: 'Descendre {name}',
    signIn: 'Se connecter…',
    signInAgain: 'Se reconnecter…',
    /** A Cursor or Mistral Vibe account other than the machine's: its API key given again (ESC-43, ESC-267). */
    changeKey: 'Changer la clé…',
    /** A harness that tells Escouade no quota (Cursor CLI, Mistral Vibe): its own page; `{site}`: « cursor.com ». */
    usage: 'Usage sur {site}',
    usageHint:
      '{harness} ne donne pas son usage à Escouade : son tableau de bord le montre (l’usage inclus du mois, la remise à zéro). Une limite atteinte arrête le tour sur une carte qui le dit.',
    /** Antigravity CLI (ESC-271): its tokens told, its quota in its interface alone. */
    usageHintInterface:
      '{harness} ne donne à Escouade que les jetons de ses tours : son quota (par groupe de modèles, hebdomadaire) se lit par /usage dans son interface, et {site} dit ce que chaque forfait inclut. Une limite atteinte arrête le tour sur une carte qui le dit ; l’agent réessaie seul une heure plus tard, ou passe sur un autre compte selon tes réglages.',
    /** Gemini CLI (ESC-275): its tokens told, its usage in its interface alone (/stats), its page for a key. */
    usageHintGemini:
      '{harness} ne donne à Escouade que les jetons de ses tours : son usage se lit par /stats dans son interface, et {site} le montre pour une clé d’API. Une limite atteinte arrête le tour sur une carte qui le dit ; l’agent réessaie seul quand Gemini le dit (une heure plus tard pour son quota du jour), ou passe sur un autre compte selon tes réglages.',
    /** Mistral Vibe (ESC-267): its cost told, its quota not. */
    usageHintCost:
      '{harness} donne à Escouade le coût de chaque tour, pas de quota : {site} montre l’usage et la facture du compte. Une limite de débit arrête le tour sur une carte qui le dit, et l’agent reprend seul une minute plus tard.',
    remove: 'Supprimer',
    add: 'Ajouter un compte…',
    /** A harness installed without an account (ESC-128): nothing starts on it. */
    none: 'Aucun compte',
    noneHint: 'Aucun agent ni ticket ne démarre sur {harness} avant qu’un compte soit ajouté.',
    addTo: 'Ajouter un compte {harness}…',
    /** The Principal of a harness: its usual folder, as it is. */
    machine: 'Compte de la machine',
    /** What the account's new agents go with (ESC-128). */
    defaults: 'Nouveaux agents',
    defaultModel: 'Modèle',
    defaultEffort: 'Effort',
    defaultMode: 'Mode',
    harnessDefault: 'Celui du harness',
    harnessDefaultNamed: 'Celui du harness ({name})',
    reserved: 'Réservé aux projets',
    reservedAll: 'Tous les projets',
    /** For a harness without a quota (OpenCode), which goes through the providers' APIs. */
    budget: 'Budget par jour ($)',
    budgetNone: 'Aucun',
    budgetHint:
      'Atteint, le pilote auto se met en pause jusqu’à minuit, comme au quota épuisé. Vide : aucun budget. Un modèle local ne coûte rien.',
    budgetInvalid: 'Le budget par jour est un montant en dollars, positif.',
    /** Un projet fermé (son historique est gardé) auquel le compte reste réservé : coché, jusqu'à ce qu'on le décoche ou que l'historique soit effacé. */
    closedProject: '{name} (fermé)',
    /** `{names}` : les projets, chacun entre guillemets (`quoted`) et énumérés comme dans une phrase. */
    noAccount: {
      one: 'Aucun compte actif de son harness par défaut ne sert {names} : ses agents et ses tickets ne démarrent pas.',
      other: 'Aucun compte actif de leur harness par défaut ne sert {names} : leurs agents et leurs tickets ne démarrent pas.',
    },
    quoted: '« {name} »',
    reservedHint:
      'Coche des projets pour garder ce compte à eux seuls (les projets d’un client, par exemple) : les autres n’y partent jamais.',
    stillUsed: { one: 'Le compte sert encore à {count} agent.', other: 'Le compte sert encore à {count} agents.' },
    state: {
      checking: 'Vérification…',
      connected: 'Connecté · {email}',
      connectedNoEmail: 'Connecté',
      notConnected: 'Pas connecté',
      /** Principal without a claude.ai sign-in: most likely run with an API key. */
      principalNotConnected: 'Pas connecté par un compte claude.ai (clé d’API ?)',
      inactive: 'Inactif',
    },
    removeMachineConfirm: {
      title: 'Retirer le compte de la machine « {name} » ?',
      body: 'Les nouveaux agents ne partiront plus dessus. {harness} reste connecté sur cette machine : le retirer ne déconnecte rien ({path}).',
      confirm: 'Retirer',
    },
    removeConfirm: {
      title: 'Supprimer le compte « {name} » ?',
      body: 'Les nouveaux agents ne partiront plus dessus. Son dossier reste sur le disque, avec sa connexion et ses conversations : {path}',
      confirm: 'Supprimer',
    },
  },

  // « Ajouter un compte… » and « Se connecter… ».
  modal: {
    addTitle: 'Ajouter un compte {harness}',
    harness: 'Harness',
    /** What is added (ESC-128): the machine's account of the harness, or a new one. */
    kind: 'Compte',
    machine: 'Le compte de la machine',
    machineConnected: '{harness} est connecté sur cette machine · {email}',
    machineConnectedNoEmail: '{harness} est connecté sur cette machine',
    machineFound: '{harness} est installé sur cette machine',
    machineNotConnected: '{harness} n’est pas connecté sur cette machine',
    /** Antigravity CLI (ESC-271): added first, signed in just after by agy in the terminal of the window. */
    machineSignInLater: '{harness} n’est pas encore connecté sur cette machine : tu t’y connectes juste après, dans un terminal',
    machineAdded: 'Déjà ajouté',
    machineChecking: 'Vérification…',
    machineHint: 'Son dossier habituel, tel quel : ses réglages, sa connexion et ses conversations ({path}).',
    newAccount: 'Un nouveau compte',
    newHint: 'Un dossier à lui sous ~/.escouade, et sa propre connexion.',
    /** Cursor CLI: a new account is an API key (ESC-43). */
    newHintKey: 'La clé d’API d’un autre compte Cursor, gardée dans le trousseau du système.',
    /** Added from a private project's settings (ESC-141). */
    reservedTo: 'Réservé au projet « {name} » : ses sessions, sa mémoire et son historique ne seront partagés avec aucun autre projet.',
    addMachine: 'Ajouter ce compte',
    signInTitle: 'Se connecter au compte « {name} »',
    name: 'Nom',
    namePlaceholder: 'Pro, Équipe…',
    share: 'Partager avec {name}',
    /** The machine's account of the harness, not added (ESC-128): what a new account shares, and what is said of it. */
    shareMachine: 'Partager avec le compte de la machine',
    machineName: 'Le compte de la machine',
    files: 'Fichiers',
    folders: 'Dossiers',
    filesCopied: 'copiés (un lien de fichier demande des droits d’administrateur)',
    mode: 'Partage',
    link: 'Lier (un changement vaut pour les deux comptes)',
    copy: 'Copier',
    nothingToShare: '{name} n’a ni réglages, ni CLAUDE.md, ni skills, agents, commandes, plugins, hooks ou styles de sortie à partager.',
    neverShared: 'La connexion (.credentials.json) et .claude.json ne sont jamais partagés : chaque compte a les siens.',
    create: 'Créer et se connecter',
    /** Above the sign-in's terminal. */
    signIn: 'Connexion',
    signInHint: 'Connecte-toi dans le terminal ci-dessous (commande /login si Claude Code ne la propose pas).',
    connected: 'Connecté au compte {email}',
    connectedNoEmail: 'Connecté',
    ended: '{harness} s’est arrêté avant la connexion.',
    // What a Codex account keeps apart (its folder is a CODEX_HOME).
    codex: {
      nothingToShare: '{name} n’a ni config.toml, ni AGENTS.md, ni skills, prompts ou règles à partager.',
      neverShared: 'La connexion (auth.json) n’est jamais copiée : chaque compte a la sienne (son jeton change à chaque renouvellement).',
      signInHint: 'Connecte-toi dans le terminal ci-dessous : codex login ouvre ton navigateur sur la connexion de ChatGPT.',
    },
    // What a Copilot CLI account keeps apart (its folder is a COPILOT_HOME).
    copilot: {
      nothingToShare: '{name} n’a ni mcp-config.json, ni copilot-instructions.md, ni agents ou skills à partager.',
      neverShared:
        'La connexion n’est jamais copiée : chaque compte se connecte par copilot login (son config.json et le trousseau du système), sans le jeton du GitHub CLI.',
      signInHint: 'Connecte-toi dans le terminal ci-dessous : copilot login donne un code à saisir sur github.com/login/device.',
    },
    cursor: {
      nothingToShare:
        'Rien à partager : chaque agent Cursor a son propre dossier de configuration, où « Pour cet agent » garde ses règles.',
      neverShared:
        'La connexion de Cursor CLI (agent login) est celle de la machine : elle ne se copie pas, et un dossier de configuration ne la déplace pas.',
      signInHint: 'Connecte-toi dans le terminal ci-dessous : agent login ouvre ton navigateur sur cursor.com.',
      /** A new Cursor account is its API key (ESC-43): the machine's is the sign-in of agent login. */
      keyTitle: 'Clé d’API du compte « {name} »',
      key: 'Clé d’API',
      keyPlaceholder: 'Sur cursor.com, Settings › API Keys',
      keyHint:
        'Gardée dans le trousseau du système, donnée aux agents de ce compte par CURSOR_API_KEY, jamais écrite dans un fichier ni sur une ligne de commande. Leurs tours vont au compte Cursor de cette clé ; le compte de la machine reste la connexion d’agent login.',
      create: 'Ajouter le compte',
      saveKey: 'Enregistrer la clé',
    },
    /** Antigravity CLI (ESC-271): the machine's account is its Google sign-in (agy), a new one its Gemini API key. */
    agy: {
      nothingToShare:
        'Rien à partager : chaque agent Antigravity a son propre dossier, où Escouade écrit ses règles et où il garde ses conversations.',
      neverShared:
        'La connexion Google d’Antigravity CLI est celle de la machine : elle reste dans le trousseau du système, et un dossier ne la déplace pas.',
      signInHint:
        'Connecte-toi dans le terminal ci-dessous : agy ouvre ton navigateur sur ton compte Google. Escouade ne voit jamais tes identifiants.',
      newHintKey: 'La clé d’API Gemini d’un autre compte Google, gardée dans le trousseau du système.',
      keyPlaceholder: 'Sur aistudio.google.com, Get API key',
      keyHint:
        'Gardée dans le trousseau du système, donnée aux agents de ce compte par GEMINI_API_KEY, jamais écrite dans un fichier ni sur une ligne de commande. Leurs tours vont à l’API Gemini de cette clé (ses modèles seulement) ; le compte de la machine reste la connexion Google d’agy.',
    },
    /** Gemini CLI (ESC-275): a new account is its Google sign-in, made by gemini in a terminal of its folder, or its API key. */
    gemini: {
      nothingToShare:
        'Rien à partager : chaque agent Gemini a son propre GEMINI_CLI_HOME, où Escouade écrit ses réglages et ses règles et où il garde ses sessions.',
      neverShared:
        'La connexion Google de ce compte reste dans son dossier, où gemini l’écrit : Escouade la donne à ses agents par GOOGLE_APPLICATION_CREDENTIALS sans jamais la lire. ~/.gemini n’est jamais touché.',
      signInHint:
        'Connecte-toi dans le terminal ci-dessous : gemini ouvre ton navigateur sur ton compte Google. Escouade ne voit jamais tes identifiants.',
      newHintKey: 'Une connexion Google faite par gemini dans un terminal, ou une clé d’API Gemini gardée dans le trousseau du système.',
      keyPlaceholder: 'Sur aistudio.google.com, Get API key',
      keyHint:
        'Gardée dans le trousseau du système, donnée aux agents de ce compte par GEMINI_API_KEY, jamais écrite dans un fichier ni sur une ligne de commande. Leurs tours vont à l’API Gemini de cette clé.',
      machineApart:
        'Gemini CLI garde sa connexion Google dans ~/.gemini, qu’Escouade ne touche pas : ajoute un nouveau compte, connecté par gemini ou par une clé d’API. Une clé GEMINI_API_KEY de la machine en ferait le compte de la machine.',
      way: 'Connexion',
      google: 'Compte Google',
      googleHint: 'gemini s’y connecte dans un terminal, juste après',
      apiKey: 'Clé d’API',
      apiKeyHint: 'Une clé d’API Gemini, d’aistudio.google.com',
    },
    /** Mistral Vibe (ESC-267): the machine's account is the key of vibe --setup, a new one its API key. */
    vibe: {
      nothingToShare: 'Rien à partager : chaque agent Vibe a son propre VIBE_HOME, où « Pour cet agent » garde ses règles et ses sessions.',
      neverShared:
        'La clé de Mistral Vibe (vibe --setup) est celle de la machine : elle reste dans le trousseau du système, et un dossier de configuration ne la déplace pas.',
      signInHint: 'Donne ta clé d’API dans le terminal ci-dessous : vibe --setup la garde dans le trousseau du système.',
      newHintKey: 'La clé d’API d’un autre compte Mistral, gardée dans le trousseau du système.',
      keyPlaceholder: 'Sur console.mistral.ai, API Keys',
      keyHint:
        'Gardée dans le trousseau du système, donnée aux agents de ce compte par MISTRAL_API_KEY, jamais écrite dans un fichier ni sur une ligne de commande. Leurs tours sont facturés au compte Mistral de cette clé ; le compte de la machine reste la clé de vibe --setup.',
    },
    restart: 'Relancer',
    done: 'Terminé',
  },

  // The account chip of the Composer, before the agent's first message.
  composer: {
    caption: 'Compte',
    auto: 'Automatique ({name})',
    autoNone: 'Automatique (aucun compte)',
    autoDetail:
      'Celui du projet, sinon le premier compte dont la 5 h est sous les seuils de pause et de changement de compte et dont la semaine tient son rythme.',
    inactive: '{name} (inactif)',
    locked: 'Le compte d’un agent ne change plus après son premier message (sa conversation est rangée dans ce compte).',
  },

  // « Compte préféré » in a project's settings.
  project: {
    label: 'Compte préféré',
    desc: 'Le compte sur lequel partent les nouveaux agents et les tickets de ce projet.',
    auto: 'Automatique',
    autoTitle:
      'Le premier compte actif dont la 5 h est sous les seuils de pause et de changement de compte et dont la semaine tient son rythme, dans l’ordre des comptes',
    inactive: '{name} (inactif)',
  },

  /** A private project's accounts (ESC-141), in its settings and in Réglages › Comptes. `{names}`: quoted, as in a sentence. */
  private: {
    own: {
      one: 'Compte réservé à ce projet : {names}. Sessions, mémoire et historique de {harness} n’y sont partagés avec aucun autre projet.',
      other:
        'Comptes réservés à ce projet : {names}. Sessions, mémoire et historique de {harness} n’y sont partagés avec aucun autre projet.',
    },
    /** After `own` when the machine's account is one of them. */
    machineToo: 'Le compte de la machine reste celui de {harness} hors d’Escouade.',
    shared: {
      one: 'Ses agents peuvent partir sur {names}, un compte partagé avec d’autres projets : sessions, mémoire, historique et réglages globaux de {harness} y sont communs.',
      other:
        'Ses agents peuvent partir sur {names}, des comptes partagés avec d’autres projets : sessions, mémoire, historique et réglages globaux de {harness} y sont communs.',
    },
    add: 'Ajouter un compte réservé…',
    reserve: 'Réserver {name} à ce projet',
    reserveTitle: 'Les autres projets ne partiront plus sur ce compte.',
    sharedOnLimit: 'Comptes partagés à la limite',
    sharedOnLimitDesc:
      'À la limite d’usage de ses comptes réservés, ses agents peuvent continuer sur un compte partagé (changement de compte, passage à un autre harness). Éteint, ils attendent la remise à zéro.',
    /** Réglages › Comptes, under an account a private project shares. */
    sharing: {
      one: 'Le projet privé {names} part encore sur ce compte, partagé avec d’autres projets.',
      other: 'Les projets privés {names} partent encore sur ce compte, partagé avec d’autres projets.',
    },
    reserveTo: 'Réserver à « {name} »',
  },

  // Under the autopilot's pause: the accounts that are past « Pause au-delà du quota ».
  pause: {
    over: { one: 'Le compte {accounts} a passé le seuil.', other: 'Les comptes {accounts} ont passé le seuil.' },
  },

  // The current account's quota in the status bar (the percentage is in a tooltip), and the panel of every account's.
  // Each bar is a window of the quota, labelled by its length (`{n}`), or the month's credits.
  quota: {
    hours: { label: '{n} h', name: { one: 'Quota sur {count} heure', other: 'Quota sur {count} heures' } },
    days: { label: '{n} j', name: { one: 'Quota sur {count} jour', other: 'Quota sur {count} jours' } },
    credits: { label: 'crédits', name: 'Crédits du mois' },
    /** A week's window, as its bar is labelled. */
    week: { label: '7 j' },
    /** `{countdown}`: the time left, as `fCountdown` writes it. */
    reset: 'reset {countdown}',
    /** The same, when it is also the account new agents go to (no other one takes them). */
    overageCurrent:
      '{names} : usage supplémentaire en cours, facturé à part. Les agents déjà lancés y restent, et les nouveaux y partent aussi.',
    /** The account, over its window, goes on answering as extra usage, billed apart; `{names}`: its name(s). */
    overage:
      '{names} : usage supplémentaire en cours, facturé à part. Les agents déjà lancés y restent, les nouveaux partent sur un autre compte.',
    /** The same, while « Changer de compte à » moves its agents (ESC-148). */
    overageMoving:
      '{names} : usage supplémentaire en cours, facturé à part. Ses agents passent sur un autre compte à la fin de leur tour, et les nouveaux y partent.',
    /** After the reset: where the window would end at the pace used so far; `{pct}`: « 112 % ». */
    projected: '→ {pct}',
    /** Before it, for screen readers only (the arrow and the colour say it to the eye). */
    projectedSr: 'projection à la remise à zéro : ',
    /** After the day's cost of a harness without a quota: where it would end at midnight; `{amount}`: « 3,40 $ ». */
    projectedCost: '→ {amount}',
    /** After a window's tooltip, whatever the status bar shows; `{pct}`: « 64 % ». */
    projectedTip: 'projection à la remise à zéro : {pct}',
    /** The value of a bar: `{pct}`: « 42 % »; `{date}`: « 10/10 »; `{time}`: « 18:00 ». */
    tip: '{pct} · remise à zéro le {date} à {time}',
    unavailable: 'Quota indisponible',
    /** One window as the bar group says it to the keyboard and to screen readers; `{name}`: « Quota sur 5 heures », `{tip}`: the tooltip's text. */
    line: '{name} : {tip}',
  },

  // The panel the quota group opens, above the status bar, when there are several accounts.
  panel: {
    /** The button that is the quota group; `{name}`: the current account. */
    open: 'Quotas par compte (compte en cours : {name})',
    title: 'Quotas des comptes',
    /** The status bar's gauges, a button that opens the panel. */
    label: 'Quotas par compte',
    /** Under a harness without a quota: only what its turns cost. */
    noQuota: 'Pas de quota : seul le coût de ses tours compte.',
    /** Cursor CLI (ESC-43): it tells Escouade neither its quota nor its cost. */
    usageElsewhere: '{harness} ne donne ni son usage ni son coût à Escouade.',
    usageLink: 'Voir l’usage sur {site}',
    budget: 'Budget par jour',
    budgetReset: 'Remise à zéro à minuit.',
    projectionNote:
      'Après chaque quota, où il en sera à sa remise à zéro au rythme actuel : vert, orange au-delà de 80 %, rouge au-delà de 100 %. La barre d’état ne montre que celles qui passent 80 %.',
    /** The same with « Projection des quotas » on « Toujours ». */
    projectionNoteAlways:
      'Après chaque quota, où il en sera à sa remise à zéro au rythme actuel : vert, orange au-delà de 80 %, rouge au-delà de 100 %.',
    current: 'en cours',
    over: 'au-delà du seuil de pause ou de changement de compte',
    /** A long window (week, 30 days, credits) that would not last to its reset at the current pace: the account goes behind the others. */
    behind: 'rythme trop rapide pour sa fenêtre',
    /** A long window at 95 % or more, on pace or not: the account goes behind the others. */
    nearLimit: 'près de la limite de sa fenêtre',
    /** What the account's turns cost today; `{amount}`: « 1,20 $ » (« ≈ 1,20 $ » while a turn is running). */
    today: "Aujourd'hui : {amount}",
  },

  // The card of a turn stopped by the usage limit: go on on another account (the session goes along).
  resume: {
    /** `{account}`: the account the agent goes on on. */
    on: 'Reprendre sur {account}',
    /** The button that opens the menu of the other accounts. */
    another: 'Reprendre sur un autre compte',
    /** The turn that followed the move failed; `{account}`: the account it went to, `{error}`: what Claude Code said. */
    failed: 'La reprise sur {account} a échoué : {error}.',
    /** `{account}`: the account the agent came from. */
    back: 'Revenir sur {account}',
  },

  // The « Quotas » group, at the end of the « Comptes » tab: how the status bar shows them, for every account.
  quotas: {
    title: 'Quotas',
    /** Where an agent goes at the limit, gathered in « Comptes » (choice 2 of the 2.0 design). */
    limitTitle: 'Quand un quota est épuisé',
    limitNote:
      'Les nouveaux agents partent sur le premier compte actif de leur harness dont la 5 h est sous les seuils « Pause au-delà du quota » et « Changer de compte à » et dont la semaine tient son rythme. Ce qui suit dit où va un agent qui les atteint.',
    /** « Changer de compte à » (ESC-148): its choices are percentages, and « À la limite ». */
    switchAt: 'Changer de compte à',
    switchAtLimit: 'À la limite',
    switchAtDesc:
      'Un agent passe sur un autre compte de son harness à la fin de son tour quand une fenêtre courte (5 h) de son compte atteint ce seuil, ou qu’une fenêtre longue (semaine, 30 jours, crédits du mois) l’atteint sans tenir jusqu’à sa remise à zéro au rythme actuel. Les nouveaux agents et les tickets partent sur le premier compte de l’ordre dont la 5 h n’est pas pleine et dont la semaine tient son rythme, même sur « À la limite » (un agent lancé ne bouge alors pas avant 100 %, puis « Reprendre sur un autre compte… », ci-dessous, décide). « Pause au-delà du quota » (Kanban) retient en plus les tickets quand tous les comptes l’ont passée.',
    projection: 'Projection des quotas',
    projectionDesc:
      'Après chaque quota, où il en sera à sa remise à zéro au rythme actuel (« → 64 % ») : vert, orange au-delà de 80 %, rouge au-delà de 100 % ; pour un harness sans quota, le coût du jour projeté à minuit, rapporté au budget. Le panneau des quotas et les infobulles les montrent toutes ; la barre d’état, celles qui passent 80 % ou toutes, sauf sous 1240 px de large, où elles restent dans les infobulles.',
    /** The choices of « Projection des quotas ». */
    projectionAlert: 'Au-delà de 80 %',
    projectionAlways: 'Toujours',
    projectionOff: 'Jamais',
  },

  // « Ordre des harness », in the « Comptes » tab once there are two harnesses.
  engineOrder: {
    label: 'Ordre des harness',
    desc: 'Après son harness par défaut, un projet passe aux suivants dans cet ordre quand le quota du sien est épuisé, s’ils ont un compte actif qui le sert.',
    up: 'Monter {name}',
    down: 'Descendre {name}',
  },

  // The setting of the Harness tab.
  switchOnLimit: {
    label: 'Reprendre sur un autre compte un agent de ticket arrêté par la limite',
    desc: 'Sinon il attend la remise à zéro de son compte',
  },
} as const satisfies Tree;
