import type { Tree } from '../types';

// The window on the remote API of the core (ESC-495, docs/REMOTE.md « La fondation 2b »): the band of a lost connection,
// the errors the API names, what a terminal lost during a cut.
export default {
  band: {
    lost: 'Connexion au cœur perdue · nouvel essai…',
    stopped: 'Connexion au cœur refusée : {reason}',
  },
  error: {
    deskOnly: '« {command} » se fait sur ce poste, pas à distance.',
    notServedYet: '« {command} » n’est pas encore servie à distance.',
    policy: 'La stratégie de ce poste interdit l’API distante.',
    token: 'Le jeton de cet appareil est absent, inconnu ou révoqué.',
    version: 'Le cœur et cette fenêtre ne sont pas de la même version.',
    tooMany: 'Trop de connexions au cœur ouvertes.',
    busy: 'Trop d’appels en attente de réponse : réessaie dans un instant.',
    tooLarge: 'Envoi trop gros pour le cœur.',
    lost: 'Connexion au cœur perdue avant la réponse : « {command} » n’a peut-être pas été faite.',
    offline: 'Connexion au cœur perdue : « {command} » n’a pas été envoyée.',
  },
  term: {
    lost: '[{size} perdus pendant la coupure]',
  },
} as const satisfies Tree;
