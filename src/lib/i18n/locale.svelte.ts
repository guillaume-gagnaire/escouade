// The language of the interface. A rune: `t` and the formats of `format.ts` read it, so what is shown from them
// follows a change at once, without reloading the window.

export type Lang = 'fr' | 'en';

/** The language of a locale tag (`fr-FR`, `en-US`): French when its language subtag is, English otherwise (as the backend reads the system's). */
export const langOfTag = (tag: string): Lang => (tag.split(/[-_]/)[0].toLowerCase() === 'fr' ? 'fr' : 'en');

/**
 * `ui`: the language the interface is written in. Until the backend says the one of the settings, a few
 * milliseconds after the window opens, the browser's: the system's, the default setting (what is shown before
 * does not flash in another language). French without a browser; the tests stay French: their setup puts it back
 * before each test.
 */
export const locale: { ui: Lang } = $state({ ui: typeof navigator === 'undefined' ? 'fr' : langOfTag(navigator.language) });
if (typeof document !== 'undefined') document.documentElement.lang = locale.ui;

/** Switches the interface to `lang`, and tells the document (spell checking of the fields, screen readers). */
export function setLang(lang: Lang) {
  locale.ui = lang;
  if (typeof document !== 'undefined') document.documentElement.lang = lang;
}

/** Each language named in itself, as a choice of language shows it whatever the interface's: the same in every catalog. */
export const LANG_NAMES: Record<Lang, string> = { fr: 'Français', en: 'English' };

/** The locale `Intl` formats a language with. */
export const intlLocale = (lang: Lang = locale.ui) => (lang === 'fr' ? 'fr-FR' : 'en-US');
