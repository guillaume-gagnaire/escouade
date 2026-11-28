// The site’s pages of their own, the privacy page, the legal notice, the page of the mobile app and its privacy
// policy (MO-8): one Markdown file per language, content/<page>/<lang>.md, rendered like a page of the documentation
// (data/render.ts) but standing on their own, at /privacy/, /legal/, /mobile/ and /mobile/privacy/ (and under /en/).
import { pathIn, type Lang } from './language';

export type PageId = 'privacy' | 'legal' | 'mobile' | 'mobile-privacy';
export const PAGE_IDS: readonly PageId[] = ['privacy', 'legal', 'mobile', 'mobile-privacy'];

/** The route of each page, in French (the English one under /en/). */
const ROUTES: Record<PageId, string> = {
  privacy: '/privacy/',
  legal: '/legal/',
  mobile: '/mobile/',
  'mobile-privacy': '/mobile/privacy/',
};

/** Where a page lives in `lang` (a route, without the base URL). */
export const pagePath = (id: PageId, lang: Lang) => pathIn(lang, ROUTES[id]);

/** The Markdown file of a page, from the website’s folder: what the “edit this page” link opens on GitHub. */
export const pageSource = (id: PageId, lang: Lang) => `content/${id}/${lang}.md`;

export const privacyPath = (lang: Lang) => pagePath('privacy', lang);
export const privacySource = (lang: Lang) => pageSource('privacy', lang);
export const legalPath = (lang: Lang) => pagePath('legal', lang);
export const legalSource = (lang: Lang) => pageSource('legal', lang);
export const mobilePath = (lang: Lang) => pagePath('mobile', lang);
export const mobilePrivacyPath = (lang: Lang) => pagePath('mobile-privacy', lang);
