// What lands in an agent's landing zone, south of its compass (spec docs/superpowers/specs/2026-11-25-vue-workspace-design.md,
// « Au sud : la zone d'atterrissage » and « Choix validés »): the pages it published, its remote CI, its deployments.
// Local tests are not here: they stick under the card that runs them (`workspace-tests.ts`). Pure.

import type { Agent, PrDetail, ProjectEnvironments } from './types';

/** `stopped`: a test run stopped before its end, neither passed nor failed. */
export type RibbonState = 'running' | 'success' | 'failure' | 'stopped';
export type Step = { name: string; state: RibbonState };
/** What the card's button opens: a background task's output, a call in the conversation, an address. */
export type Opens = { kind: 'task'; task: string } | { kind: 'call'; call: string } | { kind: 'url'; url: string } | null;
/** A card with a ribbon: tests, CI or a deployment. Times in ms; a branch, never a commit. */
export type Ribbon = {
  kind: 'tests' | 'ci' | 'deploy';
  id: string;
  number: number | null;
  state: RibbonState;
  since: number | null;
  until: number | null;
  line: string;
  steps: Step[] | null;
  total: number | null;
  failed: number | null;
  /** The host of the address the button opens, '' without one. */
  where: string;
  opens: Opens;
};
/** A page published with `Artifact`: `url` null while it is being published. */
export type ArtifactLanding = { kind: 'artifact'; id: string; url: string | null; title: string; versions: number; at: number };
export type Landing = ArtifactLanding | Ribbon;
export type LandingCtx = { pr: PrDetail | null; environments: ProjectEnvironments | null };

/** Landed, else coming: hovering over its slot. */
export const landed = (l: Landing): boolean => (l.kind === 'artifact' ? l.url !== null : l.state !== 'running');

const hostOf = (url: string | null | undefined): string => {
  try {
    return url ? new URL(url).host : '';
  } catch {
    return '';
  }
};

const ribbon = (over: Partial<Ribbon> & Pick<Ribbon, 'kind' | 'id' | 'state' | 'line'>): Ribbon => ({
  number: null,
  since: null,
  until: null,
  steps: null,
  total: null,
  failed: null,
  where: '',
  opens: null,
  ...over,
});

const urlOpens = (url: string | null | undefined): Pick<Ribbon, 'opens' | 'where'> =>
  url ? { opens: { kind: 'url', url }, where: hostOf(url) } : { opens: null, where: '' };

function deployState(state: string): RibbonState {
  if (['success', 'active', 'inactive'].includes(state)) return 'success';
  if (['failure', 'error'].includes(state)) return 'failure';
  return 'running';
}

export function landingOf(agent: Agent, ctx: LandingCtx): Landing[] {
  const out: Landing[] = [];
  const c = agent.canvas;
  for (const a of [...(c?.artifacts ?? [])].sort((x, y) => y.at - x.at)) {
    out.push({ kind: 'artifact', id: `art:${a.url}`, url: a.url, title: a.title, versions: a.versions, at: a.at });
  }
  for (const p of c?.publishing ?? []) {
    out.push({ kind: 'artifact', id: `art:${p.call}`, url: null, title: p.name, versions: 0, at: p.since });
  }

  const pr = agent.prs.find((p) => p.state === 'open');
  if (pr && pr.checks) {
    const checks = ctx.pr?.checks ?? [];
    const steps = ctx.pr
      ? checks.map((k) => ({
          name: k.name,
          state: (k.cancelled || k.state === 'failure' ? 'failure' : k.state === 'pending' ? 'running' : 'success') as RibbonState,
        }))
      : null;
    const started = checks.flatMap((k) => (k.started === null ? [] : [k.started]));
    const finished = checks.flatMap((k) => (k.finished === null ? [] : [k.finished]));
    const allDone = checks.length > 0 && finished.length === checks.length;
    out.push(
      ribbon({
        kind: 'ci',
        id: `ci:pr:${pr.number}`,
        number: pr.number,
        state: pr.checks === 'pending' ? 'running' : pr.checks === 'failure' ? 'failure' : 'success',
        line: pr.head,
        steps,
        since: started.length ? Math.min(...started) * 1000 : null,
        until: allDone ? Math.max(...finished) * 1000 : null,
        ...urlOpens(pr.url),
      }),
    );
  } else if (!pr && agent.branchCi?.checks) {
    const b = agent.branchCi;
    const checks = b.checks;
    out.push(
      ribbon({
        kind: 'ci',
        id: `ci:branch:${b.branch}`,
        state: checks === 'pending' ? 'running' : checks === 'failure' ? 'failure' : 'success',
        line: b.branch,
        total: b.checksTotal,
        failed: b.checksFailed,
        ...urlOpens(b.failed[0]?.url),
      }),
    );
  }

  const branch = agent.worktree?.branch ?? agent.foundWorktree?.branch ?? '';
  const envs = new Set<string>();
  // Those of the open pull request only: a detail read before it closed lands nothing. The host gives the latest first:
  // one an environment, the latest.
  for (const d of pr ? (ctx.pr?.deployments ?? []) : []) {
    if (envs.has(d.environment)) continue;
    envs.add(d.environment);
    out.push(
      ribbon({
        kind: 'deploy',
        id: `deploy:${d.environment}`,
        state: deployState(d.state),
        line: `${pr?.head ?? branch} → ${d.environment}`,
        ...urlOpens(d.url),
      }),
    );
  }
  for (const { deployment: d, agents } of ctx.environments?.branches ?? []) {
    if (!agents.includes(agent.id) || envs.has(d.environment)) continue;
    envs.add(d.environment);
    out.push(
      ribbon({
        kind: 'deploy',
        id: `deploy:${d.environment}`,
        state: deployState(d.state),
        line: `${branch} → ${d.environment}`,
        since: d.created ? d.created * 1000 : null,
        ...urlOpens(d.log ?? d.url),
      }),
    );
  }
  return out;
}
