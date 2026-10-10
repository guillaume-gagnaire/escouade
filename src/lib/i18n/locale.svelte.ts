// The language of the interface. A rune: `t` and the formats of `format.ts` read it, so what is shown from them
// follows a change at once, without reloading the window.

export type Lang = 'fr' | 'en';

/**
 * `ui`: the language the interface is written in. French until the backend says otherwise (the app was French
 * only up to 1.6, and the tests stay French: their setup puts it back before each test).
 */
export const locale: { ui: Lang } = $state({ ui: 'fr' });

/** Switches the interface to `lang`, and tells the document (spell checking of the fields, screen readers). */
export function setLang(lang: Lang) {
  locale.ui = lang;
  if (typeof document !== 'undefined') document.documentElement.lang = lang;
}

/** The locale `Intl` formats a language with. */
export const intlLocale = (lang: Lang = locale.ui) => (lang === 'fr' ? 'fr-FR' : 'en-US');
