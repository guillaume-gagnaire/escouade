// The store's two languages, as on the site: French at the root, English under /en/.

export type Lang = 'fr' | 'en';

/** The language of a route (`/`, `/en/`, `/plugins/…`, `/en/plugins/…`). */
export function langOfPath(path: string): Lang {
  return /^\/en(?:[/?#]|$)/.test(path) ? 'en' : 'fr';
}

/** The same page as the route `path`, in `lang`: the English pages are the French ones under /en/. */
export function pathIn(lang: Lang, path: string): string {
  const page = path.replace(/[?#].*$/, '').replace(/^\/en(?=\/|$)/, '') || '/';
  const rooted = page.startsWith('/') ? page : `/${page}`;
  return lang === 'en' ? (rooted === '/' ? '/en' : `/en${rooted}`) : rooted;
}

/** The other language, which the footer's link leads to. */
export const otherLang = (lang: Lang): Lang => (lang === 'fr' ? 'en' : 'fr');
