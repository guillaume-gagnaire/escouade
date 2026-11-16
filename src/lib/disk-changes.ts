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

// The git state of a project the core told changed (`git`, its git loop: a commit, a checkout, a base that moved): the
// VS Code view reads its bases and its files changed again (ESC-476).
const gitListeners = new Set<(projectId: string) => void>();

/** Calls `f` with the project at each git state told; returns what stops it. */
export function onGitChange(f: (projectId: string) => void): () => void {
  gitListeners.add(f);
  return () => gitListeners.delete(f);
}

export function gitChanged(projectId: string) {
  for (const f of gitListeners) f(projectId);
}
