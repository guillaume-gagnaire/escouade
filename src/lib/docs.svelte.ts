// What the Docs tab (ESC-258) has read of each project's docs: the spec's tree, the decisions, the documents, the last
// changes and what is left to share. Read when the tab opens, again after a write and when what is left to share changes.
import { t } from './i18n';
import { api } from './ipc';
import { app } from './state.svelte';
import type { DocsDecision, DocsFile, DocsFolder, DocsMoved, DocsPending, DocsTree } from './types';

class DocsStore {
  tree = $state<Record<string, DocsTree>>({});
  decisions = $state<Record<string, DocsDecision[]>>({});
  documents = $state<Record<string, DocsFile[]>>({});
  /** The folders of the drive (ESC-260). */
  folders = $state<Record<string, DocsFolder[]>>({});
  moved = $state<Record<string, DocsMoved[]>>({});
  pending = $state<Record<string, DocsPending>>({});
  /** Why a project's docs could not be read (its docs branch refused, a link in `.escouade`…). */
  error = $state<Record<string, string>>({});
  /** The projects whose « Planifier » runs (ESC-262), from this window. */
  planning = $state<Record<string, boolean>>({});
  /** The plannings whose card the user closed (ESC-291), kept from one start to the next. */
  closedPlans = $state<string[]>(readClosed());
  /** The last load asked for each project: an older answer coming after a newer one is dropped. */
  #asked: Record<string, number> = {};

  /** Reads all a project's docs. A part that fails leaves the others. */
  async load(projectId: string) {
    const n = (this.#asked[projectId] = (this.#asked[projectId] ?? 0) + 1);
    const [tree, decisions, documents, moved, pending] = await Promise.allSettled([
      api.docsSpecTree(projectId),
      api.docsDecisions(projectId),
      api.docsDrive(projectId),
      api.docsRecent(projectId),
      api.docsPending(projectId),
    ]);
    if (this.#asked[projectId] !== n) return;
    if (tree.status === 'fulfilled') {
      this.tree[projectId] = tree.value;
      const comparing = tree.value.comparing ?? [];
      if ((app.docsComparing[projectId] ?? []).join('|') !== comparing.join('|')) app.docsComparing[projectId] = comparing;
      delete this.error[projectId];
    } else {
      this.error[projectId] = String(tree.reason);
    }
    if (decisions.status === 'fulfilled') this.decisions[projectId] = decisions.value;
    if (documents.status === 'fulfilled') {
      this.documents[projectId] = documents.value.files;
      this.folders[projectId] = documents.value.folders;
    }
    if (moved.status === 'fulfilled') this.moved[projectId] = moved.value;
    if (pending.status === 'fulfilled') {
      this.pending[projectId] = pending.value;
      app.docsShare[projectId] = { pending: pending.value.count, conflict: pending.value.conflict };
    }
  }

  /** « Planifier » (ESC-262): the changes of the discussion `agent`, or every change without tickets, made into tickets
   * « À faire » by the planner; what it made is told. */
  async plan(projectId: string, agent: string | null = null) {
    this.planning[projectId] = true;
    const done = await app.run(api.docsPlan(projectId, agent));
    this.planning[projectId] = false;
    if (done) app.toast(t('docs.plan.done', { count: done.made.length }), 'ok');
    await this.load(projectId);
    return done;
  }

  /** Closes the card of a planning; the 50 last closed are kept. */
  closePlan(plan: string) {
    if (this.closedPlans.includes(plan)) return;
    this.closedPlans = [...this.closedPlans, plan].slice(-50);
    try {
      localStorage.setItem(CLOSED_KEY, JSON.stringify(this.closedPlans));
    } catch {
      // Closed for this start only.
    }
  }

  /** « Annuler » of a planning: its tickets not started yet removed. */
  async cancelPlan(projectId: string, plan: string) {
    const done = await app.run(api.docsPlanCancel(projectId, plan));
    if (done) {
      const kept = done.kept.length ? ' ' + t('docs.plan.kept', { count: done.kept.length, keys: done.kept.join(', ') }) : '';
      app.toast(t('docs.plan.cancelled', { count: done.removed.length }) + kept, 'ok');
    }
    await this.load(projectId);
    return done;
  }
}

const CLOSED_KEY = 'escouade.docs.closedPlans';

function readClosed(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(CLOSED_KEY) ?? '[]');
    return Array.isArray(v) ? v.filter((x) => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

export const docsStore = new DocsStore();
