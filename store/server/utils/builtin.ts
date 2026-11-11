import type { Logo } from '../../shared/marks';
import { RESERVED_PREFIX } from '../../../schemas/plugin-manifest.mjs';

// Escouade's own plugins on the store: each is a plugin of the store, from `plugins/<short name>/` of the app's
// repository, published by `npm run plugins:publish` (docs/PLUGINS-DESIGN.md §8.6.2), with its versions and its signature, and a
// line of the catalogue like anyone's (the plugins the app gave, `BUILTINS`, left with the origin `builtin`, ESC-355).
// Their ids are in the reserve `dev.escouade.*`, which the store grants to the app's repository alone
// (STORE_FIRST_PARTY_REPO, ESC-314): that reserve is what gives them the mark « Escouade » and their tile.

/** Whether `id` is in the reserve: one of Escouade's own, as only the app's repository publishes them (ESC-314). */
export const isOwnId = (id: string) => id.startsWith(RESERVED_PREFIX);

/** The brands whose published rules allow their mark without asking (PLG-13, docs/PLUGINS-DESIGN.md §3.1). */
const BRANDS: Record<string, Logo> = {
  'dev.escouade.github': { brand: 'github' },
  'dev.escouade.gitea': { brand: 'gitea' },
  'dev.escouade.opencode': { brand: 'opencode' },
};

/** The tile of one of Escouade's own: its brand's mark, else its category's glyph; null when it has neither. */
export function ownLogo(id: string, categories: readonly string[]): Logo | null {
  if (!isOwnId(id)) return null;
  if (BRANDS[id]) return BRANDS[id]!;
  if (categories.includes('forges')) return { category: 'forge' };
  if (categories.includes('ticket-sources')) return { category: 'source' };
  return categories.includes('harness') ? { category: 'harness' } : null;
}
