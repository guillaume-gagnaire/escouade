// Réglages › Projets › Kanban › Workflow (ESC-195): what the reading found (its sources, its workflows), and the steps as the
// user corrects them, ahead of the reading. The rows edited here make the project's `workflowFixes`; the backend applies
// them the same way (`workflowfix.rs`) for what follows the agents.
import type { Workflow, WorkflowFix, WorkflowFixStep, WorkflowScan, WorkflowSignal } from './types';

/** A step as the settings list it. */
export interface StepRow {
  /** What the list knows it by while it is edited (its first step read, else « new-n »). */
  key: string;
  name: string;
  /** The steps of the reading it stands for. */
  from: string[];
  signals: WorkflowSignal[];
  /** Its commands and subagents, as read: what tells it. */
  marks: string[];
  /** « deviné » by the reading, not kept yet. */
  guessed: boolean;
  /** A correction of steps the reading no longer has: said, then left out. */
  stale: boolean;
}

export const signalValue = (s: WorkflowSignal) => ('command' in s ? s.command : 'agent' in s ? s.agent : s.file);
export const signalKind = (s: WorkflowSignal): 'command' | 'agent' | 'file' =>
  'command' in s ? 'command' : 'agent' in s ? 'agent' : 'file';
export const signalOf = (kind: 'command' | 'agent' | 'file', value: string): WorkflowSignal =>
  kind === 'command' ? { command: value } : kind === 'agent' ? { agent: value } : { file: value };

/** The corrections of `w` among `fixes`. */
export const fixOf = (fixes: WorkflowFix[] | undefined, w: Workflow) => fixes?.find((f) => f.workflow === w.id);

function marksOf(w: Workflow, from: string[], signals: WorkflowSignal[]): string[] {
  const out: string[] = [];
  for (const n of from) {
    const s = w.steps.find((x) => x.name === n);
    if (!s) continue;
    if (s.command) out.push(s.command);
    for (const a of s.agents) out.push(a.name);
  }
  for (const g of signals) if (signalValue(g).trim()) out.push(signalValue(g).trim());
  return [...new Set(out)];
}

/** The steps of `w` as corrected by `fix`, in their order: those listed, then those read since and not taken out. */
export function stepRows(w: Workflow, fix: WorkflowFix | undefined): StepRow[] {
  const read = (name: string) => w.steps.find((s) => s.name === name);
  const rows: StepRow[] = [];
  const listed = new Set<string>();
  let added = 0;
  for (const s of fix?.steps ?? []) {
    const from = s.from ?? [];
    const signals = s.signals ?? [];
    from.forEach((n) => listed.add(n));
    const found = from.filter((n) => read(n));
    rows.push({
      key: from[0] ?? `new-${added++}`,
      name: s.name ?? from[0] ?? '',
      from,
      signals,
      marks: marksOf(w, from, signals),
      guessed: false,
      stale: from.length > 0 && found.length === 0 && signals.length === 0,
    });
  }
  for (const s of w.steps) {
    if (listed.has(s.name) || fix?.removed?.includes(s.name)) continue;
    rows.push({
      key: s.name,
      name: s.name,
      from: [s.name],
      signals: [],
      marks: marksOf(w, [s.name], []),
      guessed: !!s.guessed,
      stale: false,
    });
  }
  return rows;
}

/**
 * The corrections the rows make of `w`, `removed` the steps read taken out: undefined when they are the reading as it
 * is. A guessed step not kept is left out of them: it stays guessed, after the others.
 */
export function fixFrom(w: Workflow, rows: StepRow[], removed: string[], dismissed = false): WorkflowFix | undefined {
  const listed = rows.filter((r) => !r.guessed);
  const plain = (r: StepRow, name: string) => r.from.length === 1 && r.from[0] === name && r.name.trim() === name && !r.signals.length;
  const expected = w.steps.filter((s) => !removed.includes(s.name) && !s.guessed);
  const asIs = listed.length === expected.length && listed.every((r, i) => plain(r, expected[i].name));
  if (!dismissed && !removed.length && asIs) return undefined;
  const steps: WorkflowFixStep[] = listed.map((r) => ({
    name: r.name,
    ...(r.from.length ? { from: r.from } : {}),
    ...(r.signals.length ? { signals: r.signals } : {}),
  }));
  return {
    workflow: w.id,
    ...(dismissed ? { dismissed: true } : {}),
    ...(asIs ? {} : { steps }),
    ...(removed.length ? { removed: [...new Set(removed)] } : {}),
  };
}

