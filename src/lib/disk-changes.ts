// The files changed on disk that the core tells (`fsChanged`, its watcher, ESC-447), for whoever follows them: the VS
// Code view's file system provider, so that a file an agent wrote is read again.

export interface DiskChange {
  projectId: string;
  /** Their absolute paths, as the watcher saw them; null: too many to tell, anything may have changed. */
  paths: string[] | null;
}

const listeners = new Set<(c: DiskChange) => void>();

/** Calls `f` at each change told; returns what stops it. */
export function onDiskChange(f: (c: DiskChange) => void): () => void {
  listeners.add(f);
  return () => listeners.delete(f);
}

export function diskChanged(c: DiskChange) {
  for (const f of listeners) f(c);
}
