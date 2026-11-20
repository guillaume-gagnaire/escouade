import type { Tree } from '../types';

// The features switched on or off as a whole (F-1, ESC-550): Réglages › Fonctionnalités.
export default {
  /** The essential, never switched off. */
  essential: {
    name: 'Agents et conversation',
    desc: 'L’essentiel, toujours là : les agents et leur conversation, le panneau Git, les terminaux et les lancements.',
    always: 'Ne se coupe pas.',
  },
  kanban: {
    name: 'Kanban',
    desc: 'Les tickets et leurs critères, le pilote auto, les sources de tickets.',
    takes:
      'Coupé : son onglet, ses colonnes, ses réglages et ses outils MCP disparaissent, le pilote auto ne lance plus rien. Ses tickets restent.',
  },
  editor: {
    name: 'Éditeur',
    desc: 'Ouvrir, modifier et rechercher dans les fichiers du projet.',
    takes: 'Coupé : la vue de l’éditeur, Ctrl P et ses serveurs de langage s’arrêtent. Ses réglages restent.',
  },
  docs: {
    name: 'Espace Docs',
    desc: 'La spécification, les décisions, les documents et les discussions.',
    takes: 'Coupé : son onglet et ses outils MCP disparaissent. Les pages, décisions et documents restent.',
  },
  cost: {
    name: 'Coût',
    desc: 'Le coût des tours, des agents et des tickets ; la température du cache.',
    takes: 'Coupé : les montants et la température disparaissent partout. Les coûts restent comptés.',
  },
  stats: {
    name: 'Statistiques',
    desc: 'L’activité et les dépenses dans le temps.',
    takes: 'Coupé : sa vue disparaît du rail. Les chiffres restent comptés.',
  },
  workflows: {
    name: 'Workflows',
    desc: 'Les enchaînements d’agents et de skills d’un projet.',
    takes: 'Coupé : son onglet et son outil MCP disparaissent, plus rien n’est lu. Leurs fichiers restent.',
  },
  scheduled: {
    name: 'Tâches programmées',
    desc: 'Des tickets qui reviennent à heure fixe.',
    takes: 'Coupé : leur section et leurs outils MCP disparaissent, aucune tâche ne part. Les tâches restent.',
  },
  overseer: {
    name: 'Overseer',
    desc: 'L’agent qui pilote tous les projets, à qui tu parles.',
    takes: 'Coupé : son écran, son raccourci et ses outils MCP disparaissent, il s’arrête. Son journal reste.',
  },
  voice: {
    name: 'Voix',
    desc: 'La dictée et la conversation à voix haute.',
    takes: 'Coupé : le micro et son raccourci disparaissent, plus rien n’écoute. Ses réglages et ses modèles restent.',
  },
  page: {
    used: 'Utilisée',
    usedOn: { one: 'Utilisée sur {count} projet', other: 'Utilisée sur {count} projets' },
    needs: 'Part avec « {need} » : rallume-le d’abord.',
    foot: 'Une fonctionnalité coupée disparaît vraiment, ce qu’elle faisait en fond s’arrête ; ses données restent et reviennent quand tu la rallumes.',
  },
} as const satisfies Tree;
