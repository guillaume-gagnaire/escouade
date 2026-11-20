import type { Tree } from '../types';

// « Coûts » de l'équipe (TS-16, docs/TEAM-SERVER.md §10): des totaux par jour chiffrés pour les seuls administrateurs,
// le tableau par membre (si l'équipe l'a choisi), projet, ticket, harness et modèle, les budgets, et ce que chaque
// membre voit de ce qui remonte de lui.
export default {
  open: 'Coûts',
  title: 'Coûts et quotas',
  leadAdmin:
    'Les totaux de chaque poste de l’équipe, par jour : chiffrés par la clé des administrateurs, le serveur d’équipe n’en lit rien. Jamais un prompt, une réponse, un fichier ni un tour.',
  leadMember:
    'Tes propres totaux, par jour, et le total de chaque projet : les totaux des autres membres sont chiffrés pour les seuls administrateurs.',
  loading: 'Lecture des coûts…',
  empty: 'Aucun tour sur un projet de l’équipe pour cette période.',
  period: {
    label: 'Période',
    week: 'Semaine',
    month: 'Mois',
    custom: 'Période choisie',
    from: 'Du',
    to: 'au',
  },
  by: {
    label: 'Regrouper',
    member: 'Par membre',
    project: 'Par projet',
    ticket: 'Par ticket',
    model: 'Par harness et modèle',
  },
  col: {
    name: 'Nom',
    tokens: 'Tokens',
    cost: 'Coût',
    quota: 'Quota max',
  },
  total: 'Total',
  member: {
    me: 'Toi',
    other: 'Membre {id}',
    unnamed: 'Sans nom (par projet)',
  },
  plan: {
    only: 'équivalent API',
    some: 'en partie un équivalent API',
    hint: 'Un compte Claude Max ou ChatGPT Pro n’a pas de coût par tour : c’est son quota consommé, compté au prix de l’API.',
  },
  budgets: {
    title: 'Budgets du mois',
    none: 'L’équipe n’a pas fixé de budget : un administrateur les écrit dans le fichier de l’équipe (costs.budgets).',
    stop: 'Au-delà d’un budget, le pilote auto s’arrête (imposé par l’équipe).',
    spent: '{spent} sur {budget}',
  },
  summary: {
    title: 'Le mois de l’équipe',
    team: 'Toute l’équipe',
  },
  key: {
    none: 'Aucun administrateur n’a encore tiré la clé des coûts : tes totaux attendent sur ce poste.',
    stale: 'La clé des coûts attend d’être changée (un administrateur est parti) : tes totaux attendent sur ce poste.',
    epoch: 'Clé des administrateurs n° {epoch}',
  },
  mine: {
    title: 'Ce qui remonte de toi',
    lead: 'Tes totaux par jour, jamais ton travail : les tokens, le coût estimé, le plus haut quota et le total de chaque ticket, une fois par heure.',
    pending: {
      one: '{count} jour attend d’être envoyé.',
      other: '{count} jours attendent d’être envoyés.',
    },
    none: 'Rien n’est encore parti de ce poste.',
    show: 'Voir ce qui est parti',
    hide: 'Masquer',
    send: 'Envoyer maintenant',
  },
  grain: {
    project: 'L’équipe {team} voit les totaux par projet : les tiens partent sans ton nom.',
    member: 'L’équipe {team} voit les totaux par membre : ses administrateurs voient tes totaux par jour, à ton nom.',
    memberUntold:
      'L’équipe {team} a choisi de voir les totaux par membre : tes totaux partent sans ton nom tant que tu n’as pas lu l’écran qui le dit.',
  },
  told: {
    title: 'Ton équipe voit tes totaux par jour',
    body: 'L’équipe {team} est passée au grain « par membre » : ses administrateurs voient désormais, à ton nom, tes totaux par jour (tokens, coût estimé, plus haut quota, total de chaque ticket).',
    never: 'Jamais ton travail : aucun prompt, aucune réponse, aucun nom de fichier, aucun tour ne part.',
    where: 'Tu vois à tout moment ce qui remonte de toi dans Équipe › Coûts.',
    ok: 'J’ai compris',
    see: 'Voir ce qui remonte',
  },
  notice: {
    launchBody: '{text}. Le lancer quand même ?',
    launchStops: '{text} : le pilote auto de ce projet est arrêté par l’équipe {team}. Le lancer quand même ?',
    stopped: 'Pilote auto arrêté : {text}.',
  },
} as const satisfies Tree;
