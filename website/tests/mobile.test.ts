import { existsSync, readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { CATALOGS } from '../app/data/catalogs';
import type { Lang } from '../app/data/language';
import { mobilePath, mobilePrivacyPath, pagePath, pageSource, type PageId } from '../app/data/privacy';
import { renderDoc } from '../app/data/render';
import { SITE } from '../app/data/site';
import { escapeText, headOf, tags } from './helpers';

// The page of the mobile app and its privacy policy (MO-8, ESC-576): the stores link to the second
// (apps/mobile/store/listing/ios/<locale>/privacy_url.txt), the Play Console asks for it too.
const OUT = new URL('../.output/public/', import.meta.url);
const BASE = '/';
const LANGS: Lang[] = ['fr', 'en'];
const PAGES: PageId[] = ['mobile', 'mobile-privacy'];

const source = (id: PageId, lang: Lang) => readFileSync(new URL(`../${pageSource(id, lang)}`, import.meta.url), 'utf8');
const render = (id: PageId, lang: Lang) => renderDoc(source(id, lang), { lang, link: (p) => p, callouts: CATALOGS[lang].docs.callouts });

describe('the addresses of the mobile app’s pages', () => {
  it('are the same pages in both languages, and the ones the stores’ listings give', () => {
    expect(mobilePath('fr')).toBe('/mobile/');
    expect(mobilePath('en')).toBe('/en/mobile/');
    expect(mobilePrivacyPath('fr')).toBe('/mobile/privacy/');
    expect(mobilePrivacyPath('en')).toBe('/en/mobile/privacy/');
    for (const [locale, lang] of [
      ['fr-FR', 'fr'],
      ['en-US', 'en'],
    ] as const) {
      const listing = (f: string) =>
        readFileSync(new URL(`../../apps/mobile/store/listing/ios/${locale}/${f}`, import.meta.url), 'utf8').trim();
      expect(listing('privacy_url.txt')).toBe(`${SITE}${mobilePrivacyPath(lang).slice(1)}`);
      expect(listing('marketing_url.txt')).toBe(`${SITE}${mobilePath(lang).slice(1)}`);
    }
  });
});

describe.each(PAGES)('the Markdown of %s', (id) => {
  it('has a title, a lede and the same sections in both languages', () => {
    const [fr, en] = LANGS.map((l) => render(id, l));
    expect(en.headings.map((h) => h.depth)).toEqual(fr.headings.map((h) => h.depth));
    for (const lang of LANGS) expect(render(id, lang).lede.length, lang).toBeGreaterThan(60);
  });

  it('writes the typography the way the documentation does', () => {
    const prose = (s: string) => s.replace(/```[\s\S]*?```/g, '').replace(/`[^`\n]*`/g, '');
    for (const lang of LANGS) {
      const text = prose(source(id, lang));
      expect(text, lang).not.toMatch(/['"]/);
      expect(text, lang).not.toContain('...');
      if (lang === 'fr') expect(text).not.toMatch(/«(?! )|(?<! )»/);
      else expect(text).not.toMatch(/[«»]/);
    }
  });
});

describe('the mobile app’s privacy policy', () => {
  it('says what goes where: nothing but the sealed push, Firebase reduced to FCM, nothing on iOS', () => {
    for (const lang of LANGS) {
      const text = source('mobile-privacy', lang);
      for (const fact of ['WireGuard', 'push.escouade.dev', 'FCM', 'APNs', 'Analytics', 'Crashlytics', 'CNIL', 'Keystore'])
        expect(text, `${lang}: ${fact}`).toContain(fact);
      expect(text, lang).toMatch(lang === 'fr' ? /aucun SDK de Firebase/ : /no Firebase SDK/);
      expect(text, lang).toContain(`(${pagePath('legal', lang)})`);
      expect(text, lang).toContain(`(${pagePath('privacy', lang)})`);
      expect(text, lang).not.toMatch(/anonym/i);
    }
  });
});

describe.each(LANGS.flatMap((lang) => PAGES.map((id) => [id, lang] as const)))('the page %s (%s)', (id, lang) => {
  const path = pagePath(id, lang).slice(1);
  let html = '';
  beforeAll(() => {
    const file = new URL(`${path}index.html`, OUT);
    if (!existsSync(file)) throw new Error(`${path}index.html : génère d’abord le site (npm run generate)`);
    html = readFileSync(file, 'utf8');
  });

  it('is titled, described and pointed to its version in the other language', () => {
    const doc = render(id, lang);
    expect(html).toContain(`<title>${escapeText(`${doc.title} · Escouade`)}</title>`);
    expect(html).toMatch(new RegExp(`<html[^>]*lang="${lang}"`));
    const links = tags(headOf(html), 'link').filter((l) => l.rel === 'alternate');
    expect(Object.fromEntries(links.map((l) => [l.hreflang, l.href]))).toEqual({
      fr: `${SITE}${pagePath(id, 'fr').slice(1)}`,
      en: `${SITE}${pagePath(id, 'en').slice(1)}`,
      'x-default': `${SITE}${pagePath(id, 'fr').slice(1)}`,
    });
  });

  it('serves every local file and links to pages that exist', () => {
    const local = [...html.matchAll(/(?:src|href)="(\/[^"]*)"/g)].map((m) => m[1]);
    for (const ref of local) {
      const file = decodeURI(ref.slice(BASE.length).split(/[?#]/)[0]);
      expect(existsSync(new URL(file.endsWith('/') || !file ? `${file}index.html` : file, OUT)), ref).toBe(true);
    }
  });

  it('is linked from the footer of the home page (the app’s page)', () => {
    if (id !== 'mobile') return;
    const home = readFileSync(new URL(lang === 'en' ? 'en/index.html' : 'index.html', OUT), 'utf8');
    const footer = home.match(/<footer class="foot[\s\S]*?<\/footer>/)?.[0] ?? '';
    expect(tags(footer, 'a').find((a) => a.href === `${BASE}${path}`)).toBeDefined();
  });
});