/** `fixes` with the corrections of `w` replaced by `fix` (taken out when undefined). */
export function withFix(fixes: WorkflowFix[] | undefined, w: Workflow, fix: WorkflowFix | undefined): WorkflowFix[] {
  const others = (fixes ?? []).filter((f) => f.workflow !== w.id);
  return fix ? [...others, fix] : others;
}

// ---------- what the user does to the rows ----------

/** « Fusionner avec la suivante »: one row, the name of the first, what tells both. */
export function merge(rows: StepRow[], i: number): StepRow[] {
  const [a, b] = [rows[i], rows[i + 1]];
  if (!a || !b) return rows;
  const merged: StepRow = {
    ...a,
    from: [...a.from, ...b.from],
    signals: [...a.signals, ...b.signals],
    marks: [...new Set([...a.marks, ...b.marks])],
    guessed: false,
    stale: a.stale && b.stale,
  };
  return [...rows.slice(0, i), merged, ...rows.slice(i + 2)];
}

/** The row `i` moved to `j`. */
export function move(rows: StepRow[], i: number, j: number): StepRow[] {
  if (i === j || i < 0 || j < 0 || i >= rows.length || j >= rows.length) return rows;
  const out = [...rows];
  const [r] = out.splice(i, 1);
  out.splice(j, 0, r);
  return out;
}

/** The steps read that a list of rows no longer has, `before` the rows as they were: taken out. */
export function removedBy(before: StepRow[], after: StepRow[], removed: string[]): string[] {
  const still = new Set(after.flatMap((r) => r.from));
  return [...new Set([...removed, ...before.flatMap((r) => r.from).filter((n) => !still.has(n))])];
}

/** The name of a workflow in the list: with its repository's in a project of several (« api · GSD »). */
export const workflowName = (w: Pick<Workflow, 'name' | 'repo'>) => (w.repo ? `${w.repo} · ${w.name}` : w.name);

/** Its subagents, each once. */
export const agentsOf = (w: Workflow) => new Set(w.steps.flatMap((s) => s.agents.map((a) => a.name))).size;

/** What the reading looked at, grouped by folder for what the harnesses keep in folders, the rest one by one. */
export interface SourceGroup {
  path: string;
  count: number;
  /** Files reached by an `@`. */
  included: boolean;
}

const GROUPED = new Set(['command', 'skill', 'agent', 'include']);

/** A path of the reading as the user knows it: from the project's folder, `~/.claude/…`, `~/…`. */
export function shownPath(scan: WorkflowScan, path: string): string {
  const under = (dir: string | undefined) => {
    const d = dir?.replace(/\/+$/, '');
    return d && path.toLowerCase().startsWith(d.toLowerCase() + '/') ? path.slice(d.length + 1) : null;
  };
  const inRoot = under(scan.root);
  if (inRoot !== null) return inRoot;
  const inClaude = under(scan.claude);
  if (inClaude !== null) return `~/.claude/${inClaude}`;
  const inHome = under(scan.home);
  return inHome !== null ? `~/${inHome}` : path;
}

export function sourceGroups(scan: WorkflowScan): SourceGroup[] {
  const out: SourceGroup[] = [];
  for (const s of scan.sources) {
    if (!s.modified && !s.len) continue;
    const grouped = GROUPED.has(s.kind);
    const shown = shownPath(scan, s.path);
    const path = grouped ? shown.replace(/\/(?:[^/]+\/SKILL\.md|[^/]+)$/, '') : shown;
    const included = s.kind === 'include';
    const g = out.find((x) => x.path === path && x.included === included);
    if (g) g.count++;
    else out.push({ path, count: 1, included });
  }
  // The folders first, the most files first; then the files.
  return out.sort((a, b) => Number(b.count > 1) - Number(a.count > 1) || b.count - a.count);
}
