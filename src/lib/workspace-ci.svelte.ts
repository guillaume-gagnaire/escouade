// What the « Workspace » reads for its landing zones (spec docs/superpowers/specs/2026-11-25-vue-workspace-design.md,
// « D'où viennent les données »): the detail of an agent's open pull request (its checks, its deployments), for the
// compasses on screen, and the project's environments, followed while the canvas is open as the view « Environnements ».

import { api } from './ipc';
import { app } from './state.svelte';
import type { Agent, PrDetail } from './types';

/** The detail of each agent's open pull request, as last read well. */
export const prDetails = $state<Record<string, PrDetail>>({});

/** The pull request each agent's read was last asked for, and when: a compass mounted again reads it once a minute. */
const asked: Record<string, { key: string; readAt: number }> = {};

/**
 * Reads the detail of the agent's open pull request; one that cannot be read leaves what was there. An answer for a pull
 * request that is no longer the one asked last (closed, or its checks moved on) is dropped.
 */
export async function readPrDetail(agent: Agent): Promise<void> {
  const pr = agent.prs.find((p) => p.state === 'open');
  if (!pr) {
    delete asked[agent.id];
    delete prDetails[agent.id];
    return;
  }
  const key = `${pr.hostRepo ?? ''}#${pr.number}:${pr.checks}`;
  const last = asked[agent.id];
  if (last?.key === key && Date.now() - last.readAt < 60_000) return;
  const mine = { key, readAt: Date.now() };
  asked[agent.id] = mine;
  try {
    const r = await api.agentPrDetail(agent.id, pr.number, pr.hostRepo);
    if (r.data && asked[agent.id] === mine) prDetails[agent.id] = r.data;
  } catch {
    // The host out of reach: the card keeps the pull request's state.
  }
}

// An agent removed: nothing kept of it.
app.onAgentRemoved((id) => {
  delete asked[id];
  delete prDetails[id];
});

/** How often the project is said followed (the core forgets it after 15 minutes). */
const KEEP_EVERY = 5 * 60 * 1000;

/** What `EnvironmentsView` does while open: its environments read, the core told to follow them. Returns the stop. */
export function followEnvironments(projectId: string): () => void {
  void (async () => {
    const p = await api.projectEnvironments(projectId).catch(() => null);
    if (!p) return;
    app.environments[projectId] = p;
    // Read when never read, or read more than a minute ago; a host without deployments never. In the background: a
    // host out of reach says nothing here, the view « Environnements » tells it.
    if (p.unsupported || (p.readAt && Date.now() - p.readAt <= 60_000)) return;
    const read = await api.projectEnvironmentsRead(projectId).catch(() => null);
    if (read) app.environments[projectId] = read;
  })();
  const keep = setInterval(() => void api.projectEnvironments(projectId).catch(() => {}), KEEP_EVERY);
  return () => clearInterval(keep);
}
