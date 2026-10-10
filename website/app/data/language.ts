// The site's two languages: French at the root, English under /en/.

export type Lang = 'fr' | 'en';

export interface LangInfo {
  code: Lang;
  /** What the switch shows. */
  label: string;
  /** The language's name in itself, for the switch's tooltip. */
  name: string;
  /** Where its page lives, under the site's base URL. */
  path: string;
}

export const LANGS: readonly LangInfo[] = [
  { code: 'fr', label: 'FR', name: 'Français', path: '' },
  { code: 'en', label: 'EN', name: 'English', path: 'en/' },
];

/** The language of a route of the site (`/` or `/en/`), as the router gives it: without the base URL. */
export function langOfPath(path: string): Lang {
  return /^\/en(?:[/?#]|$)/.test(path) ? 'en' : 'fr';
}
