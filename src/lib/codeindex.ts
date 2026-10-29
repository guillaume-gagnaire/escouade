import type { Project, Settings } from './types';

// « Index du code » of a project (ESC-293): its own choice, or « Index du code sur tous les projets » when it made none,
// as `Core::code_index_on` decides it in the backend. Every reader of the window asks here.

/** Whether the project's index is on. */
export function codeIndexOn(
  project: Pick<Project, 'codeIndexChoice'>,
  settings: Pick<Settings, 'codeIndexAll'> | null | undefined,
): boolean {
  return project.codeIndexChoice ?? !!settings?.codeIndexAll;
}

/** The projects that follow the app's setting (no choice of their own): those it reads, or erases. */
export function codeIndexFollowers<P extends Pick<Project, 'codeIndexChoice'>>(projects: P[]): P[] {
  return projects.filter((p) => p.codeIndexChoice == null);
}
