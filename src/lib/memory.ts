import { api } from './ipc';
import { app } from './state.svelte';

// The shared memory of a project's agents (ESC-255): where it lives, and the offer of what Claude Code remembers of
// the project, made once.

/** Where the memory lives, in the project's folder. */
export const MEMORY_DIR = '.escouade/memory/';

/** The projects whose offer was asked of the backend in this session (and is not to be asked again). */
const asked = new Set<string>();

/**
 * Offers what Claude Code remembers of `projectId` to its shared memory, once: not when it was offered already (the
 * project says so), nor while another window is open (asked again once it closes), nor when Claude Code has nothing.
 */
export async function offerClaudeMemory(projectId: string | null): Promise<void> {
  if (!projectId || asked.has(projectId) || app.modal) return;
  const project = app.projects.find((p) => p.id === projectId);
  // Its shared memory off (ESC-658): nothing offered to it.
  if (!project || project.claudeMemoryAsked || !app.partOn(projectId, 'memory')) return;
  asked.add(projectId);
  const offered = await api.claudeMemoryOffer(projectId).catch(() => []);
  if (!offered?.length) return;
  if (app.modal || app.ui.activeProject !== projectId) {
    // Another window came first, or the user went elsewhere: offered at the next visit.
    asked.delete(projectId);
    return;
  }
  app.modal = { kind: 'claudeMemory', projectId, offered };
}

/** For the tests: every project may be offered again. */
export function forgetOffers() {
  asked.clear();
}
