import { defineZone } from '../types';

// The features switched on or off as a whole (F-1, ESC-550): Settings › Features.
export default defineZone('features', {
  essential: {
    name: 'Agents and conversation',
    desc: 'The essentials, always there: the agents and their conversation, the Git panel, the terminals and the launches.',
    always: 'Cannot be turned off.',
  },
  kanban: {
    name: 'Kanban',
    desc: 'Tickets and their criteria, the autopilot, ticket sources.',
    takes: 'Off: its tab, columns, settings and MCP tools disappear, and the autopilot no longer starts anything. Its tickets are kept.',
  },
  editor: {
    name: 'Editor',
    desc: 'Open, edit and search the project’s files.',
    takes: 'Off: the editor view, Ctrl P and its language servers stop. Its settings are kept.',
  },
  docs: {
    name: 'Docs space',
    desc: 'The specification, decisions, documents and discussions.',
    takes: 'Off: its tab and MCP tools disappear. Pages, decisions and documents are kept.',
  },
  cost: {
    name: 'Cost',
    desc: 'The cost of turns, agents and tickets; the cache temperature.',
    takes: 'Off: amounts and the temperature disappear everywhere. Costs are still counted.',
  },
  stats: {
    name: 'Statistics',
    desc: 'Activity and spending over time.',
    takes: 'Off: its view leaves the rail. The figures are still counted.',
  },
  workflows: {
    name: 'Workflows',
    desc: 'A project’s chains of agents and skills.',
    takes: 'Off: its tab and MCP tool disappear, and nothing is read. Their files are kept.',
  },
  scheduled: {
    name: 'Scheduled tasks',
    desc: 'Tickets that come back at a set time.',
    takes: 'Off: their section and MCP tools disappear, and no task runs. The tasks are kept.',
  },
  overseer: {
    name: 'Overseer',
    desc: 'The agent that drives every project, the one you talk to.',
    takes: 'Off: its screen, shortcut and MCP tools disappear, and it stops. Its journal is kept.',
  },
  voice: {
    name: 'Voice',
    desc: 'Dictation and spoken conversation.',
    takes: 'Off: the microphone and its shortcut disappear, and nothing listens. Its settings and models are kept.',
  },
  page: {
    used: 'In use',
    usedOn: { one: 'In use on {count} project', other: 'In use on {count} projects' },
    needs: 'Goes with “{need}”: turn it on first.',
    foot: 'A feature turned off really disappears, and what it did in the background stops; its data are kept and come back when you turn it on again.',
  },
  offer: {
    label: 'Suggestion: {name}',
    kicker: 'Feature turned off',
    noticed: 'What Escouade noticed',
    would: 'What it would do',
    private: 'What Escouade notices stays on your machine.',
    enable: 'Turn on',
    later: 'Not now',
    never: 'Don’t suggest again',
    neverHint: 'You can bring it back in Settings › Features.',
    signals: {
      mcpTickets: {
        one: '{count} ticket asked of the MCP server this week was refused: the Kanban is off.',
        other: '{count} tickets asked of the MCP server this week were refused: the Kanban is off.',
      },
      longPrompts: {
        one: 'You started {count} agent by hand this week, on a long instruction.',
        other: 'You started {count} agents by hand this week, each on a long instruction.',
      },
      specEdits: {
        one: 'The spec of “{project}” (SPEC.md, docs/) changed on {count} day in the last two weeks.',
        other: 'The spec of “{project}” (SPEC.md, docs/) changed on {count} days in the last two weeks.',
      },
      documents: {
        one: 'You dropped {count} document into a conversation in the last two weeks.',
        other: 'You dropped {count} documents into the conversations in the last two weeks.',
      },
      usageLimit: {
        one: 'An agent hit its usage limit this week.',
        other: 'Agents hit their usage limit {count} times this week.',
      },
      agentFiles: {
        one: '“{project}” has {count} skill, command or agent (.claude/, .agents/).',
        other: '“{project}” has {count} skills, commands or agents (.claude/, .agents/).',
      },
      sameMessage: {
        one: 'You send the same instruction by hand, at about the same time.',
        other: 'You send the same instruction by hand at about the same time, {count} days in a row.',
      },
      waitingProjects: {
        one: 'Agents were waiting for an answer on {count} project.',
        other: 'Agents were waiting for your answer on {count} projects at once.',
      },
    },
    does: {
      kanban: 'The Kanban would make them tickets: each with its agent, worktree and criteria, started by the autopilot.',
      editor: 'The editor would open the project’s files next to the agents.',
      docs: 'The Docs space would hold the project’s spec, decisions and documents, which the agents read and cite.',
      cost: 'Cost would show what turns, agents and tickets cost, and the cache temperature, so you see the limit coming.',
      stats: 'Statistics would show the activity and spending over time.',
      workflows: 'Workflows would read these chains and show the step each agent is at.',
      scheduled: 'A scheduled task would send it for you, at a set time, as a ticket.',
      overseer: 'The Overseer would watch every project at once and tell you who is waiting for you.',
      voice: 'Voice would let you dictate your messages and talk to the agents.',
    },
  },
  declined: {
    title: 'Dismissed suggestions',
    desc: 'What you chose never to be suggested again when it would help.',
    none: 'None: Escouade may suggest a feature turned off when it would help, one at a time, one a day at most.',
    restore: 'Suggest again',
    restoreLabel: 'Suggest “{name}” again',
  },
});
