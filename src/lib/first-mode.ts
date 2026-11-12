// The question of the first start (SEC-09): the permission mode of the new agents, among four, the recommended one first
// (`RECOMMENDED_MODE`: edits in the agent's folder pass, every command asks). « Bypass » is never offered there: it
// waits for « Autoriser le mode Bypass » in Réglages › Agents.
/** The mode the question recommends, and a new installation's until it is answered (`accounts::RECOMMENDED_MODE`). */
export const RECOMMENDED_MODE = 'acceptEdits';

export const FIRST_MODES = [RECOMMENDED_MODE, 'default', 'plan', 'auto'] as const;
export type FirstMode = (typeof FIRST_MODES)[number];

/** Whether the question is still to ask: a new installation that has not answered it (an existing one has `auto`). */
export function modeToAsk(settings: { agentMode?: string | null }): boolean {
  return !settings.agentMode?.trim();
}
