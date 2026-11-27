import { fileURLToPath } from 'node:url';
import { configDefaults, defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';

export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  // The phone's app (apps/mobile, MO-1) imports the app's interface where it is, as `$shared`.
  // The phone's native side (apps/mobile-plugin, MO-4) from its sources, its Capacitor the app's.
  resolve: {
    alias: {
      $shared: fileURLToPath(new URL('./src', import.meta.url)),
      '@escouade/capacitor-mobile': fileURLToPath(new URL('./apps/mobile-plugin/src/index.ts', import.meta.url)),
      '@capacitor/core': fileURLToPath(new URL('./apps/mobile/node_modules/@capacitor/core', import.meta.url)),
    },
  },
  test: {
    environment: 'jsdom',
    include: [
      'src/**/*.test.ts',
      'schemas/**/*.test.ts',
      'scripts/**/*.test.ts',
      'sdk/**/*.test.ts',
      'tests/e2e/**/*.test.ts',
      'apps/mobile-plugin/test/**/*.test.ts',
      'apps/mobile/src/**/*.test.ts',
    ],
    // A plugin's template runs its own tests, with node --test, once written (sdk/scripts/template-ci.mjs).
    exclude: [...configDefaults.exclude, 'sdk/create-escouade-plugin/templates/**'],
    setupFiles: ['./src/test/setup.ts'],
    // A temporary folder of its own, removed at the end (scripts/test-tmp.ts).
    globalSetup: ['./scripts/vitest-global-tmp.ts'],
    restoreMocks: true,
    // A loaded machine outlasts the default 5 s on a test that is right: a CI runner, as well as a workstation that
    // compiles beside it (ESC-522: tests of 2 to 3 s alone fell there one or two by full run, never the same). The same
    // 15 s on both, so that the workstation raises no false alarm the CI would not; a test that hangs still falls. It waits
    // for nothing: a test waits for the state it wants (`findBy`, `waitFor`), never for a time.
    testTimeout: 15_000,
    // The guard of `expect.poll`, the same as Testing Library's (`asyncUtilTimeout`, src/test/setup.ts): 1 s by default, which
    // a module loaded at its first use outlasts in a full run on a loaded machine (Ctrl+S imports the editor's actions,
    // ESC-522). It waits for nothing: the poll gives back as soon as the state is there.
    expect: { poll: { timeout: 5_000 } },
  },
});
