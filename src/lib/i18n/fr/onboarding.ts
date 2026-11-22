import type { Tree } from '../types';

// Le premier lancement (ESC-504, docs/ONBOARDING.md, direction C « La ligne ») : une station par question, sur une ligne
// tracée comme le git graph ; et l'icône Aide du rail, d'où il revient.
export default {
  title: 'Premier lancement',
  /** `{count}`: the stations. */
  subtitle: { one: '{count} station · environ 3 minutes', other: '{count} stations · environ 3 minutes' },
  /** The name of the line of the stations, for a screen reader. */
  line: 'Les stations du premier lancement',
  skipAll: 'Tout passer',
  later: 'Plus tard',
  continue: 'Continuer',
  back: 'Retour',
  /** Done again from Aide: it closes, nothing undone. */
  close: 'Fermer',
  /** The keys, beside their caps: « ↑ ↓ choisir · Entrée valider ». */
  choose: 'choisir',
  confirm: 'valider',
  enter: 'Entrée',
  resume: 'Tout se passe et se reprend. Le parcours revient depuis Aide › Premier lancement.',
  then: 'Ensuite : À découvrir',
  thenLanes: 'Planifier · Coder · Relire · Livrer · Piloter',
  /** `{n}`, the station's number; `{name}`, its name in a sentence. */
  order: 'Station {n} · {name}',
  /** Under a station's name in the line. */
  state: {
    now: 'maintenant',
    skipped: 'passé · à reprendre',
    policy: 'imposé',
  },
  stations: {
    harness: { name: 'Harness', inline: 'le harness', todo: 'le programme des agents' },
    account: { name: 'Compte', inline: 'le compte', todo: 'celui de la machine' },
    defaults: { name: 'Réglages de départ', inline: 'les réglages de départ', todo: 'le mode des agents' },
    hookups: { name: 'Ce que tu veux brancher', inline: 'ce que tu veux brancher', todo: 'tickets, voix, hébergeur' },
    project: { name: 'Premier projet', inline: 'le premier projet', todo: 'dossier, clone ou démo' },
    agent: { name: 'Premier agent', inline: 'le premier agent', todo: 'un message, une réponse' },
    features: { name: 'Fonctionnalités', inline: 'les fonctionnalités', todo: 'ce qu’Escouade te montre' },
    telemetry: { name: 'Statistiques', inline: 'les statistiques', todo: 'partagées ou non' },
  },
  harness: {
    question: 'Avec quel harness tes agents travaillent-ils ?',
    why: 'Un harness est le programme d’agent de code de ta machine. Escouade le lance et le pilote ; il garde ta connexion et tes réglages.',
    ready: 'Connecté · {email}',
    readyNoEmail: 'Connecté sur cette machine',
    installed: 'Installé, pas connecté',
    installedHint: 'Tu t’y connecteras dans un terminal, juste après.',
    absent: 'Introuvable',
    tooOld: 'Trop ancien',
    unsupported: 'Version non prise en charge',
    copy: 'Copier',
    /** `{harness}`: its name. */
    copyLabel: 'Copier la commande de {harness}',
    copied: 'Commande copiée',
    none: 'Aucun harness n’est installé. Copie la commande de celui que tu veux, lance-la dans un terminal : Escouade le trouvera dès que tu reviens.',
    /** The harness chosen has no plugin installed yet: the seed's, with no network (ESC-373). */
    plugin: 'Escouade installe le plugin de {harness}, depuis l’app, sans réseau.',
    /** Signed in on the machine: the account station is folded into this one (choice O6). */
    machineAccount: 'Ses agents partent sur le compte déjà connecté ; un compte à part se crée dans Réglages › Comptes et harness.',
    /** The harness chosen goes first in « Ordre des harness ». */
    first: 'Le harness choisi passe en tête de l’ordre des harness : c’est celui des nouveaux projets.',
    /** `{harness}`: the only one a policy allows. */
    onlyAllowed: '{harness} est le seul harness permis sur ce poste.',
  },
  account: {
    question: 'Quel compte pour {harness} ?',
    why: 'Les quotas, les statistiques et la connexion sont tenus par compte. Tu pourras en ajouter d’autres (perso, pro) : Escouade passe au suivant quand l’un atteint sa limite.',
    machine: 'Le compte de la machine',
    machineEmail: 'Le compte de la machine · {email}',
    machineHint: 'Son dossier habituel, tel quel : ses réglages, sa connexion et ses conversations.',
    separate: 'Un compte à part',
    separateHint: 'Un dossier à lui sous ~/.escouade, et sa propre connexion.',
    notSignedIn: '{harness} n’est pas encore connecté sur cette machine : tu t’y connectes juste après, dans un terminal.',
    /** The account added already (elsewhere, or by this station). */
    added: 'Compte ajouté : {name}',
    noHarness: 'Choisis d’abord un harness, à la station précédente.',
  },
  defaults: {
    question: 'Quand tes agents doivent-ils te demander ?',
    /** `{model}`, `{effort}`: the account's, as Claude Code's are said. */
    why: 'Le mode des nouveaux agents ; tu le changes pour chacun, sous son message. Modèle {model}, effort {effort} : ceux de ton compte.',
    whyNoModel:
      'Le mode des nouveaux agents ; tu le changes pour chacun, sous son message. Effort {effort} : celui de ton compte, avec le modèle par défaut de ton harness.',
    bypass:
      'Éteint : l’agent ferait tout sans rien te montrer, même ce qu’un contenu piégé lui dicterait. À réserver à une machine jetable ; il s’allume dans Réglages › Agents.',
    bypassAllowed: 'Permis dans Réglages › Agents : jamais proposé comme mode de départ, il se choisit agent par agent.',
    off: 'Éteint',
    allowed: 'Permis',
    /** ESC-489: a harness whose only mode is Bypass. */
    onlyBypass: '{harness} ne tourne qu’en Bypass : ses agents agiront sans demander.',
    keep: 'Garder {mode}',
  },
  /** « Ce que tu veux brancher » (ESC-512): a card per hookup, none ticked. */
  hookups: {
    question: 'Que veux-tu brancher ?',
    why: 'Rien n’est coché : coche ce qui te sert. Rien ne s’installe, ne se connecte ni ne se télécharge sans ton geste ; ce que tu passes t’attend dans À découvrir et dans les Réglages.',
    /** The cards' group, for a screen reader. */
    cards: 'Les branchements',
    /** Under the station's name in the line, once gone through: `{count}` cards plugged in. */
    plugged: { one: '{count} branché', other: '{count} branchés' },
    nothing: 'rien de branché',
    later: 'Plus tard',
    /** `{name}`: the card's title. */
    laterLabel: 'Plus tard : {name}',
    done: 'Branché',
    passed: 'Passé · à reprendre',
    /** `{by}`: « Imposé par la stratégie de ce poste ». */
    locked: 'Verrouillé · {by}',
    /** The real list of what a card offers, its names joined. */
    sources: {
      title: 'Une source de tickets',
      body: 'Les tickets de {list} arrivent dans le Kanban : un agent les prend et les fait avancer. Il faut un jeton de la source, puis un projet ou un tableau à relier.',
      pick: 'Quelle source ?',
      /** What each source imports, and what it asks. A source the window does not know has the generic line. */
      asks: {
        jira: 'Les tickets d’un projet Jira · ton site, ton e-mail et un jeton d’API',
        trello: 'Les cartes d’un tableau Trello · une clé et un jeton d’API',
        github: 'Les issues d’un dépôt GitHub · un jeton GitHub (ou celui de gh)',
        linear: 'Les tickets d’une équipe Linear · une clé d’API',
        clickup: 'Les tâches d’une liste ClickUp · une clé d’API personnelle',
        gitlab: 'Les issues d’un projet GitLab · le compte de l’hébergeur GitLab',
        azure: 'Les work items d’un projet Azure Boards · le compte de l’hébergeur Azure DevOps',
        other: 'Ses tickets dans le Kanban · un jeton de la source',
      },
      link: 'Le projet ou le tableau se relie ensuite à chaque projet, dans ses Réglages › Tickets importés.',
    },
    voice: {
      title: 'La voix',
      body: 'Veux-tu dicter ou parler à tes agents ? Whisper sur ta machine, rien n’en sort ; ou un service en ligne, avec ta clé.',
      pick: 'Qui transcrit ce que tu dis ?',
      local: 'Whisper, sur ta machine',
      /** `{model}`, `{size}`: the model to download. */
      localHint:
        'Le modèle « {model} » : {size} à télécharger une fois, gardés sur ton disque. Rien de ce que tu dis ne quitte ta machine.',
      download: 'Télécharger ({size})',
      /** `{done}`, `{total}`: sizes. */
      downloading: 'Téléchargement : {done} sur {total}',
      cancel: 'Annuler le téléchargement',
      ready: 'Modèle « {model} » prêt',
      online: 'Un service en ligne',
      onlineHint: 'Plus rapide sur une petite machine. Chaque dictée part chez le service choisi, avec ta clé.',
      service: 'Le service',
      /** `{service}`, `{host}`: what leaves the machine, and where to. */
      sends: 'Ce qui part chez {service} ({host}) : l’audio de tes dictées, rien d’autre. Ta clé reste dans le trousseau du système.',
      tryTitle: 'Essaie le micro',
      tryHint: 'Clique sur le micro, dis une phrase, puis reclique : elle s’écrit ici. Rien n’est envoyé à un agent.',
      tryField: 'Ta phrase dictée',
      conversation: 'La voix qui lit les réponses dans une conversation se règle dans Réglages › Voix.',
    },
    forge: {
      title: 'Un hébergeur de code',
      body: 'Les PR et la CI des tickets dans Escouade, sur {list}. Il faut un jeton de l’hébergeur, gardé dans le trousseau du système.',
      pick: 'Quel hébergeur ?',
    },
    /** `{plugin}`: its name. */
    plugin: 'Escouade installe le plugin {plugin}, depuis l’app, sans réseau.',
    pluginInstalled: 'Plugin {plugin} installé.',
    /** Not in the seed: the store's, installed in Plugins with its permissions read. */
    pluginStore: 'Son plugin vient du store : installe-le depuis Plugins, ses permissions sous les yeux.',
    /** `{list}`: the store's other plugins of the kind, from its catalogue already read. */
    alsoStore: 'Aussi dans le store : {list}. Ils s’installent depuis Plugins.',
    offline: 'Hors ligne : ceci demande le réseau. La carte t’attend dans À découvrir, tu la reprendras une fois en ligne.',
  },
  project: {
    question: 'Où travaille ton premier agent ?',
    why: 'Dans un projet, chaque agent travaille dans son worktree, sur sa branche : deux agents ne se gênent pas, et rien n’entre dans ta branche sans toi.',
    folder: 'Un dossier de ta machine',
    folderHint: 'Un dépôt que tu as déjà. Escouade le lit sans rien y changer.',
    add: 'Ajouter un projet…',
    opened: 'Projet ouvert : {name}',
    /** The four words of the product, each in a line (§ 4.4). */
    words: {
      label: 'Les mots d’Escouade',
      /** Between a word and what it is. */
      sep: ' : ',
      project: { term: 'Un projet', text: 'un dossier, avec ses agents, son Kanban et ses docs.' },
      agent: { term: 'Un agent', text: 'une conversation avec ton harness, qui lit et modifie le code.' },
      worktree: { term: 'Un worktree', text: 'une copie de travail à l’agent seul : deux agents ne se gênent pas.' },
      branch: { term: 'Une branche par agent', text: 'son travail reste à part jusqu’à ce que tu l’intègres.' },
    },
    /** The drawing of the branches, beside the words. */
    schema: {
      main: 'ta branche',
      agent: 'agent 1 · son worktree',
      agent2: 'agent 2',
    },
    browse: 'Parcourir…',
    folderLabel: 'Le dossier du projet',
    folderPlaceholder: 'Le dossier de ton dépôt',
    missing: 'Ce dossier n’existe pas.',
    notWritable: 'Escouade ne peut pas écrire dans ce dossier ({path}) : choisis-en un autre.',
    noGit: 'Ce dossier n’est pas un dépôt git : Escouade peut en faire un, avec un premier commit.',
    firstCommit: 'Faire un premier commit',
    /** `{list}`: the keys the file sets. */
    yml: 'Ce dépôt a un .escouade.yml : il règle {list}. Ses commandes se relisent avant de tourner.',
    open: 'Ouvrir le projet',
    clone: 'Cloner un dépôt',
    cloneHint: 'Une adresse git, ou un dépôt de GitHub ou GitLab si tu en as connecté un.',
    cloneUrl: 'Adresse du dépôt',
    clonePlaceholder: 'https://github.com/acme/api.git',
    cloneHosts: 'Tes dépôts sur {host}',
    cloneParent: 'Dans le dossier',
    cloneFolder: 'Nom du dossier',
    cloneGo: 'Cloner et ouvrir',
    cloning: 'Clonage…',
    retry: 'Réessayer',
    cloneOffline: 'Cloner demande le réseau. Choisis un dossier ou la démonstration, ou réessaie une fois en ligne.',
    /** `{error}`: what git said. */
    cloneFailed: 'Le clonage a échoué : {error}. Vérifie l’adresse et ton accès, puis réessaie.',
    demo: 'Un projet de démonstration',
    demoHint: 'Un petit dépôt jetable, créé sur ta machine sans réseau, pour essayer sans risque. Supprimé d’un clic.',
    /** `{path}`: where it is written. */
    demoWhere: 'Il s’écrit dans {path} : une liste de tâches en TypeScript, deux tests qui échouent exprès. Rien ne sort de ta machine.',
    demoGo: 'Créer la démonstration',
    creating: 'Création…',
    team: 'Les projets de ton équipe',
    /** `{team}`: its name. */
    teamHint: '{team} partage ses projets : clone celui sur lequel tu travailles.',
    teamGo: 'Voir les projets de {team}',
  },
  agent: {
    question: 'Ton premier message',
    /** `{harness}`: the agent's. */
    why: 'Choisis-en un, modifie-le, puis envoie-le. Il part sur ton compte {harness} et compte dans son quota.',
    startersLabel: 'Les messages proposés',
    starters: {
      explain: 'Explique-moi ce dépôt en cinq lignes.',
      tests: 'Lance les tests et dis-moi ce qui casse.',
      bug: 'Trouve un petit bug et propose un correctif.',
    },
    /** `{agent}`: its name. */
    ready: '{agent} est prêt. Un message proposé t’attend : modifie-le ou choisis-en un autre. Rien ne part avant que tu l’envoies.',
    frame: 'La conversation de ton premier agent',
    preparing: 'Ton agent se prépare…',
    /** Sent, the agent's worktree being made (at the first message). */
    preparing2: {
      title: 'Ton agent prépare son worktree',
      body: 'Ton message est parti : il attend que la copie de travail de l’agent soit prête, sur sa branche à lui. Sa réponse suit, à droite.',
    },
    running: {
      title: 'Ton agent travaille',
      body: 'Sa réponse arrive en direct, à droite. Il travaille dans son worktree, sur sa branche : ton dépôt ne bouge pas.',
      stop: 'Pour l’arrêter : Échap, ou ■ sous le message.',
    },
    permission: {
      title: 'Ton agent te demande la permission',
      /** `{mode}`: the agent's; `{allow}`, `{always}`, `{deny}`: the buttons' names, in bold. */
      body: 'Il veut lancer une commande. En {mode}, chaque commande te demande : {allow} cette fois, {always} pour cette commande dans ce projet, ou {deny} pour qu’il cherche autrement.',
      allow: 'Autoriser',
      always: 'Toujours',
      deny: 'Refuser',
      /** The shortcuts of the permission card and of the next agent waiting, as the system writes them. */
      keys: 'Au clavier : {allow} autorise, {always} autorise toujours. {next} t’amène au prochain agent qui attend.',
      later: 'Plus tard, une demande comme celle-ci fait clignoter la carte de l’agent.',
    },
    /** Said once the message is sent. */
    quota: 'Ton message est parti sur ton compte {harness} et compte dans son quota.',
    asking: {
      title: 'Ton agent te pose une question',
      body: 'Réponds sur sa carte, à droite : il reprend dès que tu as répondu.',
    },
    done: {
      title: 'Ton agent a répondu',
      body: 'L’agent a fini, dans son worktree, sur sa branche : ton dépôt n’a pas bougé.',
      diff: 'Voir le diff',
      files: { one: '{count} fichier changé', other: '{count} fichiers changés' },
      noDiff: 'Il n’a rien changé : il n’y a pas de diff à voir.',
      /** `{agent}`: its name. */
      where: 'Tu le retrouves dans la barre de ton projet, sous son nom « {agent} » ; d’autres messages s’écrivent sous sa conversation.',
      diffTitle: 'Ce que {agent} a changé',
    },
    limit: {
      title: 'Ce compte est à sa limite',
      /** `{time}`: when it resets. */
      body: 'Ce compte a atteint sa limite d’usage jusqu’à {time}. Ajoute un autre compte, ou reviens après.',
      bodyNoTime: 'Ce compte a atteint sa limite d’usage. Ajoute un autre compte, ou reviens après.',
      add: 'Ajouter un compte…',
    },
    offline: {
      title: 'Hors ligne',
      body: '{harness} n’a pas pu joindre son service : tu sembles hors ligne. Le message reste prêt.',
      again: 'Remettre le message',
    },
    signedOut: {
      title: '{harness} n’est pas connecté',
      body: '{harness} n’est pas connecté : reviens à la station du compte.',
      back: 'Revenir à la station du compte',
    },
    failed: {
      title: 'Le tour a échoué',
      /** `{error}`: what the harness said. */
      body: 'Le tour a échoué : {error}. Réessaie, ou passe à la suite avec « Plus tard ».',
    },
    noProject: 'Ton premier agent a besoin d’un projet : choisis-en un à la station précédente.',
    toProject: 'Revenir au premier projet',
    noAccount: 'Ton premier agent a besoin d’un compte connecté : ajoute-le à la station du compte.',
  },
  /** The band of the demonstration project, at the top of the project (§ 4.4). */
  demo: {
    label: 'Bac à sable',
    /** `{path}`: its folder. */
    text: 'un dépôt de démonstration sur ta machine ({path}) · rien ne sort d’ici',
    remove: 'Supprimer',
    confirmTitle: 'Supprimer le projet de démonstration ?',
    confirmBody:
      'Le projet « bac-a-sable », ses agents et leurs conversations, et son dossier {path} s’effacent. Rien d’autre n’est touché.',
    removed: 'Projet de démonstration supprimé.',
  },
  telemetry: {
    stays: 'Tes projets, ton code, tes conversations et tes comptes restent sur ta machine, dans ~/.escouade.',
    imposedOff: 'Éteintes par la stratégie de ce poste.',
    imposedOn: 'Allumées par la stratégie de ce poste.',
    answered: 'Ta réponse : {answer}. Elle se change dans Réglages › Général.',
  },
  end: {
    title: 'Escouade est prêt',
    body: 'Le reste se découvre en le faisant : la voix, le Kanban, les PR… La carte t’attend dans Aide › À découvrir.',
    skipped: {
      one: '{count} station passée : elle t’attend dans Aide › Premier lancement.',
      other: '{count} stations passées : elles t’attendent dans Aide › Premier lancement.',
    },
    start: 'Commencer',
    /** The map of the discovery (ESC-506), from the end of the journey. */
    discover: 'Voir ce qu’il y a à découvrir',
  },
  /** The rail's Aide (choice O10). */
  help: {
    label: 'Aide',
    tip: 'Le premier lancement, À découvrir, la documentation',
    firstLaunch: 'Premier lancement',
    firstLaunchDetail: 'Revoir le harness, le compte, le mode des agents',
    docs: 'Documentation',
    docsDetail: 'Le guide d’Escouade, sur escouade.dev',
    report: 'Signaler un problème',
    reportDetail: 'Sur GitHub',
  },
  /** Réglages › Général. */
  redo: {
    row: 'Premier lancement',
    rowDesc: 'Revoir le harness, le compte, le mode des nouveaux agents et les statistiques. Rien n’est remis à zéro.',
    button: 'Refaire le premier lancement',
  },
} as const satisfies Tree;
