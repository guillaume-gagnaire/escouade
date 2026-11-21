// The synchronisation of the docs (ESC-534, after ESC-258's « Partager »): the button of the Docs space. With changes of
// the docs to share it shares them as « Partager » did (the remote fetched first, the docs alone committed, the others'
// work merged, pushed), commits not pushed of the code told first and sent only once confirmed; with nothing to share
// it still brings in what the others pushed on the docs branch. A conflict is kept for « L'agent résout » or « Annuler »,
// told by the badge of the Docs tab and shown in the Docs space.
import { docsStore } from './docs.svelte';
import { t } from './i18n';
import { api } from './ipc';
import { app } from './state.svelte';
import type { DocsShared } from './types';

/** What a synchronisation did, told in a toast; a conflict kept for its two gestures. */
function told(projectId: string, done: DocsShared) {
  if (done.kind === 'nothing') app.toast(t('docs.share.nothing'));
  else if (done.kind === 'done') {
    const received = done.received;
    const said = [
      done.committed ? t('docs.share.done', { count: done.committed }) : '',
      received ? t('docs.share.received', { count: received }) : '',
    ].filter(Boolean);
    app.toast(said.length ? said.join(' ') : t('docs.share.nothing'), said.length ? 'ok' : undefined);
  } else app.docsShare[projectId] = { pending: app.docsShare[projectId]?.pending ?? 0, conflict: done.conflict };
}

/** Reads again what is left to share. A read that fails leaves what was known. */
export async function readDocsPending(projectId: string) {
  try {
    const p = await api.docsPending(projectId);
    if (p) app.docsShare[projectId] = { pending: p.count, conflict: p.conflict };
  } catch {
    // Not a git folder, docs branch refused: the Docs space says why.
  }
}

/** One synchronisation, the flag of the button held while it runs. */
async function once(projectId: string, run: () => Promise<DocsShared | undefined>) {
  docsStore.syncing[projectId] = true;
  try {
    const done = await run();
    if (done) told(projectId, done);
  } finally {
    docsStore.syncing[projectId] = false;
  }
  await readDocsPending(projectId);
}

/**
 * « Synchroniser »: what is left to share read first; nothing of the docs to share, what the others pushed is brought in
 * (`docs_pull`); else they are shared (`docs_share`), after a confirmation when code would go with them. Not run again
 * while one runs.
 */
export async function syncDocs(projectId: string) {
  if (docsStore.syncing[projectId]) return;
  docsStore.syncing[projectId] = true;
  let p;
  try {
    p = await app.run(api.docsPending(projectId));
  } finally {
    docsStore.syncing[projectId] = false;
  }
  if (!p) return;
  if (p.count === 0) return once(projectId, () => app.run(api.docsPull(projectId)));
  if (p.codeCommits > 0) {
    app.modal = {
      kind: 'confirm',
      title: t('docs.share.withCodeTitle'),
      body: t('docs.share.withCode', { count: p.codeCommits }),
      confirm: t('docs.share.goAll'),
      onConfirm: () => once(projectId, () => app.run(api.docsShare(projectId, null, true))),
    };
    return;
  }
  await once(projectId, () => app.run(api.docsShare(projectId, null, false)));
}

/** « L'agent résout » the conflict a synchronisation stopped on. */
export async function resolveDocsConflict(projectId: string) {
  const c = await app.run(api.docsShareResolve(projectId));
  if (c) app.docsShare[projectId] = { pending: app.docsShare[projectId]?.pending ?? 0, conflict: c.files.length ? c : null };
}

/** « Annuler » it: the docs as they were before. */
export async function cancelDocsConflict(projectId: string) {
  const done = await app.run(api.docsShareCancel(projectId).then(() => true));
  if (!done) return;
  app.docsShare[projectId] = { pending: app.docsShare[projectId]?.pending ?? 0, conflict: null };
}
