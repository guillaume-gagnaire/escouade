// Le Svelte, vite et marked viennent du node_modules du dépôt (une seule copie de Svelte pour l'interface commune et
// le code repris d'Escouade) : `npm ci` à la racine du dépôt d'abord.
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: Number(process.env.PORT ?? 0) || undefined, strictPort: true, host: process.env.TAURI_DEV_HOST || false },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: { target: 'es2022', outDir: 'dist', emptyOutDir: true },
});
