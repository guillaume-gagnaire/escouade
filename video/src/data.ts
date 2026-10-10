// Demo data: made-up projects, agents and tickets. The numbers and names are written for the language of the picture.

import { fmtOf, useLang, type Lang } from './lang';
import type { AgentInfo } from './ui/Sidebar';
import type { Status, Tab } from './ui/Shell';

export const TABS: Tab[] = [
  { name: 'demo-api', hue: 48, delta: 7, running: true },
  { name: 'studio-web', hue: 300, delta: 3 },
  { name: 'mobile-app', hue: 200, delta: 12, running: true },
  { name: 'infra', hue: 150 },
];

/** Agents named from a prompt, so named in its language: the auth refactor that opens most scenes, and the one Haiku names. */
const NAMES = {
  refactor: { fr: 'refacto-auth', en: 'auth-refactor' },
  csv: { fr: 'export-csv-factures', en: 'invoice-csv-export' },
} as const;

export function agentsOf(lang: Lang): AgentInfo[] {
  const { tok, usd } = fmtOf(lang);
  return [
    { name: NAMES.refactor[lang], status: 'running', model: 'Opus 5.5', time: '12m 40s', tokens: tok(184), cost: usd(2.41), files: 6 },
    { name: 'tests-e2e', status: 'running', model: 'Sonnet 5.5', time: '8m 02s', tokens: tok(96), cost: usd(0.73), files: 3 },
    {
      name: 'pagination-users',
      status: 'running',
      model: 'Opus 5.5',
      time: '3m 15s',
      tokens: tok(41),
      cost: usd(0.32),
      files: 3,
      branch: 'pagination-users',
    },
    { name: 'docs-api', status: 'done', model: 'Haiku 4.5', time: '1m 50s', tokens: tok(12), cost: usd(0.02), files: 1 },
    { name: 'fix-login', status: 'idle', model: 'Fable 5.1', time: '0m 00s', tokens: '0', cost: usd(0), files: 0 },
  ];
}

/** pagination-users before it is moved to Opus, in the composer scene: the scenes up to it. */
export const onSonnet = (agents: AgentInfo[]) => agents.map((a) => (a.name === 'pagination-users' ? { ...a, model: 'Sonnet 5.5' } : a));

export function statusOf(lang: Lang): Status {
  const { tr, pct } = fmtOf(lang);
  return {
    active: 3,
    waiting: 0,
    done: 1,
    session: 22,
    weekly: 36,
    sessionReset: tr('2 h 14', '2h14'),
    weeklyReset: tr('3 j', '3d'),
    cost: 3.48,
    procs: { n: 4, mem: tr('1,2 Go', '1.2 GB'), cpu: pct(6) },
    sync: { branch: 'main', behind: 3, ahead: 1 },
  };
}
export const EMPTY_STATUS: Status = { active: 0, waiting: 0, done: 0, session: 0, weekly: 0, cost: 0 };

/** What the scenes share, in a language. */
export interface Demo {
  agents: AgentInfo[];
  status: Status;
  /** The auth refactor's agent, and the CSV export's one. */
  refactor: string;
  csv: string;
}

export const demoOf = (lang: Lang): Demo => ({
  agents: agentsOf(lang),
  status: statusOf(lang),
  refactor: NAMES.refactor[lang],
  csv: NAMES.csv[lang],
});

/** The demo of the picture being drawn: `const { agents, status, refactor } = useDemo()`. */
export const useDemo = (): Demo => demoOf(useLang());
