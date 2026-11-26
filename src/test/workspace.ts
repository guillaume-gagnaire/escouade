// Builders for the tests of the « Workspace »: plans and pending requests, smallest that the window accepts.
import type {
  BgTask,
  CheckState,
  CommandRun,
  KitUse,
  PendingRequest,
  PermissionItem,
  PlanState,
  PlanTask,
  PrCheck,
  PrDetail,
  ProjectEnvironments,
  QuestionItem,
  SubAgent,
} from '../lib/types';
import type { ArtifactLanding, Ribbon, RibbonState } from '../lib/workspace-landing';

const task = (i: number, status: PlanTask['status']): PlanTask => ({ id: String(i + 1), title: `Tâche ${i + 1}`, status });

export const sub = (id: string, status: SubAgent['status'] = 'running'): SubAgent => ({
  id,
  title: `sous-agent ${id}`,
  status,
  background: false,
  tools: 0,
  startedAt: 1,
});

export const bg = (id: string, status: BgTask['status'] = 'running'): BgTask => ({
  id,
  kind: 'command',
  title: `tâche ${id}`,
  status,
  startedAt: 1,
});

/** A plan of tasks in these states, and nothing else. */
export const planWith = (states: PlanTask['status'][], over: Partial<PlanState> = {}): PlanState => ({
  source: 'tools',
  tasks: states.map((s, i) => task(i, s)),
  agents: [],
  launched: 0,
  workflows: [],
  bgTasks: [],
  ...over,
});

/** A plan with no task, `subs` subagents and `bgs` background tasks running, and one of each ended. */
export const planWithRunning = (subs: number, bgs: number): PlanState =>
  planWith([], {
    agents: [...Array.from({ length: subs }, (_, i) => sub(`s${i}`)), sub('ended', 'done')],
    bgTasks: [...Array.from({ length: bgs }, (_, i) => bg(`b${i}`)), bg('ended', 'done')],
  });

export const req = (id: string, kind: PendingRequest['kind'] = 'question'): PendingRequest => ({
  id,
  kind,
  tool: 'AskUserQuestion',
  arg: '',
  description: null,
  reason: null,
  questions: [{ question: `Question ${id} ?`, options: ['Oui', 'Non'] }],
  cut: false,
  defaultNo: false,
});

/** The conversation's item of a question `id` (as `req(id)` sums it up): « Question <id> ? », « Oui » or « Non ». */
export const questionItem = (id: string, over: Partial<QuestionItem> = {}): QuestionItem => ({
  kind: 'question',
  id,
  toolUseId: `t-${id}`,
  questions: [{ question: `Question ${id} ?`, header: '', multiSelect: false, options: [{ label: 'Oui' }, { label: 'Non' }] }],
  answers: null,
  ts: 1,
  ...over,
});

/** The conversation's item of a permission `id` to run `command`. */
export const permissionItem = (id: string, command: string, over: Partial<PermissionItem> = {}): PermissionItem => ({
  kind: 'permission',
  id,
  toolUseId: `t-${id}`,
  toolName: 'Bash',
  input: { command },
  canAlways: false,
  defaultNo: false,
  decision: null,
  ts: 1,
  ...over,
});

/** A skill or a plugin of a canvas summary, with its calls and those running. */
export const kit = (kind: KitUse['kind'], name: string, calls = 1, running = 0): KitUse => ({
  kind,
  name,
  calls,
  running,
  firstAt: 1,
  lastAt: 1,
});

/** A pull request's detail with nothing in it. */
export const prDetail = (): PrDetail => ({
  checks: [],
  reviews: [],
  comments: [],
  deployments: [],
  can: { checks: true, jobLogs: true, rerun: true, reviews: true, deployments: true },
});

/** A check of a pull request, in `state`, with no time. */
export const check = (name: string, state: CheckState): PrCheck => ({
  name,
  state,
  url: null,
  job: null,
  started: null,
  finished: null,
  cancelled: false,
});

/** A project's environments, none read. */
export const projectEnvs = (): ProjectEnvironments => ({ projectId: 'p1', environments: [], unsupported: false, host: 'GitHub' });

/** An artifact of the landing zone, landed (published) or not. */
export const art = (landed: boolean): ArtifactLanding => ({
  kind: 'artifact',
  id: landed ? 'art:https://claude.ai/artifact/z' : 'art:p',
  url: landed ? 'https://claude.ai/artifact/z' : null,
  title: 'Rapport',
  versions: landed ? 1 : 0,
  at: 1,
});

/** A ribbon card with nothing but its kind and state. */
export const ribbon = (kind: Ribbon['kind'], state: RibbonState): Ribbon => ({
  kind,
  id: `${kind}:${state}`,
  number: null,
  state,
  since: null,
  until: null,
  line: '',
  steps: null,
  total: null,
  failed: null,
  where: '',
  opens: null,
});

/** A terminal call, ended and passed. */
export const cmd = (call: string, command: string, over: Partial<CommandRun> = {}): CommandRun => ({
  call,
  command,
  startedAt: 1,
  endedAt: 9,
  failed: false,
  ...over,
});
