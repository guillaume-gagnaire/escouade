// « Continuer sur <harness> » (ESC-88): an agent goes on on another harness by a handover, as the backend has it
// (`handover.rs`). The window offers what the backend would take; the backend checks it again.

import { engineOrder, usableFor } from './accounts';
import { app } from './state.svelte';
import type { Agent, EngineId } from './types';

type Who = Pick<Agent, 'engine' | 'projectId' | 'handedFrom'>;

/**
 * The harnesses a project runs on, in the order it goes to them (`accounts::fallback_engines`): its default harness, then the
 * others in « Ordre des harness », only those with an active account serving it.
 */
export function fallbackEngines(projectId: string): EngineId[] {
  const project = app.projects.find((p) => p.id === projectId);
  const first = project?.engine ?? app.engines[0]?.id ?? 'claude';
  const out: EngineId[] = [];
  for (const e of [first, ...engineOrder()]) {
    if (!out.includes(e) && usableFor(projectId, e).length) out.push(e);
  }
  return out;
}

/** Whether the harness the agent left is still set aside: its quota has not reset. */
function setAside(agent: Who, engine: EngineId, now: number): boolean {
  const left = agent.handedFrom;
  return !!left && left.engine === engine && left.until !== null && left.until > now;
}

/**
 * The harnesses the agent may be handed over to now (`Core::handover_targets`): its project's fallback harnesses but its own,
 * found here and their plugin on (ESC-391), the one it left not before its quota resets.
 */
export function handoverTargets(agent: Who, now: number = app.now): EngineId[] {
  const own = agent.engine ?? 'claude';
  return fallbackEngines(agent.projectId).filter(
    (e) => e !== own && app.enginesFound.includes(e) && !app.enginesOff.includes(e) && !setAside(agent, e, now),
  );
}

/** The harness the agent left, once it may go back to it (« Revenir sur <harness> »): never by itself. */
export function backTarget(agent: Who, now: number = app.now): EngineId | null {
  const left = agent.handedFrom?.engine;
  return left && handoverTargets(agent, now).includes(left) ? left : null;
}

/** The harnesses offered to go on on that are not the way back. */
export function forwardTargets(agent: Who, now: number = app.now): EngineId[] {
  const back = backTarget(agent, now);
  return handoverTargets(agent, now).filter((e) => e !== back);
}
