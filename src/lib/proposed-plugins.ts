// The plugins a project proposes (PLG-14, ESC-288): `plugins.recommended` of its `.escouade.yml`, those not installed on
// this machine and not set aside by « Pas pour ce projet ». The band of Settings › Project names them, each with its
// page in the store; nothing is installed nor turned on but by the user, plugin by plugin.

import { sharedFile } from './shared-file';
import type { Project } from './types';

/** The key of `.escouade.yml` that lists them. */
export const RECOMMENDED = 'plugins.recommended';

/** « Pas pour ce projet », by project: the ids the band showed when the user set it aside. Kept on this machine. */
const KEY = 'escouade.plugins.notForProject';

/** The ids the file proposes, in its order (none when it has none, or cannot be read). */
export function recommendedOf(p: Project | null | undefined): string[] {
  const file = sharedFile(p);
  if (!file || file.error) return [];
  const v = file.values[RECOMMENDED];
  return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
}

/** The ids the band names: proposed, not installed, not set aside for this project. */
export function proposedPlugins(p: Project | null | undefined, installed: (id: string) => boolean, setAside: string[]): string[] {
  return recommendedOf(p).filter((id) => !installed(id) && !setAside.includes(id));
}

export function readSetAside(): Record<string, string[]> {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? '{}');
    return v && typeof v === 'object' && !Array.isArray(v) ? v : {};
  } catch {
    return {};
  }
}

/** Sets `ids` aside for the project `project`: a plugin the file proposes later still shows. */
export function setAside(all: Record<string, string[]>, project: string, ids: string[]): Record<string, string[]> {
  const next = { ...all, [project]: [...new Set([...(all[project] ?? []), ...ids])] };
  try {
    localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    // Not kept: shown again at the next start.
  }
  return next;
}
