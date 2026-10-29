// The blink of a project with an agent to look at, on its tile of the projects' rail (ProjectRail): its colour, by what
// the agent did.

import type { AgentStatus } from './types';

/** The color a project blinks in for an agent to look at: red when it failed, amber when it asks, green when it finished. */
export function alertColor(status: AgentStatus): string {
  return status === 'error' ? 'var(--del)' : status === 'waiting' ? 'var(--wait)' : 'var(--run)';
}

/** The blinks, most pressing first: an error, then a question, then an end. */
export const ALERTS = (['error', 'waiting', 'done'] as const).map(alertColor);

/** The most pressing of these blinks (`ALERTS`), or null. */
export function mostPressing(alerts: (string | null)[]): string | null {
  return ALERTS.find((a) => alerts.includes(a)) ?? null;
}
