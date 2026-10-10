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

/** Where the visitor’s choice is kept in the browser; the redirection of the French root reads it. */
export const LANG_KEY = 'escouade-lang';

/**
 * Keeps the visitor’s choice, so that the redirection of the root leaves them where they asked to be. Some
 * browsers refuse to store anything (private mode, cookies blocked), even to hand the storage over: that is
 * no reason to stop the visit, so the storage is asked for here, inside the guard.
 */
export function rememberLanguage(lang: Lang, storage: () => Pick<Storage, 'setItem'> = () => localStorage): void {
  try {
    storage().setItem(LANG_KEY, lang);
  } catch {
    // The choice just won’t be kept.
  }
}
