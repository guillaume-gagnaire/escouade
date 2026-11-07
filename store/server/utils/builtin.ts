import type { Logo } from '../../shared/marks';
import { RESERVED_PREFIX } from '../../../schemas/plugin-manifest.mjs';
import type { Text } from './checks';

// What is left of the plugins the app gave (docs/PLUGINS.md §4.3, INT-4's origin `builtin`). The list is empty since
// ESC-353: each of Escouade's own is a plugin of the store, from `plugins/<short name>/` of the app's repository, published
// by `npm run plugins:publish` (docs/PLUGINS.md §8.6.2), with its versions and its signature, and a line of the catalogue
// like anyone's. Their ids are in the reserve `dev.escouade.*`, which the store grants to the app's repository alone
// (STORE_FIRST_PARTY_REPO, ESC-314): that reserve is what gives them the mark « Escouade » and their tile. The list and
// what reads it go with the origin `builtin` (docs/PLUGINS.md §8.10, Z3).

export interface Builtin {
  id: string;
  name: string;
  categories: string[];
  description: Text;
  /** The hosts its program reaches, as its manifest will declare them; none for a harness (the CLI's own). */
  network: string[];
  /** Whether it keeps the secrets of its accounts (a token, an API key). */
  secrets: boolean;
  /** Its page in Escouade's documentation. */
  docs: Text;
  /** Its tile (PLG-13): its brand's mark where the brand's published rules allow it without asking, else its
   * category's glyph (shared/marks.ts, docs/PLUGINS.md §3.1). */
  logo: Logo;
  /** Its folder in the app's repository, once it is a plugin of its own there (`plugins/<short name>`, INT-4's rule:
   * manifest, README, tests): its page links to it. None for those still in the core. */
  source?: string;
}

/** Where a folder of the app's repository is read. */
export const sourceUrl = (path: string) => `https://github.com/guillaume-gagnaire/escouade/tree/main/${path}`;

export const BUILTINS: Builtin[] = [];

/** Whether `id` is one of the plugins the app gives (not every `dev.escouade.*`: those of `plugins/<id>/` are the store's). */
export const isBuiltinId = (id: string) => BUILTINS.some((b) => b.id === id);

/** Whether `id` is in the reserve: one of Escouade's own, as only the app's repository publishes them (ESC-314). */
export const isOwnId = (id: string) => id.startsWith(RESERVED_PREFIX);

/** The brands whose published rules allow their mark without asking (PLG-13, docs/PLUGINS.md §3.1). */
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
