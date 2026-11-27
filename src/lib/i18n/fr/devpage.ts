// The page of a launch inside Escouade (ESC-311): its bar, its states, its « Pages » window.
export default {
  /** The view of the agent that shows its page. */
  view: 'Page',
  viewHint: 'La page de son lancement, dans Navigateurs',
  back: 'Retour',
  forward: 'Avancer',
  reload: 'Recharger',
  address: 'Adresse de la page',
  devtools: 'Devtools',
  devtoolsHint: 'Outils de développement de cette page ({key})',
  modesLabel: 'Largeur de la page',
  modes: { full: 'Pleine largeur', tablet: 'Tablette', mobile: 'Mobile' },
  /** `{width}`: « · 768 px », empty for the whole room; `{key}`: its shortcut. */
  modeHint: '{mode}{width} ({key} pour passer à la suivante)',
  modeKey: 'Ctrl+Maj+M',
  width: '{width} px',
  shrunkShort: 'Rétréci à {width} px',
  shrunk: 'La zone est plus étroite que {device} px : la page y est rétrécie à {width} px.',
  system: 'Ouvrir dans le navigateur du système',
  more: 'Plus d’actions sur la page',
  /** The agent's menu (ESC-419): its page in a pane of the split screen. */
  openBeside: 'Ouvrir la page à côté',
  openBelow: 'Ouvrir la page en dessous',
  /** `{title}`: « pagination-users · web ». */
  owner: 'Page de {title}',
  apart: 'Page à part : ni Escouade, ni tes cookies',
  loading: 'Chargement de {host}…',
  /** The launch is not up yet. */
  waiting: 'Le lancement démarre…',
  waitingText: 'La page s’ouvre ici dès que son serveur répond.',
  stalled: '{host} ne répond pas',
  stalledText: 'La page n’a pas fini de charger en 20 s. Le serveur tourne peut-être encore : recharge, ou regarde ses logs.',
  /** A browser opened alone (ESC-657): no launch, no logs. */
  stalledAlone: 'La page n’a pas fini de charger en 20 s : son serveur est peut-être lent ou arrêté. Recharge quand il répond.',
  stopped: 'Le lancement s’est arrêté',
  stoppedText: 'Relance pour retrouver la page à la même adresse.',
  relaunch: 'Relancer',
  viewLogs: 'Voir les logs',
  covered: 'La page revient quand ce qui passe dessus se ferme.',
  /** `{host}`: the other site; `{page}`: the launch's. */
  outside: '{host} s’est ouvert dans ton navigateur : ici, seul {page} s’affiche.',
  refused: 'La page a voulu ouvrir {url} : refusé.',
  /** Where the devtools go on this system (ESC-657): docked in the page where its webview can, else a window of their own. */
  devtoolsWhere: {
    windows: 'Dans leur propre fenêtre : WebView2 ne les loge pas dans la page.',
    mac: 'Dans leur propre fenêtre : logé dans la page, l’inspecteur de WebKit en change la largeur.',
    linux: 'Dans la page, en bas : leurs boutons les mettent à droite ou à part.',
  },
  /** The section « Navigateurs » of the sidebar (ESC-657): one row per tab. */
  /** The band « veut lire » of a browser (ESC-669): an agent asks to read a page outside its project's launch. */
  consent: {
    /** `{agent}`: its name; the host follows, then `session`. */
    wants: '{agent} veut lire',
    session: '(avec ta session, dans ce navigateur)',
    more: { one: 'et {count} autre demande', other: 'et {count} autres demandes' },
    allow: 'Permettre',
    refuse: 'Refuser',
    /** Its row's menu: the leaves given and the requests waiting, gone. */
    revoke: 'Retirer les accords',
    revoked: 'Accords retirés : les agents redemanderont pour lire ce navigateur.',
  },
  /** An agent's page (AP-3): its row of « Navigateurs », its view and its bar. */
  agentPage: {
    /** The view's name, for the screen readers. */
    view: 'Page de {agent} : {title}',
    by: '{agent}',
    templates: { review: 'Revue', diagram: 'Schéma', compare: 'Comparaison', html: 'Page HTML', custom: 'Page à soi' },
    versions: 'Versions de la page',
    /** A version, short: the bar's chip, the row's tag, the card. */
    version: 'v{n}',
    versionItem: 'Version {n}',
    latest: 'la dernière',
    older: 'Tu regardes la v{n} ; la dernière est la v{last}.',
    toLatest: 'Revenir à la v{last}',
    close: 'Fermer la page',
    loading: 'La page se charge…',
    gone: 'Cette page est fermée : sa carte dans la conversation la rouvre.',
    wants: 'La page veut ouvrir',
    wantsOpen: 'Ouvrir dans le navigateur du système',
    wantsIgnore: 'Ignorer',
    /** Its row in « Navigateurs »: its tip, its tag, its point. */
    rowTip: '{title} · page de {agent}',
    unseen: 'Nouvelle page : elle attend que tu l’ouvres',
    closeRow: 'Fermer la page (elle reste gardée)',
  },
  /** The band of the choice under an agent's page (AP-4): what the page proposed, sent on the user's gesture alone. */
  choice: {
    label: 'Ta réponse à {agent}',
    idle: 'Choisis dans la page : rien ne part sans ton geste ici.',
    proposed: 'Choisi dans la page :',
    note: 'avec le mot « {note} »',
    send: 'Envoyer à {agent}',
    cancel: 'Annuler',
    inBox: 'Ouvrir dans « À toi »',
    gate: 'Un feu vert : réponds-y dans la boîte ou ici, jamais depuis la page.',
    yes: 'Oui',
    no: 'Non',
    sending: 'Envoyé à {agent} dans quelques secondes.',
    taken: 'Revenir sur ma réponse',
    answered: 'Répondu :',
    trust: 'Les mots de cette bande sont ceux de la demande, pas ceux de la page.',
  },
  /** The driving band of every row (AP-3; driven by AB-8). */
  drive: {
    label: 'Pilotage et confiance',
    trust: 'Écrite par {agent} · hors réseau, sans accès à Escouade ni à tes fichiers',
    trustTip:
      'La page vit à part : aucune adresse du web, aucune commande d’Escouade, aucun de tes fichiers. Ce qu’elle dit vient de l’agent : relis avant de copier une commande.',
    drives: '{agent} pilote',
    take: 'Reprendre la main',
    youHave: 'Tu as la main',
    give: 'La rendre',
    continue: 'Continuer',
    /** ESC-680 (AB-8): the band wired, the driven row, the numbered rows, the rail. */
    continueTip: '{agent} attend que tu aies regardé : « Continuer » le laisse poursuivre',
    takeTip: 'L’agent s’arrête aussitôt : son prochain geste est refusé tant que tu ne la lui rends pas',
    giveTip: '{agent} pourra de nouveau piloter ce navigateur',
    stopped: 'Tu ne laisses plus piloter',
    stoppedTip: 'Jusqu’à la fermeture de ce navigateur',
    lastSr: 'Geste {n} : {what}',
    /** The row of « Navigateurs » an agent drives. */
    rowDriven: 'Piloté par {agent}',
    rowYours: 'Tu as repris la main sur {agent}',
    rowStopped: 'Pilotage arrêté',
    stop: 'Ne plus laisser piloter',
    /** A gesture in the conversation. */
    running: 'Pilote le navigateur…',
    stepSr: 'Geste {n} : ',
    rowTip: 'Afficher le navigateur',
    rowPageTip: 'Afficher la page où l’agent l’avait mise',
    rowGone: 'Ce navigateur est fermé',
    /** The rail of an agent’s page (direction A). */
    rail: 'Repères de {agent}',
    pinTip: '{n} · {what} — y revenir',
    pins: { one: '{count} repère', other: '{count} repères' },
    /** C2: an agent asks to fill a password. */
    password: '{agent} veut remplir un mot de passe',
    passwordWhy: '(dans ce navigateur, jusqu’à sa fermeture ; la valeur n’est jamais relue)',
  },
  /** The card of a page in its agent's conversation (AP-3). */
  card: {
    label: 'Page montrée par {agent}',
    by: 'Page de {agent}',
    template: 'modèle {name}',
    where: { beside: 'à côté', waiting: 'attend dans Navigateurs', closed: 'fermée', erased: 'plus gardée' },
    shown: 'Montrée',
    updated: 'Mise à jour',
    newer: 'v{n} depuis',
    show: 'Afficher',
    reopen: 'Rouvrir',
    opening: 'Ouverture de la page…',
    erased: 'Cette page n’est plus gardée : son agent ou son projet n’est plus là, ou la place a manqué.',
    off: 'Les pages des agents sont coupées (Réglages › Agents).',
    closedBy: 'Page {page} fermée par {agent} : elle reste gardée.',
  },
  /** The band « veut ouvrir » of an agent's conversation (ESC-678, choice C3): an address outside the project's launch. */
  open: {
    /** `{agent}`: its name; the host follows, then `where`. */
    wants: '{agent} veut ouvrir',
    where: '(hors du lancement du projet, dans un navigateur d’Escouade)',
    more: { one: 'et {count} autre demande', other: 'et {count} autres demandes' },
    allow: 'Permettre une fois',
    refuse: 'Refuser',
  },
  browsers: {
    title: 'Navigateurs',
    new: 'Nouveau navigateur',
    none: 'Aucun navigateur ouvert',
    field: 'Adresse du nouveau navigateur',
    placeholder: 'localhost:3000',
    open: 'Ouvrir',
    /** Its row's dot, for the screen readers and its tip. */
    state: { loading: 'charge', ready: 'chargée', stalled: 'ne répond pas' },
    copy: 'Copier l’adresse',
    copied: 'Adresse copiée',
    close: 'Fermer le navigateur',
    /** `{name}`: the agent whose launch opened it. */
    by: 'Lancement de {name}',
    /** A tab an agent opened (ESC-678). `{name}`: the agent. */
    opener: 'Ouvert par {name}',
    /** Its row while the user looks elsewhere than the agent's conversation (ESC-678). */
    waiting: '{name} veut te montrer la page',
    empty: 'Tape une adresse.',
    notAnAddress: 'Ce n’est pas une adresse (localhost:3000, par exemple).',
    scheme: 'Seule une adresse http:// ou https:// s’ouvre dans un navigateur d’Escouade.',
  },
} as const;
