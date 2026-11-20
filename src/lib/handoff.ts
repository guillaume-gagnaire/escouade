// « Prendre » an agent a teammate passes (TS-17, `src-tauri/src/teamhandoff.rs`), from the card of its ticket or from
// the box « À toi »: on the agent's harness when this machine has it for the project, else on the one the user picks
// (a window lists those of this machine). No account ever passes: the agent goes on on this machine's own.
import { t } from './i18n';
import { app } from './state.svelte';
import { teamBoardApi } from './teamboard';
import type { EngineId } from './types';

/** Takes the passage of the ticket `id` by `send` (its harness, or null for the agent's own). Whether it went. */
export async function takeHandoff(id: string, send: (engine: EngineId | null) => Promise<boolean>): Promise<boolean> {
  const choice = await app.run(teamBoardApi.handoffEngines(id));
  if (!choice) return false;
  if (choice.preferred) return send(choice.preferred);
  if (choice.engines.length === 0) {
    app.toast(t('teamBoard.handoff.noEngine'), 'error');
    return false;
  }
  return new Promise((resolve) => {
    app.modal = {
      kind: 'teamHandoffTake',
      ticketId: id,
      engines: choice.engines,
      onPick: (engine) => void send(engine).then(resolve),
      onCancel: () => resolve(false),
    };
  });
}

/** « Prendre » from the card: straight through `ticket_team_handoff_take`. */
export function takeFromCard(id: string): Promise<boolean> {
  return takeHandoff(id, async (engine) => (await app.run(teamBoardApi.handoffTake(id, engine).then(() => true))) ?? false);
}
