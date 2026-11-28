import { CATALOGS } from '~/data/catalogs';
import { LANGS, redirectScript, type Lang } from '~/data/language';
import { pagePath, type PageId } from '~/data/privacy';
import { renderDoc, type RenderedDoc } from '~/data/render';
import { SITE, imageOf } from '~/data/site';

const SOURCES = import.meta.glob<string>(
  ['../../content/privacy/*.md', '../../content/legal/*.md', '../../content/mobile/*.md', '../../content/mobile-privacy/*.md'],
  { query: '?raw', import: 'default' },
);

/** A page of its own in `lang`, rendered with the site’s addresses. */
export async function loadPage(id: PageId, lang: Lang, link: (path: string) => string): Promise<RenderedDoc | null> {
  const source = SOURCES[`../../content/${id}/${lang}.md`];
  return source ? renderDoc(await source(), { lang, link, callouts: CATALOGS[lang].docs.callouts }) : null;
}

/**
 * The head of a page of its own: its name and description for search engines and social networks, the address to
 * index and that of the page in the other language. Like the French root, the French page sends a first visit from
 * a browser without French to the English one.
 */
export function usePageHead(id: PageId, o: { title: string; description: string }) {
  const { lang } = useLang();
  const asset = useAsset();
  const url = (l: Lang) => `${SITE}${pagePath(id, l).slice(1)}`;
  const image = `${SITE}${imageOf(lang.value, 'poster.jpg')}`;
  const other = LANGS.find((l) => l.code !== lang.value)!.code;

  useSeoMeta({
    title: o.title,
    description: o.description,
    ogTitle: o.title,
    ogDescription: o.description,
    ogType: 'article',
    ogUrl: url(lang.value),
    ogImage: image,
    ogSiteName: 'Escouade',
    ogLocale: CATALOGS[lang.value].meta.locale,
    ogLocaleAlternate: CATALOGS[other].meta.locale,
    twitterCard: 'summary_large_image',
    twitterTitle: o.title,
    twitterDescription: o.description,
    twitterImage: image,
  });
  useHead({
    htmlAttrs: { lang: lang.value },
    link: [
      { rel: 'canonical', href: url(lang.value) },
      ...LANGS.map((l) => ({ rel: 'alternate' as const, hreflang: l.code, href: url(l.code) })),
      { rel: 'alternate', hreflang: 'x-default', href: url('fr') },
    ],
  });
  if (lang.value === 'fr') {
    useServerHead({ script: [{ innerHTML: redirectScript(asset(pagePath(id, 'en'))), tagPriority: 'critical' }] });
  }
}
