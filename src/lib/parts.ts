// The parts of a project switched on or off (ESC-658, spec « Les parties d'un projet »): the window's side of the core's
// `escouade_model::parts`. A part off takes away, in its project alone, its tab, its buttons and what shows it; its MCP
// tools, what its agents are told and what it does in the background are the core's to take away. Its data stay.
//
// Who says it, the first who says it winning: the team that imposes it, this device (its page of the settings,
// Réglages › Projets › Parties du projet), the repository's .escouade.yml (`parts.<id>`, « proposé par le dépôt »), the
// team that recommends it, else on.

import type { IconName } from './icons';
import type { Feature, Part, Project } from './types';

/** The parts, in the order of the page (the core's `Part::ALL`). */
export const PARTS: Part[] = ['docs', 'memory', 'kanban', 'launch'];

/** The feature of the whole app each is of: off there, it is off in every project. */
export const PART_FEATURE: Partial<Record<Part, Feature>> = { docs: 'docs', kanban: 'kanban' };

export const PART_ICONS: Record<Part, IconName> = {
  docs: 'file',
  memory: 'read',
  kanban: 'kanban',
  launch: 'play',
};

/** Who says whether a part is on (`PartBy`). */
export type PartBy = 'default' | 'teamDefault' | 'repo' | 'device' | 'teamEnforced';

export interface PartState {
  part: Part;
  on: boolean;
  by: PartBy;
  /** The team's name, when it is the team that says it. */
  team?: string;
}

/** Its key in a `.escouade.yml`. */
export const partKey = (part: Part) => `parts.${part}`;

const said = (values: Record<string, unknown> | null | undefined, part: Part): boolean | undefined => {
  const v = values?.[partKey(part)];
  return typeof v === 'boolean' ? v : undefined;
};

/** Whether `part` is on in `p` (its own switch, whatever the feature of the app says), and who says so. */
export function partState(p: Project | null | undefined, part: Part): PartState {
  const team = p?.shared?.teamParts ?? null;
  const at = (on: boolean, by: PartBy): PartState =>
    by === 'teamDefault' || by === 'teamEnforced' ? { part, on, by, team: team?.team ?? '' } : { part, on, by };
  const enforced = said(team?.enforced, part);
  if (enforced !== undefined) return at(enforced, 'teamEnforced');
  const device = p?.parts?.[part];
  if (typeof device === 'boolean') return at(device, 'device');
  const repo = said(p?.shared?.file?.values, part);
  if (repo !== undefined) return at(repo, 'repo');
  const recommended = said(team?.defaults, part);
  if (recommended !== undefined) return at(recommended, 'teamDefault');
  return at(true, 'default');
}

/** Whether `part` is on in `p`: its own switch alone (the app's feature is `featureOn`'s). A project unknown has it. */
export function partOn(p: Project | null | undefined, part: Part): boolean {
  return partState(p, part).on;
}
