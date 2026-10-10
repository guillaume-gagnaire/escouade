// Closing a project: from its tab's menu or its settings.

import { buffers, lossNotice } from './editor/buffers.svelte';
import { t } from './i18n';
import { api } from './ipc';
import { expectStops, forgetLaunches, testLaunchIds } from './launch-actions';
import { app } from './state.svelte';
import { closeTerminal } from './term-actions';
import type { Project } from './types';

/**
 * Asks before closing the project: it leaves the app with its agents, its files stay on the disk.
 * `back`: where to go once it is closed or not (no modal by default).
 */
export function askCloseProject(p: Project, back?: () => void) {
  app.modal = {
    kind: 'confirm',
    title: t('nav.project.closeTitle', { name: p.name }),
    // The notice of unsaved files is a sentence of its own, added after the question's.
    body: t('nav.project.closeBody') + lossNotice(buffers.unsavedIn(p.id)),
    confirm: t('nav.project.closeConfirm'),
    danger: true,
    onCancel: back,
    onConfirm: async () => {
      await closeProject(p);
      back?.();
    },
  };
}

async function closeProject(p: Project) {
  // The backend kills its launch commands and its agents' test launches: not crashes.
  const runs = [...p.runCommands.map((c) => c.id), ...testLaunchIds(p.id)];
  const undo = expectStops(runs);
  // Only forget the project once the backend removed it.
  const removed = await app.run(api.removeProject(p.id).then(() => true));
  if (!removed) return undo();
  forgetLaunches(runs);
  for (const term of app.terminals.filter((x) => x.projectId === p.id)) closeTerminal(term.id);
  app.forgetProject(p.id);
}
