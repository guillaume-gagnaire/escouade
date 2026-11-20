// The features switched on or off as a whole (F-1, ESC-550, docs/FEATURES.md § 1, spec « Fonctionnalités »): the window's
// side of the core's register (`escouade_model::features`). The state comes from the core, in the settings: a client of
// a server shows its core's. A feature off takes with it its views, tabs, buttons, menus, shortcuts and its place in
// the palette (each reads `featureOn`); its MCP tools, its discoveries and what it does in the background are the
// core's to take away. Its data stay, and come back when it is on again.

import type { IconName } from './icons';
import type { Feature, Settings } from './types';

/** The features, in the order of Réglages › Fonctionnalités (the core's `Feature::ALL`). */
export const FEATURES: Feature[] = ['kanban', 'editor', 'docs', 'cost', 'stats', 'workflows', 'scheduled', 'overseer', 'voice'];

/** The feature each needs: a scheduled task's run is a ticket, which the Kanban holds. */
export const NEEDS: Partial<Record<Feature, Feature>> = { scheduled: 'kanban' };

/** Each one's icon in Réglages › Fonctionnalités. */
export const FEATURE_ICONS: Record<Feature, IconName> = {
  kanban: 'kanban',
  editor: 'edit',
  docs: 'file',
  cost: 'coin',
  stats: 'stats',
  workflows: 'flow',
  scheduled: 'clock',
  overseer: 'sparkle',
  voice: 'mic',
};

/** The row of its switch in the register of the settings (`SETTINGS_ROWS`, the policies' `rows`): `featureKanban`. */
export const featureRow = (f: Feature) => `feature${f[0].toUpperCase()}${f.slice(1)}`;

/** Its own switch, whatever it needs: on until the core settled them. */
export function featureSwitch(s: Pick<Settings, 'features'> | null | undefined, f: Feature): boolean {
  return s?.features?.[f] !== false;
}

/** Whether `f` is on: its switch, and the one of the feature it needs (`Features::on` of the core). */
export function featureOn(s: Pick<Settings, 'features'> | null | undefined, f: Feature): boolean {
  const need = NEEDS[f];
  return featureSwitch(s, f) && (!need || featureSwitch(s, need));
}
