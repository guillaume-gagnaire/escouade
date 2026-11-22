// No module of the window calls the core but through `transport.ts` (ESC-495): Tauri's `invoke` and `Channel` are
// imported there alone. What stays the window's own (its menus, its dialogs, its window, the opener of a link) is the
// list below, as `docs/REMOTE.md` says it (« La fondation 2b »).

import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const SRC = join(dirname(fileURLToPath(import.meta.url)), '..');

function sources(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      if (name !== 'test') out.push(...sources(path));
    } else if (/\.(ts|svelte)$/.test(name) && !/\.test\.ts$|\.test\.svelte$/.test(name)) {
      out.push(path);
    }
  }
  return out;
}

/** The imports of `@tauri-apps/…` of each module, by module (its path from `src`). */
function tauriImports(): Record<string, string[]> {
  const found: Record<string, string[]> = {};
  for (const file of sources(SRC)) {
    const text = readFileSync(file, 'utf8');
    const modules = [...text.matchAll(/from '(@tauri-apps\/[^']+)'/g)].map((m) => m[1]);
    if (modules.length) found[relative(SRC, file).replaceAll('\\', '/')] = [...new Set(modules)].sort();
  }
  return found;
}

describe('the window and the core', () => {
  it('speaks to the core through transport.ts alone, and keeps to itself what is the window’s', () => {
    expect(tauriImports()).toEqual({
      // The calls of the core, on Tauri's IPC or the remote API.
      'lib/transport.ts': ['@tauri-apps/api/core'],
      // The window's own: its dialogs (a folder to pick), the system's browser for a link, its window and webview
      // (title bar, zoom, theme), the events of the pages of the launches' windows.
      'components/modals/NewProjectModal.svelte': ['@tauri-apps/plugin-dialog'],
      'components/onboarding/ProjectStation.svelte': ['@tauri-apps/plugin-dialog'],
      'components/PagesWindow.svelte': ['@tauri-apps/api/event'],
      'components/plugins/PluginPage.svelte': ['@tauri-apps/plugin-opener'],
      'components/plugins/PluginsView.svelte': ['@tauri-apps/plugin-opener'],
      'components/plugins/StorePluginPage.svelte': ['@tauri-apps/plugin-opener'],
      'components/settings/AdvancedPage.svelte': ['@tauri-apps/plugin-dialog'],
      'components/team/TeamClone.svelte': ['@tauri-apps/plugin-dialog'],
      'components/team/TeamPlugins.svelte': ['@tauri-apps/plugin-dialog'],
      'components/TitleBar.svelte': ['@tauri-apps/api/window'],
      'lib/appearance.ts': ['@tauri-apps/api/window'],
      'lib/display.svelte.ts': ['@tauri-apps/api/webview', '@tauri-apps/api/window'],
      'lib/plugins-view.svelte.ts': ['@tauri-apps/plugin-dialog'],
      'lib/project-logo.svelte.ts': ['@tauri-apps/plugin-dialog'],
    });
  });
});
