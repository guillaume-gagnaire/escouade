// How the workbench of VS Code is loaded (ESC-472): in Escouade's language. VS Code reads its texts when its modules
// load (the titles of its commands, its menus, its settings), so its translations must be there before any of them: the
// French ones (monaco-vscode-api's language pack, `@codingame/monaco-vscode-language-pack-fr`) are put in first, then
// the pack (the translations of the default extensions), then the workbench. English is VS Code's own. Each in chunks
// of its own, loaded on the editor's first opening: nothing of it at the app's start.
//
// A page loads the workbench once, in one language: a change of Escouade's language reaches the view when the window
// is reloaded (`reloadForLanguage`), the unsaved files saved or given up first and the view's tabs opened again.

import type { Lang } from '../i18n';

type Workbench = typeof import('./workbench');

let loading: Promise<Workbench> | null = null;
let lang: Lang | null = null;

/** The workbench's module, loaded the first time in `ui`, the language of the interface then. */
export function loadWorkbench(ui: Lang): Promise<Workbench> {
  loading ??= (async () => {
    if (ui === 'fr') {
      const { default: messages } = await import('@codingame/monaco-vscode-language-pack-fr/translations/main.i18n.json.js');
      const g = globalThis as { _VSCODE_NLS_MESSAGES?: unknown; _VSCODE_NLS_LANGUAGE?: string };
      g._VSCODE_NLS_MESSAGES = messages;
      g._VSCODE_NLS_LANGUAGE = 'fr';
      await import('@codingame/monaco-vscode-language-pack-fr');
    }
    const mod = await import('./workbench');
    lang = ui;
    return mod;
  })().catch((e) => {
    loading = null;
    throw e;
  });
  return loading;
}

/** The language the workbench of this page speaks, once loaded (null before). */
export const workbenchLang = (): Lang | null => lang;

/** Where the view's tabs wait for the page reloaded in another language. */
export const REOPEN_KEY = 'escouade-vscode-reopen';

/** A tab of the view: its project, its source (`project` or an agent's id), its file. */
export interface Reopen {
  projectId: string;
  source: string;
  path: string;
}

/** The tabs the view opens again once its workbench is started (the one shown last), after a reload for the language. */
export const reopening: { tabs: Reopen[] } = { tabs: [] };

/** The tabs to open again (the one shown last), kept by `keepTabs`, taken once. */
export function takeTabs(store: Pick<Storage, 'getItem' | 'removeItem'> = sessionStorage): Reopen[] {
  try {
    const kept = JSON.parse(store.getItem(REOPEN_KEY) ?? '[]');
    store.removeItem(REOPEN_KEY);
    return Array.isArray(kept)
      ? kept.filter(
          (t): t is Reopen => !!t && typeof t.projectId === 'string' && typeof t.source === 'string' && typeof t.path === 'string',
        )
      : [];
  } catch {
    return [];
  }
}

/** Keeps `tabs` for the page about to load again. */
export function keepTabs(tabs: Reopen[], store: Pick<Storage, 'setItem'> = sessionStorage) {
  try {
    store.setItem(REOPEN_KEY, JSON.stringify(tabs));
  } catch {
    // No storage: the editor opens again without its tabs.
  }
}

/** Reloads the window (the page, and its workbench with it); a test replaces it. */
export const page = { reload: () => location.reload() };
