import { fileURLToPath } from 'node:url';
import { STATIC_HEADERS } from './server/utils/headers';

// The store, as Coolify serves it on store.escouade.dev: rendered by the server (its pages change, and an author
// signs in), with its API in server/api/ (Nitro). It shares the site's tokens (website/app/assets/main.css) by a
// relative path, and adds its light theme on top (app/assets/store.css).
const here = (path: string) => fileURLToPath(new URL(path, import.meta.url));

export default defineNuxtConfig({
  compatibilityDate: '2026-09-01',
  devtools: { enabled: false },
  css: [here('../website/app/assets/main.css'), '~/assets/store.css', '~/assets/catalog.css'],
  app: {
    head: {
      htmlAttrs: { lang: 'fr' },
      meta: [{ name: 'color-scheme', content: 'dark light' }],
      link: [{ rel: 'icon', type: 'image/svg+xml', href: '/logo.svg' }],
    },
  },
  // The site's stylesheet imports its fonts by package name: they resolve to the store's own copies, so that the
  // build needs nothing installed in website/.
  // No asset inlined as a data: address (Vite does it for the small font subsets): the pages' policy loads fonts
  // from the store only (font-src 'self').
  vite: {
    build: { assetsInlineLimit: 0 },
    resolve: {
      alias: {
        '@fontsource-variable/inter': here('node_modules/@fontsource-variable/inter'),
        '@fontsource/jetbrains-mono': here('node_modules/@fontsource/jetbrains-mono'),
      },
    },
  },
  runtimeConfig: {
    public: {
      // Where the store is reached from outside (NUXT_PUBLIC_SITE_URL), for its links, GitHub's callback and the Origin of its forms.
      siteUrl: 'https://store.escouade.dev',
    },
  },
  // The security headers on every answer; the pages' Content-Security-Policy is set as they render (server/plugins/csp.ts).
  routeRules: { '/**': { headers: STATIC_HEADERS } },
  typescript: { strict: true },
});
