// The modes of the new agents the first launch offers (SEC-09, ESC-504): four, the recommended one first
// (`RECOMMENDED_MODE`: edits in the agent's folder pass, every command asks). « Bypass » is never offered there: it
// waits for « Autoriser le mode Bypass » in Réglages › Agents.
/** The mode the first launch recommends, and a new installation's until it is answered (`accounts::RECOMMENDED_MODE`). */
export const RECOMMENDED_MODE = 'acceptEdits';

export const FIRST_MODES = [RECOMMENDED_MODE, 'default', 'plan', 'auto'] as const;
export type FirstMode = (typeof FIRST_MODES)[number];
