import { readFileSync } from 'node:fs';

// The app's version, read from the repo: the one its next release carries.
const tauri = JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8')) as { version: string };
// GitHub Pages serves the site under the repository's name.
const base = process.env.NUXT_APP_BASE_URL ?? '/escouade/';

export default defineNuxtConfig({
  compatibilityDate: '2026-09-01',
  devtools: { enabled: false },
  css: ['~/assets/main.css'],
  app: {
    baseURL: base,
    head: {
      htmlAttrs: { lang: 'fr' },
      link: [{ rel: 'icon', type: 'image/svg+xml', href: `${base}logo.svg` }],
    },
  },
  runtimeConfig: { public: { version: tauri.version } },
  // One page per language: the French one at the root, the English one under /en/.
  nitro: { preset: 'github_pages', prerender: { routes: ['/', '/en'] } },
});
