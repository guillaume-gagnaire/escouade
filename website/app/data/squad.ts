// The hero's demonstration: five agents of one project working at once, as the app shows them.
// Every name, file and number here is made up. Time runs in beats; the statuses repeat every
// cycle, while tokens, durations and spend only ever grow. What is said (names, question, results)
// comes from the catalog of the page's language.
import { CATALOGS } from './catalogs';
import { plural, type SquadText } from './catalog';
import type { Lang } from './language';

export const BEAT_MS = 1200;
export const CYCLE = 24;
/** Where the demo stands still (reduced motion, no JavaScript): work under way, a question, a finished task, a ticket mid-loop. */
export const SNAPSHOT = 7;

export type Model = 'Fable' | 'Opus' | 'Sonnet' | 'Haiku';
export type Status = 'ready' | 'running' | 'question' | 'done' | 'totest';

export interface Activity {
  tool: 'Read' | 'Edit' | 'Write' | 'Grep' | 'Bash';
  target: string;
  /** Lines added and removed, for an edit. */
  diff?: [number, number];
}

export interface Question {
  text: string;
  options: string[];
}

export interface Ticket {
  key: string;
  loop: number;
  max: number;
  /** One per acceptance criterion. */
  met: boolean[];
}

export interface Agent {
  slot: number;
  name: string;
  model: Model;
  status: Status;
  ticket: Ticket | null;
  activity: Activity | null;
  question: Question | null;
  /** What a finished agent ended on. */
  result: string | null;
  tokens: number;
  cost: number;
  /** Active time. */
  ms: number;
}

export interface Project {
  name: string;
  color: string;
  delta: number;
  agents: Status[];
}

export interface Squad {
  project: { name: string; branch: string; delta: number };
  agents: Agent[];
  counts: { running: number; waiting: number; done: number };
  /** Spent today, all projects. */
  today: number;
  /** The projects in the windows behind. */
  projects: Project[];
}

/** The visitor's answer to the question pending at `at` (in beats). */
export interface Answer {
  at: number;
  option: number;
}

/** What a finished agent ended on, before it is put into words. */
type Result = { kind: 'files' | 'tests'; count: number } | { kind: 'met' };

interface Step {
  at: number;
  status: Status;
  activity?: Activity;
  result?: Result;
  /** For the ticket agent: its loop, its criteria, and which of its ticket's files the activity touches. */
  loop?: number;
  met?: boolean[];
  file?: number;
}

/** Tokens per second while running, and dollars per million tokens. */
const RATE: Record<Model, { tps: number; usd: number }> = {
  Fable: { tps: 320, usd: 24 },
  Opus: { tps: 260, usd: 13 },
  Sonnet: { tps: 210, usd: 7.6 },
  Haiku: { tps: 170, usd: 1.7 },
};

/** What the other projects spent today. */
const TODAY_BASE = 1.15;

const run = (at: number, tool: Activity['tool'], target: string, diff?: [number, number]): Step => ({
  at,
  status: 'running',
  activity: diff ? { tool, target, diff } : { tool, target },
});

/** A ticket agent's step on file `file` of its ticket. */
const onTicket = (at: number, tool: Activity['tool'], file: number, loop: number, met: boolean[], diff?: [number, number]): Step => ({
  ...run(at, tool, '', diff),
  file,
  loop,
  met,
});

interface Slot {
  /** Its name in the catalog; none for the ticket agent, named after its ticket. */
  name: keyof SquadText['names'] | null;
  model: Model;
  tokens: number;
  ms: number;
  steps: Step[];
}

const SLOTS: Slot[] = [
  {
    name: 'auth',
    model: 'Opus',
    tokens: 184_000,
    ms: 760_000,
    steps: [
      run(0, 'Read', 'src/auth/middleware.ts'),
      run(3, 'Grep', 'session'),
      run(6, 'Edit', 'src/auth/jwt.ts', [58, 0]),
      run(10, 'Edit', 'src/auth/session.ts', [21, 9]),
      run(14, 'Bash', 'npm test'),
      run(18, 'Read', 'src/routes/login.ts'),
      run(21, 'Edit', 'src/routes/login.ts', [12, 4]),
    ],
  },
  {
    name: 'e2e',
    model: 'Sonnet',
    tokens: 96_000,
    ms: 482_000,
    // Its question and what follows the answer: see ASK and AFTER.
    steps: [run(0, 'Bash', 'npx playwright test --list')],
  },
  {
    name: null,
    model: 'Sonnet',
    tokens: 41_000,
    ms: 195_000,
    steps: [
      onTicket(0, 'Read', 0, 1, [false, false]),
      onTicket(3, 'Edit', 1, 1, [false, false], [34, 6]),
      onTicket(6, 'Edit', 2, 2, [true, false], [18, 2]),
      { ...run(10, 'Bash', 'npm test'), loop: 2, met: [true, false] },
      { at: 13, status: 'totest', result: { kind: 'met' }, loop: 2, met: [true, true] },
      onTicket(18, 'Read', 0, 1, [false, false]),
    ],
  },
  {
    name: 'docs',
    model: 'Haiku',
    tokens: 12_000,
    ms: 110_000,
    steps: [
      { at: 0, status: 'done', result: { kind: 'files', count: 2 } },
      run(8, 'Write', 'docs/api/auth.md'),
      run(11, 'Edit', 'README.md', [6, 1]),
      { at: 13, status: 'done', result: { kind: 'files', count: 2 } },
    ],
  },
  {
    name: 'login',
    model: 'Fable',
    tokens: 22_000,
    ms: 125_000,
    steps: [
      run(0, 'Read', 'src/ui/LoginForm.svelte'),
      run(5, 'Edit', 'src/ui/LoginForm.svelte', [9, 3]),
      run(12, 'Bash', 'npm run check'),
      run(17, 'Edit', 'src/ui/LoginForm.svelte', [2, 2]),
      run(22, 'Bash', 'npm run check'),
    ],
  },
];

