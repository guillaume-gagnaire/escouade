import { configDefaults, defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';

export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts', 'schemas/**/*.test.ts', 'scripts/**/*.test.ts', 'sdk/**/*.test.ts', 'tests/e2e/**/*.test.ts'],
    // A plugin's template runs its own tests, with node --test, once written (sdk/scripts/template-ci.mjs).
    exclude: [...configDefaults.exclude, 'sdk/create-escouade-plugin/templates/**'],
    setupFiles: ['./src/test/setup.ts'],
    // A temporary folder of its own, removed at the end (scripts/test-tmp.ts).
    globalSetup: ['./scripts/vitest-global-tmp.ts'],
    restoreMocks: true,
    // A loaded CI runner can outlast the default 5 s on a test that is right.
    testTimeout: process.env.CI ? 15_000 : 5_000,
  },
});
