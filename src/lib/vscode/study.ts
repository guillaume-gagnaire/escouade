// ESC-444 (study): the bench of the editors, run by the page itself when the app is started with ESCOUADE_VSCODE_STUDY
// (the file of its log) and ESCOUADE_VSCODE_STUDY_OPTS (what to measure), so that it runs the same under WebView2,
// WKWebView and WebKitGTK, where no test drives the page. Its phases go to the log (`study_log`), each line with its time:
// the bench (`scripts/study/editor-bench.mjs`) samples the memory of the app's processes beside it and reads the log.
//
//   mark idle-app     the app at rest, no project
//   open <ms>         the editor of a project opened: its tree shown
//   mark rest-editor  at rest, the editor open, no file
//   ten <ms>          10 files opened one after the other, each in its tab
//   mark rest-ten     at rest, 10 files open
//   lsp <n>           (VS Code) the diagnostics of the core's language server on src/app.ts
//   ext <id> <state>  (VS Code) an extension of Open VSX installed: activated or not, its errors
//   shot              the bench takes its screenshot now
//   done              the app quits

import { invoke } from '@tauri-apps/api/core';
import { api } from '../ipc';
import { projectPalette } from '../plugin-theme.svelte';
import { app } from '../state.svelte';
import { vscodeFlags } from './flag.svelte';

interface Opts {
  /** The folder of the project to open. */
  project: string;
  mode: 'codemirror' | 'vscode' | 'vscode-node';
  /** How long each rest lasts (ms). */
  rest?: number;
}

const log = (line: string) => invoke('study_log', { line }).catch(() => {});
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function until(what: string, f: () => boolean, ms = 120_000) {
  const end = Date.now() + ms;
  while (!f()) {
    if (Date.now() > end) throw new Error(`timeout: ${what}`);
    await sleep(50);
  }
}

const count = (sel: string) => document.querySelectorAll(sel).length;

export async function maybeRun() {
  const raw = await invoke<string | null>('study_env').catch(() => null);
  if (raw === null) return;
  const opts = JSON.parse(raw || '{}') as Opts;
  const rest = opts.rest ?? 8000;
  let errors = 0;
  const onError = (what: string) => {
    errors++;
    if (errors <= 20) void log(`error ${what.slice(0, 300)}`);
  };
  addEventListener('error', (e) => onError(String(e.message ?? e)));
  addEventListener('unhandledrejection', (e) => onError(String(e.reason?.message ?? e.reason)));
  const vscode = opts.mode !== 'codemirror';
  try {
    await until('app ready', () => app.ready);
    await log(`env ${opts.mode} ${navigator.userAgent}`);
    await sleep(rest);
    await log('mark idle-app');

    vscodeFlags.editor = vscode;
    vscodeFlags.nodeHost = opts.mode === 'vscode-node';
    const p = await api.createProject({
      path: opts.project,
      name: 'demo-api',
      color: projectPalette()[0],
      worktreePerAgent: false,
      firstAgent: null,
      restore: false,
    });
    app.projects.push(p);
    app.selectProject(p.id);
    await sleep(3000);

    const tree = vscode ? '#vscode-layer .explorer-folders-view .monaco-list-row' : 'main.editor .tree [role="treeitem"]';
    const tabs = vscode ? '#vscode-layer .tabs-container .tab' : 'main.editor .tabs [role="tab"]';
    const t0 = performance.now();
    await app.openEditor({ projectId: p.id, source: 'project' });
    await until('tree', () => count(tree) > 0);
    await log(`open ${Math.round(performance.now() - t0)}`);
    if (vscode) {
      const s = (globalThis as { __escouadeVscode?: { marks: Record<string, number>; nodeHost: unknown } }).__escouadeVscode;
      await log(`marks ${JSON.stringify(s?.marks ?? {})} host ${JSON.stringify(s?.nodeHost ?? null)}`);
      const { tokenSources } = await import('./theme');
      const bg = document.querySelector(
        '#vscode-layer .monaco-workbench .part.editor .monaco-editor-background, #vscode-layer .monaco-workbench .part.sidebar',
      );
      const label = document.querySelector('#vscode-layer .explorer-folders-view .label-name');
      await log(
        `theme panel=${tokenSources.panel} text=${tokenSources.text} soft=${tokenSources.soft} p=${tokenSources.p} bg=${bg ? getComputedStyle(bg).backgroundColor : '-'} label=${label ? getComputedStyle(label).color : '-'}`,
      );
    }
    await sleep(rest);
    await log('mark rest-editor');

    const t1 = performance.now();
    for (let i = 1; i <= 10; i++) {
      await app.openEditor({ projectId: p.id, source: 'project', path: `src/f${i}.ts` });
      await until(`tab ${i}`, () => count(tabs) >= i, 30_000);
    }
    await log(`ten ${Math.round(performance.now() - t1)}`);
    await sleep(rest);
    await log('mark rest-ten');
    if (vscode) await extensions(p.id, opts.mode === 'vscode-node');
    await log('shot');
    await sleep(4000);
  } catch (e) {
    await log(`failed ${String(e)}`);
  }
  await log(`errors ${errors}`);
  await log('done');
  await sleep(300);
  await invoke('study_quit').catch(() => {});
}

