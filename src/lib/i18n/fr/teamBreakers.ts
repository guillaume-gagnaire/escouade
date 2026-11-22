import type { Tree } from '../types';

// The administrator's circuit breaker (TS-15, docs/TEAM-SERVER.md §9): the band across the window, and the team's
// window where an administrator sets one, sees those in force and lifts them.
export default {
  open: 'Coupe-circuit',
  title: 'Coupe-circuit de l’équipe',
  lead: 'Un administrateur retire un harness, un modèle ou un plugin des projets de l’équipe, ou arrête tous ses agents : chaque poste en ligne l’applique dans la seconde, les autres à leur retour. Il ne fait que restreindre, et dure jusqu’à ce qu’un administrateur le lève. Les projets personnels ne sont jamais touchés.',
  leadMember: 'Seul un administrateur pose ou lève un coupe-circuit. Ce poste vérifie la signature de chacun et le rôle de son auteur.',
  none: 'Aucun coupe-circuit en cours.',
  active: 'En cours',
  lifted: 'Levés',
  withdrawn: 'Retirés par la liste de révocation de l’équipe',
  withdrawnLead:
    'Ces plugins du store de l’équipe sont retirés dans toutes leurs versions par sa liste de révocation, signée par la clé de l’équipe : désactivés sur chaque poste, leur installation refusée. Elle se change dans « Plugins » de l’équipe.',
  withdrawnOne: 'Le plugin {id} : {reason}',
  setAt: 'Posé par {by} le {at}',
  liftedAt: 'Levé par {by} le {at}',
  lift: 'Lever',
  lastContact: 'Dernier contact avec le serveur : {at}',
  neverContact: 'Le serveur n’a encore jamais répondu à ce poste.',
  refused: {
    one: '{count} entrée refusée par ce poste (signature ou rôle)',
    other: '{count} entrées refusées par ce poste (signature ou rôle)',
  },
  alarm: {
    rollback: 'Le serveur cache une entrée du coupe-circuit déjà lue : ce poste garde ce qu’il a lu et ne lit plus rien.',
    rewritten: 'Le serveur a changé une entrée du coupe-circuit déjà lue : ce poste garde ce qu’il a lu et ne lit plus rien.',
    server: 'Le serveur numérote les entrées du coupe-circuit à rebours : ce poste ne lit plus rien.',
    fork: 'Le serveur montre deux versions de l’équipe : contacte un administrateur.',
    retry: 'Réessayer',
  },
  form: {
    title: 'Poser un coupe-circuit',
    scope: 'Portée',
    scopeTeam: 'Toute l’équipe',
    scopeProject: 'Un projet de l’équipe',
    project: 'Projet',
    noProject: 'Aucun projet de l’équipe n’a de Kanban d’équipe sur ce poste.',
    what: 'Ce qui est retiré',
    harness: 'Un harness',
    model: 'Un modèle',
    plugin: 'Un plugin',
    agents: 'Tous les agents',
    modelId: 'Modèle',
    pluginId: 'Plugin',
    pluginHint:
      'Un plugin est de l’app : il est coupé sur tout le poste de chaque membre. Pour un harness, retire plutôt le harness : seul l’espace de l’équipe est touché.',
    uninstall: 'Le désinstaller aussi',
    reason: 'Raison',
    reasonHint: 'Chaque membre la lit sur sa bande.',
    by: 'Signé par',
    next: 'Continuer',
  },
  confirm: {
    title: 'Ce qui va s’arrêter',
    online: {
      one: 'Sur {count} poste en ligne, dans la seconde ; les autres à leur retour.',
      other: 'Sur {count} postes en ligne, dans la seconde ; les autres à leur retour.',
    },
    noneOnline: 'Aucun autre poste n’est en ligne : chacun l’appliquera à son retour, avant tout nouveau lancement.',
    onlineUnknown: 'Le nombre de postes en ligne ne se lit pas : chacun l’applique en ligne ou à son retour.',
    set: 'Couper',
    back: 'Modifier',
  },
  what: {
    harnessOf: 'le harness {name}',
    modelOf: 'le modèle {name}',
    pluginOf: 'le plugin {name}',
    pluginUninstalledOf: 'le plugin {name} (désinstallé)',
    agentsOf: 'tous les agents',
  },
  scope: {
    teamOf: 'toute l’équipe {team}',
    projectOf: 'le projet {project} de l’équipe {team}',
  },
  stops: {
    harness: 'Plus aucun agent des projets de l’équipe ne démarre sur {name} ; ceux qui tournent s’arrêtent entre deux tours.',
    model: 'Plus aucun agent des projets de l’équipe ne démarre sur le modèle {name} ; ceux qui tournent s’arrêtent entre deux tours.',
    plugin: 'Le plugin {name} est désactivé sur chaque poste.',
    pluginUninstall: 'Le plugin {name} est désactivé puis désinstallé sur chaque poste.',
    agents:
      'Tous les agents des projets de l’équipe s’arrêtent tout de suite, comme à la sortie de l’app : le tour en cours est perdu, chacun reprendra sa session une fois le coupe-circuit levé.',
  },
  band: {
    asked: 'Arrêt demandé par {by} : {reason}',
    cut: 'Retiré : {what}',
    more: { one: 'et {count} autre coupe-circuit', other: 'et {count} autres coupe-circuits' },
    contact: 'Pas de nouvel agent de l’équipe {team} : son serveur n’a pas répondu depuis plus de {hours} h (stratégie de ce poste)',
    lastContact: 'Dernier contact : {at}',
    never: 'Aucun contact encore',
    view: 'Voir',
  },
} as const satisfies Tree;
