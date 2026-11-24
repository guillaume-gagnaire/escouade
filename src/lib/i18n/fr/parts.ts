import type { Tree } from '../types';

// The parts of a project switched on or off (ESC-658): Réglages › Projets › Parties du projet.
export default {
  docs: {
    name: 'Docs',
    desc: 'La spécification, les décisions, les documents et les discussions de ce projet.',
    takes:
      'Coupé : l’onglet Docs, ses outils MCP et ce qu’on en dit aux agents disparaissent pour ce projet ; la spec n’y est plus comparée au code ni planifiée. Les pages, décisions et documents restent.',
  },
  memory: {
    name: 'Mémoire partagée',
    desc: 'Le carnet de travail que partagent les agents de ce projet, quel que soit leur harness.',
    takes:
      'Coupé : ses outils MCP, ce qu’on en dit aux agents et sa section de la page Équipe disparaissent pour ce projet. Les souvenirs restent dans .escouade/memory/.',
  },
  kanban: {
    name: 'Kanban',
    desc: 'Les tickets, le pilote auto, les tâches programmées et les workflows de ce projet.',
    takes:
      'Coupé : le projet n’a plus que des agents. L’onglet Kanban, ses réglages et ses outils MCP disparaissent, aucun de ses tickets ne démarre, ses workflows ne sont plus suivis. Ses tickets restent.',
  },
  launch: {
    name: 'Lancement et navigateurs',
    desc: 'Les commandes qui lancent ce projet, leurs pages et ses navigateurs.',
    takes:
      'Coupé : les boutons de lancement, les pages et leurs sondes disparaissent pour ce projet, et les lancements de test de ses agents s’arrêtent. Les commandes restent.',
  },
  page: {
    foot: 'Une partie coupée l’est pour ce projet seul, sur ce poste : rien n’est effacé, et la rallumer retrouve tout. Allumée, elle ne lance rien d’elle-même : son travail de fond suit ses propres réglages.',
    repo: 'Proposé par le dépôt (.escouade.yml) : ton choix ici passe devant.',
    device: 'Choisi sur ce poste.',
    deviceOverRepo: 'Choisi sur ce poste : il passe devant le défaut que propose le dépôt (.escouade.yml).',
    teamDefault: 'Recommandé par l’équipe {team} : ton choix ici passe devant.',
    enforced: 'Imposé par l’équipe {team} : il ne se change pas sur ce poste.',
    feature: 'Coupé pour tout Escouade, dans Réglages › Fonctionnalités : rallume-le d’abord.',
    reset: 'Reprendre le défaut',
    resetLabel: 'Reprendre le défaut de « {name} »',
  },
} as const satisfies Tree;
