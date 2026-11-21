import type { Tree } from '../types';

// The background fetch (ESC-561): its settings (Tous les projets › Git, and each project's Git), and what the Git
// panel says of it beside « Fetch ».
export default {
  settings: {
    group: 'Git',
    auto: 'Récupérer automatiquement (git fetch)',
    autoDesc:
      'En arrière-plan, Escouade récupère chaque dépôt qui a un dépôt distant, un à la fois, sans jamais demander d’identifiants : les commits à tirer se voient d’eux-mêmes.',
    every: 'Intervalle',
    everyDesc: 'Entre deux fetch d’un même dépôt, de 1 min à 24 h.',
    timeout: 'Délai d’attente',
    timeoutDesc:
      'Au-delà, le fetch de fond est abandonné et repris à l’intervalle suivant, de 30 s à 30 min. Un fetch à la main garde ses 3 min.',
    /** A project's own. */
    project: 'Récupérer automatiquement (git fetch)',
    projectDesc: 'Le fetch de fond de ce projet, pour un dépôt lourd ou derrière un VPN.',
    modes: { default: 'Comme le réglage général', always: 'Toujours', never: 'Jamais' },
    /** What the general setting says, under the project's choice. */
    generalOn: 'Réglage général : toutes les {every}, délai de {timeout}.',
    generalOff: 'Réglage général : coupé.',
    openGeneral: 'Réglage général…',
    projectEvery: 'Intervalle propre au projet',
    projectEveryDesc: 'Vide : celui du réglage général ({every}).',
    projectTimeout: 'Délai d’attente propre au projet',
    projectTimeoutDesc: 'Vide : celui du réglage général ({timeout}).',
    /** A field's value out of its bounds, brought back to them. */
    clamped: '{typed} est hors des bornes ({min} à {max}) : ramené à {value}.',
    unitMin: 'min',
    unitSec: 's',
  },
  /** A duration, as the fields and the panel write it. */
  minutes: '{n} min',
  hours: '{n} h',
  hoursMinutes: '{h} h {m}',
  seconds: '{n} s',
  minutesSeconds: '{m} min {s} s',
  /** The Git panel, beside « Fetch ». */
  panel: {
    fetched: 'Récupéré {ago}',
    never: 'Jamais récupéré',
    at: 'Dernier fetch : {when}',
    failed: 'Fetch automatique : {reason}',
    reasons: {
      credentials: 'il demande des identifiants. Fais « Fetch » une fois : il reprendra ensuite.',
      timeout: 'pas fini en {timeout}. Le délai d’attente se règle dans les réglages.',
      network: 'le réseau ou le dépôt distant est injoignable.',
      other: 'git a refusé (au survol).',
    },
    openSettings: 'Régler',
    sshNote:
      'Ton Git for Windows 2.47.0 a un ssh qui cale sur les gros fetch : le fetch automatique passe par celui de Windows. Mets Git à jour (2.47.0(2) ou plus).',
  },
} as const satisfies Tree;
