// The phone's app (MO-1, docs/MOBILE.md § 8): its page is built from apps/mobile, on the app's Svelte interface
// (src/, shared, never copied: its tokens, its icons, its texts, its transport to the core). Svelte, Vite and the fonts
// come from the repository's node_modules (`npm ci` at its root first): one copy of Svelte for both. Capacitor's own
// packages are this folder's (`npm ci` here).
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

const root = fileURLToPath(new URL('../..', import.meta.url));
// The app's version, said in the handshake: the core takes a client of its own version alone.
const version = JSON.parse(readFileSync(new URL('../../package.json', import.meta.url), 'utf8')).version as string;

export default defineConfig({
  root: fileURLToPath(new URL('.', import.meta.url)),
  plugins: [svelte({ configFile: false, preprocess: vitePreprocess() })],
  clearScreen: false,
  define: { __ESCOUADE_VERSION__: JSON.stringify(version) },
  // The app's interface, shared: imported where it is (`$shared/lib/transport`), never copied.
  resolve: { alias: { $shared: fileURLToPath(new URL('../../src', import.meta.url)) } },
  server: {
    port: Number(process.env.PORT ?? 0) || 5174,
    strictPort: true,
    host: '127.0.0.1',
    fs: { allow: [root] },
  },
  preview: { port: Number(process.env.PORT ?? 0) || 4174, strictPort: true, host: '127.0.0.1' },
  build: { target: 'es2022', outDir: 'dist', emptyOutDir: true, chunkSizeWarningLimit: 2000 },
});
