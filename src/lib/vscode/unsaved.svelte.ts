// The files modified in the VS Code view and not saved yet (ESC-464), as its workbench tells them. Without monaco nor
// the app's state: the workbench (`workbench.ts`) fills it and gives it its actions, the app reads it (the counts of
// the warnings, « Quitter ») and asks through it before a close (`app.askUnsaved`).

/** A file of a source modified in the view. */
export interface Unsaved {
  projectId: string;
  /** 'project', or the agent whose worktree it is in. */
  source: string;
  /** From the source's root. */
  path: string;
  /** The URI VS Code knows it by. */
  uri: string;
}

/**
 * What the workbench does with them: save them (true when all are saved), read them again from the disk; and a closed
 * project's folders and tabs taken out of it.
 */
export interface UnsavedActions {
  save(uris: string[]): Promise<boolean>;
  revert(uris: string[]): Promise<void>;
  forget(projectId: string): Promise<void>;
}

class ViewUnsaved {
  files = $state<Unsaved[]>([]);
  private actions: UnsavedActions | null = null;
  private listeners = new Set<() => void>();

  /** The workbench's list, as it changed. */
  set(list: Unsaved[]) {
    const same = list.length === this.files.length && list.every((f, i) => f.uri === this.files[i].uri);
    if (same) return;
    this.files = list;
    for (const f of this.listeners) f();
  }

  /** Told when the list changes (the count « Quitter » reads). */
  onChange(f: () => void): () => void {
    this.listeners.add(f);
    return () => this.listeners.delete(f);
  }

  /** The workbench's actions, once it started. */
  serve(actions: UnsavedActions | null) {
    this.actions = actions;
  }

  /** Those of a project, or of one of its sources; all of them without a project. */
  of(projectId?: string, source?: string): Unsaved[] {
    return this.files.filter(
      (f) => (projectId === undefined || f.projectId === projectId) && (source === undefined || f.source === source),
    );
  }

  count(projectId?: string, source?: string): number {
    return this.of(projectId, source).length;
  }

  /** Saves them: true when none is left unsaved. */
  async save(files: Unsaved[]): Promise<boolean> {
    if (!files.length) return true;
    return (await this.actions?.save(files.map((f) => f.uri))) ?? false;
  }

  /** Gives their changes up: read again from the disk. */
  async revert(files: Unsaved[]): Promise<void> {
    if (files.length) await this.actions?.revert(files.map((f) => f.uri));
  }

  /** A closed project out of the workbench, its changes saved or given up already. */
  async forget(projectId: string): Promise<void> {
    await this.actions?.forget(projectId);
  }

  reset() {
    this.files = [];
    this.actions = null;
    for (const f of this.listeners) f();
  }
}

export const viewUnsaved = new ViewUnsaved();
