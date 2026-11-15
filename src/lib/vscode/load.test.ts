// The workbench of VS Code in Escouade's language (ESC-472): the French texts put in before any module of VS Code
// loads, English left as VS Code's own; the tabs kept for a reload in another language.
import { beforeEach, describe, expect, it, vi } from 'vitest';

const g = globalThis as { _VSCODE_NLS_MESSAGES?: unknown; _VSCODE_NLS_LANGUAGE?: string };
/** What VS Code's modules found when they loaded: the texts there already, or not. */
const seen = vi.hoisted(() => ({ language: undefined as string | undefined, messages: undefined as unknown }));

vi.mock('@codingame/monaco-vscode-language-pack-fr/translations/main.i18n.json.js', () => ({ default: [undefined, 'Fermer'] }));
vi.mock('@codingame/monaco-vscode-language-pack-fr', () => ({}));
vi.mock('./workbench', () => {
  const at = globalThis as typeof g;
  seen.language = at._VSCODE_NLS_LANGUAGE;
  seen.messages = at._VSCODE_NLS_MESSAGES;
  return { workbench: () => {} };
});

describe('the language of the VS Code view', () => {
  beforeEach(() => {
    vi.resetModules();
    delete g._VSCODE_NLS_LANGUAGE;
    delete g._VSCODE_NLS_MESSAGES;
    seen.language = seen.messages = undefined;
  });

  it('puts the French texts in before VS Code loads, when Escouade is in French', async () => {
    const { loadWorkbench, workbenchLang } = await import('./load');
    expect(workbenchLang()).toBeNull();
    const mod = await loadWorkbench('fr');
    expect(mod.workbench).toBeTypeOf('function');
    expect(seen.language).toBe('fr');
    expect(seen.messages).toEqual([undefined, 'Fermer']);
    expect(workbenchLang()).toBe('fr');
  });

  it('leaves VS Code in its own English, when Escouade is in English', async () => {
    const { loadWorkbench, workbenchLang } = await import('./load');
    await loadWorkbench('en');
    expect(seen.language).toBeUndefined();
    expect(seen.messages).toBeUndefined();
    expect(workbenchLang()).toBe('en');
  });

  it('loads it once per page, in the language of its first opening', async () => {
    const { loadWorkbench, workbenchLang } = await import('./load');
    const first = await loadWorkbench('en');
    expect(await loadWorkbench('fr')).toBe(first);
    expect(workbenchLang()).toBe('en');
    expect(seen.language).toBeUndefined();
  });
});

describe('the tabs kept for a reload', () => {
  const store = () => {
    const m = new Map<string, string>();
    return {
      getItem: (k: string) => m.get(k) ?? null,
      setItem: (k: string, v: string) => void m.set(k, v),
      removeItem: (k: string) => void m.delete(k),
    };
  };

  it('gives back the tabs kept, once, the one shown last', async () => {
    const { keepTabs, takeTabs } = await import('./load');
    const s = store();
    const tabs = [
      { projectId: 'p1', source: 'project', path: 'src/a.ts' },
      { projectId: 'p1', source: 'a1', path: 'src/b.ts' },
    ];
    keepTabs(tabs, s);
    expect(takeTabs(s)).toEqual(tabs);
    expect(takeTabs(s)).toEqual([]);
  });

  it('keeps nothing it cannot read', async () => {
    const { REOPEN_KEY, takeTabs } = await import('./load');
    const s = store();
    s.setItem(REOPEN_KEY, '{nope');
    expect(takeTabs(s)).toEqual([]);
    s.setItem(REOPEN_KEY, JSON.stringify([{ projectId: 'p1', path: 1 }, null, { projectId: 'p1', source: 'project', path: 'x' }]));
    expect(takeTabs(s)).toEqual([{ projectId: 'p1', source: 'project', path: 'x' }]);
  });
});