type Vs = {
  vscode: typeof import('vscode');
  extensions(): Promise<{ id: string; activated: boolean; errors: string[] }[]>;
};

/** Installs `id` from Open VSX, saying yes to VS Code's questions (the publisher trusted). */
async function install(api: Vs, id: string): Promise<string> {
  let state = 'pending';
  api.vscode.commands.executeCommand('workbench.extensions.installExtension', id).then(
    () => (state = 'ok'),
    (e) => (state = `error: ${String(e).slice(0, 200)}`),
  );
  const end = Date.now() + 120_000;
  while (state === 'pending' && Date.now() < end) {
    // By its words: the order of the buttons is each system's (macOS puts « Cancel » first, Linux « Learn More »).
    const buttons = [...document.querySelectorAll<HTMLElement>('#vscode-layer .monaco-dialog-box .monaco-button')];
    buttons.find((b) => /trust|install/i.test(b.textContent ?? ''))?.click();
    await sleep(250);
  }
  return state === 'pending' ? 'timeout' : state;
}

/** What works of the VS Code view on this system: the core's language server, extensions of Open VSX. */
async function extensions(projectId: string, node: boolean) {
  const api = (globalThis as { __escouadeVscode?: Vs }).__escouadeVscode;
  if (!api) return;
  const diags = (file: string, source?: string) =>
    api.vscode.languages
      .getDiagnostics()
      .filter(([u]) => u.path.endsWith(file))
      .flatMap(([, d]) => d)
      .filter((d) => !source || d.source === source);
  if (!node) {
    await app.openEditor({ projectId, source: 'project', path: 'src/app.ts' });
    const end = Date.now() + 60_000;
    while (!diags('app.ts').length && Date.now() < end) await sleep(250);
    await log(`lsp ${diags('app.ts').length} ${diags('app.ts')[0]?.message ?? ''}`);
  }
  const ids = node
    ? ['streetsidesoftware.code-spell-checker']
    : ['dracula-theme.theme-dracula', 'vscodevim.vim', 'tamasfe.even-better-toml'];
  for (const id of ids) {
    const state = await install(api, id);
    await sleep(3000);
    const e = (await api.extensions()).find((x) => x.id === id);
    await log(`ext ${id} ${state} activated=${!!e?.activated} errors=${e?.errors.length ?? '-'}`);
  }
  if (node) {
    await app.openEditor({ projectId, source: 'project', path: 'notes.md' });
    const end = Date.now() + 60_000;
    while (!diags('notes.md', 'cSpell').length && Date.now() < end) await sleep(250);
    await log(`cspell ${diags('notes.md', 'cSpell').length}`);
  }
}
