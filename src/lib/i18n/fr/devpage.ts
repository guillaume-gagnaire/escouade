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
    empty: 'Tape une adresse.',
    notAnAddress: 'Ce n’est pas une adresse (localhost:3000, par exemple).',
    scheme: 'Seule une adresse http:// ou https:// s’ouvre dans un navigateur d’Escouade.',
  },
} as const;
