// The page of a launch inside Escouade (ESC-311): its bar, its states, its « Pages » window.
export default {
  /** The view of the agent that shows its page. */
  view: 'Page',
  viewHint: 'La page de son lancement, dans Escouade',
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
  detach: 'Ouvrir dans une fenêtre à part',
  attach: 'Remettre dans l’agent',
  system: 'Ouvrir dans le navigateur du système',
  more: 'Plus d’actions sur la page',
  /** `{title}`: « pagination-users · web ». */
  owner: 'Page de {title}',
  apart: 'Page à part : ni Escouade, ni tes cookies',
  loading: 'Chargement de {host}…',
  /** The launch is not up yet. */
  waiting: 'Le lancement démarre…',
  waitingText: 'La page s’ouvre ici dès que son serveur répond.',
  stalled: '{host} ne répond pas',
  stalledText: 'La page n’a pas fini de charger en 20 s. Le serveur tourne peut-être encore : recharge, ou regarde ses logs.',
  stopped: 'Le lancement s’est arrêté',
  stoppedText: 'Relance pour retrouver la page à la même adresse.',
  relaunch: 'Relancer',
  viewLogs: 'Voir les logs',
  detached: 'La page est dans la fenêtre « Pages » de {project}.',
  showWindow: 'Aller à la fenêtre',
  covered: 'La page revient quand ce qui passe dessus se ferme.',
  /** `{host}`: the other site; `{page}`: the launch's. */
  outside: '{host} s’est ouvert dans ton navigateur : ici, seul {page} s’affiche.',
  refused: 'La page a voulu ouvrir {url} : refusé.',
  /** The « Pages » window. */
  window: {
    tabs: 'Pages du projet',
    empty: 'Aucune page ouverte.',
  },
} as const;
