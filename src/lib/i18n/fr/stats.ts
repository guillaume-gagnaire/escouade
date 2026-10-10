import type { Tree } from '../types';

// The statistics, and the overview of every agent.
export default {
  title: 'Statistiques',
  subtitle: "Agents lancés depuis l'app · {projects}, {agents}",
  // The period, and the word for one step of it (« Tokens par jour »).
  range: { day: 'Jour', week: 'Semaine', month: 'Mois' },
  unit: { day: 'jour', week: 'semaine', month: 'mois' },
  span: { day: '14 derniers jours', week: '12 dernières semaines', month: '12 derniers mois' },
  series: { input: 'Entrée', cache: 'Cache', output: 'Sortie' },

  kpi: {
    tokens: 'Tokens',
    spanDelta: '{span} · {delta}',
    totalCost: 'Coût global',
    costSince: '{amount} depuis le {date}',
    costAllTime: '{amount} au total',
    costPerPrompt: 'Coût moyen / prompt',
    tokensPerPrompt: '≈ {tokens} tokens / prompt',
    noPrompt: 'aucun prompt sur la période',
    prompts: 'Prompts',
    perUnit: '{n} par {unit} en moyenne',
  },

  chart: {
    title: 'Tokens par {unit}',
    label: 'Tokens par {unit}, empilés entrée, cache et sortie',
    showTable: 'Tableau',
    showChart: 'Graphique',
  },
  table: { period: 'Période', total: 'Total', prompts: 'Prompts' },

  noData: 'Aucune donnée sur la période.',
  closedProject: 'Projet fermé',
  byProject: { title: 'Par projet' },
  byModel: { title: 'Par modèle' },
  byAgent: { title: 'Par agent', tokens: 'Tokens', deleted: 'Agent supprimé' },
  byTicket: {
    title: 'Par ticket',
    ticket: 'Ticket',
    name: 'Titre',
    loops: 'Boucles',
    periodCost: 'Coût sur la période',
    none: 'Aucun ticket sur la période.',
  },
  showAll: 'Tout voir',
  showLess: 'Réduire',
  shownOf: '{shown} sur {total}',
  unavailable: 'Statistiques indisponibles : {error}',

  // « Vue d’ensemble »: every agent of every project in one list.
  overview: {
    sub: '{projects} · {agents} · {running} en cours',
    hintChoose: '{keys} choisir',
    hintOpen: '{key} ouvrir',
    hintBack: '{key} revenir',
    activity: 'Activité',
    context: 'Contexte',
    since: 'Depuis',
    contextUnknown: 'Contexte : pas encore connu',
    lastChange: 'Dernier changement {when}',
    waitingGroup: 'Attend ta réponse',
    empty: 'Aucun agent pour l’instant.',
    resumes: 'Reprise {when}',
    settingUp: 'Prépare le worktree · {step}',
    thinking: 'Réfléchit',
    // What an agent is doing, by its status.
    status: { running: 'En cours', waiting: 'Question', idle: 'Prêt', done: 'Terminé', error: 'Erreur' },
    allow: 'Autoriser',
    deny: 'Refuser',
    readInConversation: 'À lire dans la conversation avant de répondre.',
    tooLong: 'Trop long pour être lu ici : lis-la et réponds dans la conversation.',
    proposesPlan: 'Claude propose un plan',
    waitsForAnswer: 'Claude attend ta réponse',
  },
} as const satisfies Tree;