/** When tests-e2e asks its question, from `from` until it is answered (by default at `until`, with the first option). */
const ASK = { slot: 1, from: 2, until: 9 };

/** What tests-e2e does after each answer; the first step starts with the answer. */
const AFTER: Step[][] = [
  [
    run(ASK.until, 'Bash', 'npx playwright test'),
    { at: 13, status: 'done', result: { kind: 'tests', count: 38 } },
    run(19, 'Read', 'e2e/checkout.spec.ts'),
  ],
  [
    run(ASK.until, 'Bash', 'npx playwright test e2e/auth'),
    { at: 13, status: 'done', result: { kind: 'tests', count: 12 } },
    run(19, 'Read', 'e2e/checkout.spec.ts'),
  ],
];

/** The files of the tickets the ticket agent takes, in turn, from DEM-4 on (their slugs are in the catalog). */
const TICKETS = [
  { files: ['src/db/users.ts', 'src/routes/users.ts', 'src/routes/users.test.ts'] },
  { files: ['src/db/roles.ts', 'src/routes/users.ts', 'src/routes/roles.test.ts'] },
  { files: ['src/export/csv.ts', 'src/routes/export.ts', 'src/export/csv.test.ts'] },
];
const FIRST_TICKET = 4;
/** When, in a cycle, the ticket agent takes the next ticket. */
const NEXT_TICKET = 18;
const CRITERIA = 2;
const MAX_LOOPS = 5;

/** The projects behind: each mini agent's statuses through a cycle. */
const BEHIND: { name: string; color: string; delta: number; agents: Step[][] }[] = [
  {
    name: 'studio-web',
    color: 'oklch(0.7 0.12 300)',
    delta: 3,
    agents: [
      [{ at: 0, status: 'running' }],
      [
        { at: 0, status: 'running' },
        { at: 12, status: 'done' },
      ],
      [
        { at: 0, status: 'done' },
        { at: 6, status: 'running' },
      ],
    ],
  },
  {
    name: 'mobile-app',
    color: 'oklch(0.75 0.11 215)',
    delta: 12,
    agents: [
      [{ at: 0, status: 'running' }],
      [
        { at: 0, status: 'running' },
        { at: 12, status: 'question' },
        { at: 18, status: 'running' },
      ],
      [{ at: 0, status: 'ready' }],
    ],
  },
];
const DELTA_BASE = 7;

/** The step under way at `b` (beats into the cycle). */
const stepAt = (steps: Step[], b: number) => steps.reduce((cur, s) => (s.at <= b ? s : cur), steps[0]);

/** Beats spent running in [0, b) of a cycle. */
function runningIn(steps: Step[], b: number): number {
  let total = 0;
  steps.forEach((s, i) => {
    const end = Math.min(i + 1 < steps.length ? steps[i + 1].at : CYCLE, b);
    if (s.status === 'running' && end > s.at) total += end - s.at;
  });
  return total;
}

/** The answer given to cycle `c`'s question, if any, as a time in that cycle. */
function answerIn(c: number, answers: Answer[]): { at: number; option: number } | null {
  for (const a of answers) {
    const b = a.at - c * CYCLE;
    if (b >= ASK.from && b < ASK.until) return { at: b, option: a.option };
  }
  return null;
}

/** A slot's steps through cycle `c`, the question and its answer included. */
function stepsOf(slot: number, c: number, answers: Answer[]): Step[] {
  const steps = SLOTS[slot].steps;
  if (slot !== ASK.slot) return steps;
  const answer = answerIn(c, answers);
  const at = answer?.at ?? ASK.until;
  const [first, ...rest] = AFTER[answer?.option ?? 0];
  return [...steps, { at: ASK.from, status: 'question' }, { ...first, at }, ...rest];
}

