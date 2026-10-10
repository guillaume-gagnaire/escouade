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

/**
 * The language a visit to the French root should end up in. What the visitor asked for comes first: the
 * address (`?lang=`, which the switch’s French link adds), then the choice kept from an earlier visit. Failing
 * that, the browser’s languages: French if any of them is, English if there are some and none is. This runs
 * inline in the page’s head (see `redirectScript`), so it may refer to nothing but its parameters.
 */
export function pickLanguage(browser: readonly string[], stored: string | null, asked: string | null): Lang {
  for (const choice of [asked, stored]) {
    if (choice === 'fr' || choice === 'en') return choice;
  }
  const french = browser.some((l) => /^fr(?:[-_]|$)/i.test(l));
  return browser.length === 0 || french ? 'fr' : 'en';
}

/**
 * The script the French root runs before anything is drawn, so that a visitor who has no French is sent to
 * `target` (the English page) without seeing the French one first. GitHub Pages cannot look at the browser’s
 * languages itself. Without JavaScript, nothing moves. A browser that cannot keep or give back the choice
 * just decides again each time.
 */
export function redirectScript(target: string): string {
  const key = JSON.stringify(LANG_KEY);
  return (
    `(function(){try{var pick=${pickLanguage.toString()};` +
    `var asked=new URLSearchParams(location.search).get('lang');var stored=null;` +
    `try{stored=localStorage.getItem(${key})}catch(e){}` +
    `var langs=navigator.languages&&navigator.languages.length?navigator.languages:navigator.language?[navigator.language]:[];` +
    `if(asked==='fr'||asked==='en'){try{localStorage.setItem(${key},asked)}catch(e){}}` +
    `if(pick(langs,stored,asked)==='en')location.replace(${JSON.stringify(target)})}catch(e){}})()`
  );
}
