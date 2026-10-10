import { defineZone } from '../types';

// The MCP server: what a project lets its agents do with it, and how its tools read in the conversation.
export default defineZone('mcp', {
  project: {
    group: 'MCP server',
    agentsUse: 'Agents can use Escouade',
    agentsUseDesc:
      'They read the tickets and the agents, and can create tickets or start other agents, which uses quota. Applies the next time their process starts.',
  },
  tools: {
    named: 'Escouade · {tool}',
    listProjects: 'List the projects',
    listAgents: 'List the agents',
    listTickets: 'List the tickets',
    getTicket: 'Read a ticket',
    getUsage: 'Read the quota',
    getAgentSummary: 'Sum up an agent',
    createTicket: 'Create a ticket',
    updateTicket: 'Edit a ticket',
    moveTicket: 'Move a ticket',
    startTicket: 'Start a ticket',
    createAgent: 'Start an agent',
    sendMessage: 'Send a message',
    stopAgent: 'Stop an agent',
    reportProgress: 'Report progress',
    splitTicket: 'Split its ticket',
  },
  progress: {
    label: 'What the agent says it is doing',
  },
  permission: {
    args: 'What Claude gives the tool',
    argName: '{name}:',
  },
});
