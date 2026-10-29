import { CATALOG_TEXTS } from '~/data/catalog';
import { pathIn } from '~/data/language';
import type { Entry } from '~~/server/utils/listing';

/** The public pages' texts in the page's language, and the addresses of the catalog's pages in it. */
export function useCatalog() {
  const { lang } = useLang();
  const words = computed(() => CATALOG_TEXTS[lang.value]);
  const to = {
    plugin: (id: string) => pathIn(lang.value, `/plugins/${encodeURIComponent(id)}`),
    author: (e: Pick<Entry, 'builtin' | 'author'>) =>
      e.builtin ? pathIn(lang.value, '/authors/escouade') : e.author ? pathIn(lang.value, `/authors/${e.author.login}`) : null,
    search: (q: Record<string, string | undefined> = {}) => {
      const p = new URLSearchParams(Object.entries(q).filter((x): x is [string, string] => !!x[1]));
      const s = p.toString();
      return `${pathIn(lang.value, '/search')}${s ? `?${s}` : ''}`;
    },
  };
  return { lang, words, to };
}
