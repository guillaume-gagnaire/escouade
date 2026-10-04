// Demo data: made-up projects, agents and tickets.

import type { AgentInfo } from './ui/Sidebar';
import type { Status, Tab } from './ui/Shell';

export const TABS: Tab[] = [
  { name: 'demo-api', hue: 48, delta: 7, running: true },
  { name: 'studio-web', hue: 300, delta: 3 },
  { name: 'mobile-app', hue: 200, delta: 12, running: true },
  { name: 'infra', hue: 150 },
];

export const AGENTS: AgentInfo[] = [
  { name: 'refacto-auth', status: 'running', model: 'Opus 5.5', time: '12m 40s', tokens: '184 k', cost: '2,41 $', files: 6 },
  { name: 'tests-e2e', status: 'running', model: 'Sonnet 5.5', time: '8m 02s', tokens: '96 k', cost: '0,73 $', files: 3 },
  {
    name: 'pagination-users',
    status: 'running',
    model: 'Opus 5.5',
    time: '3m 15s',
    tokens: '41 k',
    cost: '0,32 $',
    files: 3,
    branch: 'pagination-users',
  },
  { name: 'docs-api', status: 'done', model: 'Haiku 4.5', time: '1m 50s', tokens: '12 k', cost: '0,02 $', files: 1 },
  { name: 'fix-login', status: 'idle', model: 'Fable 5.1', time: '0m 00s', tokens: '0', cost: '0,00 $', files: 0 },
];

/** pagination-users before it is moved to Opus, in the composer scene: the scenes up to it. */
export const onSonnet = (agents: AgentInfo[]) => agents.map((a) => (a.name === 'pagination-users' ? { ...a, model: 'Sonnet 5.5' } : a));

export const agent = (name: string) => AGENTS.find((a) => a.name === name)!;

export const STATUS: Status = {
  active: 3,
  waiting: 0,
  done: 1,
  session: 22,
  weekly: 36,
  sessionReset: '2 h 14',
  weeklyReset: '3 j',
  cost: 3.48,
  procs: { n: 4, mem: '1,2 Go', cpu: '6 %' },
  sync: { branch: 'main', behind: 3, ahead: 1 },
};
export const EMPTY_STATUS: Status = { active: 0, waiting: 0, done: 0, session: 0, weekly: 0, cost: 0 };

export const CRITERIA_DEM4 = [
  'Au-delà de 5 échecs en 15 min, la connexion est refusée',
  'Le message d’erreur dit quand réessayer',
  'Tests verts',
];
export const PROGRESS_DEM4 = [
  'Compteur d’échecs par IP et par compte',
  'Middleware de limitation sur /login',
  'Message « Réessaie dans 15 min »',
  'Tests du limiteur',
];
