import { readFileSync } from 'node:fs';
import { DOC_SLUGS, docPath } from './app/data/docs';
import { PAGE_IDS, pagePath } from './app/data/privacy';
import { withPolicy } from './csp';

// The app's version, read from the repo: the one its next release carries.
const tauri = JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8')) as { version: string };
// The site is served at the root of its own domain, escouade.dev (website/Dockerfile, on Coolify).
const base = process.env.NUXT_APP_BASE_URL ?? '/';

export default defineNuxtConfig({
  compatibilityDate: '2026-09-01',
  devtools: { enabled: false },
  css: ['~/assets/main.css', '~/assets/prose.css'],
  app: {
    baseURL: base,
    head: {
      htmlAttrs: { lang: 'fr' },
      link: [{ rel: 'icon', type: 'image/svg+xml', href: `${base}logo.svg` }],
    },
  },
  runtimeConfig: { public: { version: tauri.version } },
  // One page per language: the French one at the root, the English one under /en/; and so for each page of the
  // documentation, under /docs/ and /en/docs/.
  nitro: {
    preset: 'static',
    prerender: {
      routes: [
        '/',
        '/en',
        ...(['fr', 'en'] as const).flatMap((lang) => [docPath(lang), ...DOC_SLUGS.map((slug) => docPath(lang, slug))]),
        ...PAGE_IDS.flatMap((id) => [pagePath(id, 'fr'), pagePath(id, 'en')]),
      ],
      // The language switch's French links (`?lang=fr`) are the French pages, already generated.
      ignore: [(route: string) => route.includes('?lang=')],
    },
    hooks: {
      // Each page carries its content security policy, its own inline scripts named by their hashes (csp.ts).
      'prerender:generate'(route) {
        if (route.fileName?.endsWith('.html') && typeof route.contents === 'string') route.contents = withPolicy(route.contents);
      },
    },
  },
});
