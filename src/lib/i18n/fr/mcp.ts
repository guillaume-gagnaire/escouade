import type { Tree } from '../types';

// The MCP server: what a project lets its agents do with it, and how its tools read in the conversation.
export default {
  project: {
    group: 'Serveur MCP',
    agentsUse: 'Les agents peuvent utiliser Escouade',
    agentsUseDesc:
      'Ils lisent les tickets et les agents, et peuvent créer des tickets ou lancer d’autres agents, ce qui dépense du quota. S’applique au prochain démarrage de leur process.',
  },
  // The « Claude Code » tab of the settings: the server declared in Claude Code, and what it was asked.
  section: {
    title: 'Escouade dans Claude',
    enable: 'Claude peut piloter Escouade',
    enableDesc:
      'Claude Code (dans un terminal, un autre outil ou un agent d’Escouade) lit tes projets, agents et tickets, et peut créer des tickets ou lancer des agents.',
    /** `{accounts}`: the accounts it is declared in, each as « compte … » (below), joined by commas. */
    declared: 'Déclaré dans Claude · {accounts}',
    account: 'compte {name}',
    working: 'Déclaration dans Claude…',
    notDeclared: 'Pas déclaré pour le compte {name} : {error}',
    serverFailed: 'Le serveur n’a pas démarré : {error}',
    command: 'À lancer à la main :',
    copy: 'Copier la commande',
    copied: 'Commande copiée',
    copyFailed: 'Copie impossible : {error}',
    note: 'Le jeton est écrit en clair dans la config de Claude Code (~/.claude.json pour le compte Principal). Désactiver change le jeton.',
  },
  activity: {
    title: 'Activité MCP',
    list: 'Appels au serveur MCP, les plus récents en tête',
    empty: 'Aucun appel pour l’instant.',
    /** The result of a call (a request refused before it reached a tool has none). */
    outcome: { ok: 'Fait', refused: 'Refusé', error: 'Erreur' },
  },
  tools: {
    /** The badge of one of its tools: `tool` is its name in words (below), or as Claude names it. */
    named: 'Escouade · {tool}',
    listProjects: 'Lister les projets',
    listAgents: 'Lister les agents',
    listTickets: 'Lister les tickets',
    getTicket: 'Lire un ticket',
    getUsage: 'Lire le quota',
    getAgentSummary: 'Résumer un agent',
    createTicket: 'Créer un ticket',
    updateTicket: 'Modifier un ticket',
    moveTicket: 'Déplacer un ticket',
    startTicket: 'Lancer un ticket',
    createAgent: 'Lancer un agent',
    sendMessage: 'Envoyer un message',
    stopAgent: 'Arrêter un agent',
    reportProgress: 'Donner son avancement',
    splitTicket: 'Découper son ticket',
  },
  progress: {
    /** What a status line an agent reported (`report_progress`) is, for assistive technologies; the line is the agent's own words. */
    label: 'Ce que l’agent dit faire',
  },
  permission: {
    /** What a permission card lists: each argument the tool is given, by its name. */
    args: 'Ce que Claude donne à l’outil',
    argName: '{name} :',
    /** A value shown cut: how many characters are not, and where the whole request is. */
    cut: {
      one: '{n} caractère de plus n’est pas montré : lis la demande entière dans la conversation.',
      other: '{n} caractères de plus ne sont pas montrés : lis la demande entière dans la conversation.',
    },
  },
} as const satisfies Tree;
