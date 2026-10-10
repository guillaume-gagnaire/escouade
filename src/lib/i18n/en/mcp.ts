import { defineZone } from '../types';

// The MCP server: what a project lets its agents do with it, and how its tools read in the conversation.
export default defineZone('mcp', {
  project: {
    group: 'MCP server',
    agentsUse: 'Agents can use Escouade',
    agentsUseDesc:
      'They read the tickets and the agents, and can create tickets or start other agents, which uses quota. Applies the next time their process starts.',
  },
  section: {
    title: 'Escouade in Claude',
    enable: 'Claude can drive Escouade',
    enableDesc:
      'Claude Code (in a terminal, another tool or an Escouade agent) reads your projects, agents and tickets, and can create tickets or start agents.',
    declared: 'Declared in Claude · {accounts}',
    account: '{name} account',
    working: 'Declaring in Claude…',
    notDeclared: 'Not declared for the {name} account: {error}',
    serverFailed: 'The server did not start: {error}',
    command: 'Run it by hand:',
    copy: 'Copy the command',
    copied: 'Command copied',
    copyFailed: 'Copy failed: {error}',
    note: 'The token is written in plain text in Claude Code’s config (~/.claude.json for the Main account). Turning it off changes the token.',
  },
  activity: {
    title: 'MCP activity',
    list: 'Calls to the MCP server, the most recent first',
    empty: 'No calls yet.',
    outcome: { ok: 'Done', refused: 'Refused', error: 'Error' },
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
    cut: {
      one: '{n} more character is not shown: read the whole request in the conversation.',
      other: '{n} more characters are not shown: read the whole request in the conversation.',
    },
  },
});
