import { defineZone } from '../types';

// The parts of a project switched on or off (ESC-658): Settings › Projects › Project parts.
export default defineZone('parts', {
  docs: {
    name: 'Docs',
    desc: 'The specification, decisions, documents and discussions of this project.',
    takes:
      'Off: the Docs tab, its MCP tools and what agents are told of it disappear for this project; the spec is no longer compared to the code nor planned there. Pages, decisions and documents are kept.',
  },
  memory: {
    name: 'Shared memory',
    desc: 'The work notebook the agents of this project share, whatever their harness.',
    takes:
      'Off: its MCP tools, what agents are told of it and its section of the Team page disappear for this project. The memories stay in .escouade/memory/.',
  },
  kanban: {
    name: 'Kanban',
    desc: 'The tickets, the autopilot, the scheduled tasks and the workflows of this project.',
    takes:
      'Off: the project has agents alone. The Kanban tab, its settings and its MCP tools disappear, none of its tickets starts, its workflows are no longer followed. Its tickets are kept.',
  },
  launch: {
    name: 'Launch and browsers',
    desc: 'The commands that launch this project, their pages and its browsers.',
    takes:
      'Off: the launch buttons, the pages and their probes disappear for this project, and its agents’ test launches stop. The commands are kept.',
  },
  page: {
    foot: 'A part turned off is off for this project alone, on this device: nothing is deleted, and turning it on again finds everything. On, it starts nothing by itself: its background work follows its own settings.',
    repo: 'Proposed by the repository (.escouade.yml): your choice here goes first.',
    device: 'Chosen on this device.',
    deviceOverRepo: 'Chosen on this device: it goes before the default the repository proposes (.escouade.yml).',
    teamDefault: 'Recommended by the team {team}: your choice here goes first.',
    enforced: 'Imposed by the team {team}: it cannot be changed on this device.',
    feature: 'Off for all of Escouade, in Settings › Features: turn it on there first.',
    reset: 'Back to the default',
    resetLabel: 'Back to the default of “{name}”',
  },
});