/** Beats a slot spent running from the start to `t`. */
function runningUntil(slot: number, t: number, answers: Answer[]): number {
  const c = Math.floor(t / CYCLE);
  const plain = runningIn(stepsOf(slot, -1, []), CYCLE);
  let total = 0;
  for (let k = 0; k < c; k++) total += answerIn(k, answers) ? runningIn(stepsOf(slot, k, answers), CYCLE) : plain;
  return total + runningIn(stepsOf(slot, c, answers), t - c * CYCLE);
}

const ticketKey = (n: number) => `DEM-${FIRST_TICKET + n}`;

/** A result in words, in the language of the page. */
function resultText(result: Result, lang: Lang): string {
  const { results } = CATALOGS[lang].squad;
  return result.kind === 'met' ? results.met : plural(lang, results[result.kind], result.count);
}

/** The squad at `t` beats, speaking `lang`. */
export function squadAt(t: number, answers: Answer[] = [], lang: Lang = 'fr'): Squad {
  const text = CATALOGS[lang].squad;
  const c = Math.floor(t / CYCLE);
  const b = t - c * CYCLE;
  let today = TODAY_BASE;

  const agents = SLOTS.map((slot, i): Agent => {
    const rate = RATE[slot.model];
    const step = stepAt(stepsOf(i, c, answers), b);
    const ran = runningUntil(i, t, answers);
    today += ((slot.tokens + rate.tps * ran * (BEAT_MS / 1000)) * rate.usd) / 1e6;

    let name = slot.name ? text.names[slot.name] : '';
    let ticket: Ticket | null = null;
    let activity = step.activity ?? null;
    let since = 0;
    let base = { tokens: slot.tokens, ms: slot.ms };
    if (step.loop) {
      // A new ticket is a new agent: it starts from nothing.
      const n = c + (b >= NEXT_TICKET ? 1 : 0);
      const def = TICKETS[n % TICKETS.length];
      name = `${ticketKey(n).toLowerCase()}-${text.ticketSlugs[n % TICKETS.length]}`;
      ticket = { key: ticketKey(n), loop: step.loop, max: MAX_LOOPS, met: step.met ?? Array(CRITERIA).fill(false) };
      if (activity && step.file !== undefined) activity = { ...activity, target: def.files[step.file] };
      if (n > 0) {
        since = runningUntil(i, (n - 1) * CYCLE + NEXT_TICKET, answers);
        base = { tokens: 0, ms: 0 };
      }
    }
    const beats = ran - since;
    const tokens = Math.round(base.tokens + rate.tps * beats * (BEAT_MS / 1000));
    return {
      slot: i,
      name,
      model: slot.model,
      status: step.status,
      ticket,
      activity,
      question: step.status === 'question' ? text.question : null,
      result: step.result ? resultText(step.result, lang) : null,
      tokens,
      cost: (tokens * rate.usd) / 1e6,
      ms: Math.round(base.ms + beats * BEAT_MS),
    };
  });

  const edits = SLOTS.reduce(
    (n, _, i) =>
      n + stepsOf(i, c, answers).filter((s) => s.at <= b && (s.activity?.tool === 'Edit' || s.activity?.tool === 'Write')).length,
    0,
  );
  const count = (...st: Status[]) => agents.filter((a) => st.includes(a.status)).length;
  return {
    project: { name: 'demo-api', branch: 'main', delta: DELTA_BASE + edits },
    agents,
    counts: { running: count('running'), waiting: count('question'), done: count('done', 'totest') },
    today,
    projects: BEHIND.map((p) => ({ name: p.name, color: p.color, delta: p.delta, agents: p.agents.map((s) => stepAt(s, b).status) })),
  };
}

// Number and duration formatting, as the app's src/lib/format.ts writes them in French; the English
// way is the same figures with a point, a suffix without a space and the dollar sign in front.

/** Each language's units, largest first: where it starts, its decimals, its suffix. */
const TOKEN_UNITS: Record<Lang, [number, number, string][]> = {
  fr: [
    [1e9, 2, ' Md'],
    [1e6, 2, ' M'],
    [1e3, 1, ' k'],
  ],
  en: [
    [1e9, 2, 'B'],
    [1e6, 2, 'M'],
    [1e3, 1, 'k'],
  ],
};

export function fTok(n: number, lang: Lang = 'fr'): string {
  const unit = TOKEN_UNITS[lang].find(([from]) => n >= from);
  if (!unit) return String(Math.round(n));
  const num = (n / unit[0]).toFixed(unit[1]);
  return (lang === 'fr' ? num.replace('.', ',') : num) + unit[2];
}

export function fUsd(x: number, lang: Lang = 'fr'): string {
  const digits = x > 0 && x < 0.1 ? 3 : 2;
  const num = x.toLocaleString(lang === 'fr' ? 'fr-FR' : 'en-US', { minimumFractionDigits: digits, maximumFractionDigits: digits });
  return lang === 'fr' ? num + ' $' : '$' + num;
}

export function fDur(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = s % 60;
  return h ? `${h}h ${String(m).padStart(2, '0')}m` : `${m}m ${String(ss).padStart(2, '0')}s`;
}
