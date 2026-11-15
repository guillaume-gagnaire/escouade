// Closing a project: from its tab's menu or its settings.

import { t } from './i18n';
import { api } from './ipc';
import { expectStops, forgetLaunches, testLaunchIds } from './launch-actions';
import { app } from './state.svelte';
import { closeTerminal } from './term-actions';
import type { Project } from './types';
import { viewUnsaved } from './vscode/unsaved.svelte';

/**
 * Asks before closing the project: it leaves the app with its agents, its history is kept (unless the box is checked)
 * and its files stay on the disk. `back`: where to go once it is closed or not (no modal by default).
 */
export function askCloseProject(p: Project, back?: () => void) {
  app.modal = {
    kind: 'confirm',
    title: t('nav.project.closeTitle', { name: p.name }),
    body: t('nav.project.closeBody'),
    confirm: t('nav.project.closeConfirm'),
    danger: true,
    // Erasing is never the default: the history is what reopening the folder brings back.
    option: { label: t('nav.project.closeErase'), value: false },
    onCancel: back,
    // Its files modified in the VS Code view are saved or given up first, as VS Code asks (ESC-464).
    onConfirm: (erase) =>
      app.askUnsaved(
        viewUnsaved.of(p.id),
        async () => {
          await closeProject(p, erase);
          back?.();
        },
        back,
      ),
  };
}

async function closeProject(p: Project, erase: boolean) {
  // The backend kills its launch commands and its agents' test launches: not crashes.
  const runs = [...p.runCommands.map((c) => c.id), ...testLaunchIds(p.id)];
  const undo = expectStops(runs);
  // Only forget the project once the backend removed it.
  const removed = await app.run(api.removeProject(p.id, erase).then(() => true));
  if (!removed) return undo();
  forgetLaunches(runs);
  for (const term of app.terminals.filter((x) => x.projectId === p.id)) closeTerminal(term.id);
  app.forgetProject(p.id);
}
