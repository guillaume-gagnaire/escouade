import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

// The atelier's own server (`npx vite --config .whatsup/atelier/vite.config.ts --port <port>`): the repo as root, so the
// interface and the tests' fake backend import as they do in the tests; its own cache, apart from the app's.
export default defineConfig({
  root: fileURLToPath(new URL('../..', import.meta.url)),
  plugins: [svelte()],
  cacheDir: 'node_modules/.vite-atelier',
  clearScreen: false,
  server: { strictPort: true, host: '127.0.0.1', watch: { ignored: ['**/src-tauri/**', '**/target/**'] } },
});
