// What the VS Code view's bar says of the file shown in its workbench (ESC-476): how many of its lines changed from
// its base, as the marks of VS Code's quick diff count them. Without monaco nor the app's state: the workbench
// (`scm.ts`) fills it and gives it its comparison, the view's bar (`VscodeView.svelte`) reads it.

/** The file of the active editor of the view, as compared with its base. */
export interface ShownFile {
  projectId: string;
  /** 'project', or the agent whose worktree it is in. */
  source: string;
  /** From the source's root. */
  path: string;
  /** The lines added or changed from the base. */
  count: number;
  /** The base's name (« main », « HEAD »), null while unknown. */
  reference: string | null;
  /** Not in its base: all its lines added. */
  isNew: boolean;
}

class ViewShown {
  file = $state<ShownFile | null>(null);
  private compareBase: (() => Promise<void>) | null = null;

  set(f: ShownFile | null) {
    const same =
      f === this.file ||
      (f &&
        this.file &&
        f.projectId === this.file.projectId &&
        f.source === this.file.source &&
        f.path === this.file.path &&
        f.count === this.file.count &&
        f.reference === this.file.reference &&
        f.isNew === this.file.isNew);
    if (!same) this.file = f;
  }

  /** What « Comparer » of the bar runs: the file shown in VS Code's diff editor, against its base. */
  serve(compare: () => Promise<void>) {
    this.compareBase = compare;
  }

  compare(): Promise<void> {
    return this.compareBase?.() ?? Promise.resolve();
  }
}

export const viewShown = new ViewShown();
