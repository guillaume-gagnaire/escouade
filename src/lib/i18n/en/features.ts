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
});
