import type { Tree } from '../types';

// The MCP server: what a project lets its agents do with it, and how its tools read in the conversation.
export default {
  project: {
    group: 'Serveur MCP',
    agentsUse: 'Les agents peuvent utiliser Escouade',
    agentsUseDesc:
      'Ils lisent les tickets et les agents, et peuvent créer des tickets ou lancer d’autres agents, ce qui dépense du quota. S’applique au prochain démarrage de leur process.',
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
  },
} as const satisfies Tree;
