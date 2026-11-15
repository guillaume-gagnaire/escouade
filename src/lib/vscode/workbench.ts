// The workbench of VS Code (`@codingame/monaco-vscode-api`, pinned), the editor of a project in Escouade (ESC-444's
// study, taken in ESC-447 in place of the CodeMirror editor). Its services start once per page, in an element
// of their own (`host`) that is never destroyed nor moved in the page: it is laid over the place of the view that shows
// it (`VscodeView.svelte`, alone or in a pane) and hidden when that view goes, so that moving, closing and opening the
// pane again keep its tabs (an iframe or a webview of VS Code moved in the DOM loads again). One workbench, so one
// column of it at a time.
//
// - The files are the core's (`fs-provider.ts`): each source shown (the project's checkout, an agent's worktree) is a
//   folder of its workspace, and the changes the core's watcher tells are VS Code's file changes.
// - The theme is Escouade's (`theme.ts`), read when it starts.
// - No extension host of its own: the default extensions it carries are declarative (languages, grammars, themes), so
//   that the page needs neither `unsafe-eval` nor an iframe. The extensions of Open VSX (A5), the language servers
//   (A1, ESC-446: `onFolder` tells it each folder) and the Node extension host (N2) come later.

import { getService, initialize, IWorkbenchLayoutService, LogLevel } from '@codingame/monaco-vscode-api';
import { ExtensionHostKind, registerExtension } from '@codingame/monaco-vscode-api/extensions';
import getConfigurationServiceOverride, {
  initUserConfiguration,
  type IStoredWorkspace,
} from '@codingame/monaco-vscode-configuration-service-override';
import getDialogsServiceOverride from '@codingame/monaco-vscode-dialogs-service-override';
import getEnvironmentServiceOverride from '@codingame/monaco-vscode-environment-service-override';
import getExplorerServiceOverride from '@codingame/monaco-vscode-explorer-service-override';
import getExtensionServiceOverride from '@codingame/monaco-vscode-extensions-service-override';
import getFilesServiceOverride, {
  FileSystemProviderError,
  type FileSystemProviderErrorCode,
  type IFileSystemProviderWithFileReadWriteCapability,
  initFile,
  registerFileSystemOverlay,
} from '@codingame/monaco-vscode-files-service-override';
import getKeybindingsServiceOverride from '@codingame/monaco-vscode-keybindings-service-override';
import getLanguagesServiceOverride from '@codingame/monaco-vscode-languages-service-override';
import getLifecycleServiceOverride from '@codingame/monaco-vscode-lifecycle-service-override';
import getLogServiceOverride from '@codingame/monaco-vscode-log-service-override';
import getMarkersServiceOverride from '@codingame/monaco-vscode-markers-service-override';
import getModelServiceOverride from '@codingame/monaco-vscode-model-service-override';
import getNotificationsServiceOverride from '@codingame/monaco-vscode-notifications-service-override';
import getOutlineServiceOverride from '@codingame/monaco-vscode-outline-service-override';
import getOutputServiceOverride from '@codingame/monaco-vscode-output-service-override';
import getPreferencesServiceOverride from '@codingame/monaco-vscode-preferences-service-override';
import getQuickAccessServiceOverride from '@codingame/monaco-vscode-quickaccess-service-override';
import getSearchServiceOverride from '@codingame/monaco-vscode-search-service-override';
import getStorageServiceOverride from '@codingame/monaco-vscode-storage-service-override';
import getTextmateServiceOverride from '@codingame/monaco-vscode-textmate-service-override';
import getThemeServiceOverride from '@codingame/monaco-vscode-theme-service-override';
import getWorkbenchServiceOverride from '@codingame/monaco-vscode-workbench-service-override';
import getWorkingCopyServiceOverride from '@codingame/monaco-vscode-working-copy-service-override';
import * as vscode from 'vscode';
import 'vscode/localExtensionHost';

import '@codingame/monaco-vscode-theme-defaults-default-extension';
import '@codingame/monaco-vscode-theme-seti-default-extension';
import '@codingame/monaco-vscode-rust-default-extension';
import '@codingame/monaco-vscode-typescript-basics-default-extension';
import '@codingame/monaco-vscode-javascript-default-extension';
import '@codingame/monaco-vscode-json-default-extension';
import '@codingame/monaco-vscode-markdown-basics-default-extension';
import '@codingame/monaco-vscode-css-default-extension';
import '@codingame/monaco-vscode-html-default-extension';
import '@codingame/monaco-vscode-yaml-default-extension';
import '@codingame/monaco-vscode-xml-default-extension';
import '@codingame/monaco-vscode-shellscript-default-extension';

import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker.js?worker&url';
import textMateWorker from '@codingame/monaco-vscode-textmate-service-override/worker?worker&url';
import searchWorker from '@codingame/monaco-vscode-search-service-override/worker?worker&url';
import outputWorker from '@codingame/monaco-vscode-output-service-override/worker?worker&url';

import { onDiskChange } from '../disk-changes';
import { api } from '../ipc';
import { EscouadeFileSystem, type Source } from './fs-provider';
import { isDark, readTokens, vscodeTheme } from './theme';

export type { Source } from './fs-provider';

/** The element the workbench lives in, lent to the view that shows it. */
let host: HTMLElement | null = null;
let ready: Promise<void> | null = null;
let files: EscouadeFileSystem<vscode.Uri> | null = null;

const WORKSPACE = vscode.Uri.file('/escouade.code-workspace');

/** A folder of the workspace: a source shown, its URI, its name in the explorer. */
export interface Folder {
  source: Source;
  uri: vscode.Uri;
  name: string;
}
const folders: Folder[] = [];
const sameSource = (a: Source, b: Source) => a.projectId === b.projectId && a.agentId === b.agentId;

/**
 * Who is told of each folder of the workspace, those there already first: the language servers will be started there
 * (A1, ESC-446), the extension API of the page given (`vscode`). Returns what stops it.
 */
export function onFolder(f: (folder: Folder, api: typeof vscode) => void): () => void {
  folderListeners.add(f);
  for (const folder of folders) f(folder, vscode);
  return () => folderListeners.delete(f);
}
const folderListeners = new Set<(folder: Folder, api: typeof vscode) => void>();

/**
 * The layer the workbench lives in: fixed over the window, never moved in the page. The workbench is laid over the
 * place of the view that shows it, and hidden off screen when none does. In the app's root (`#app > .root`, which is
 * never drawn again): it isolates its stacking, so that a layer beside it would lie over all of the app.
 */
function layer(): HTMLElement {
  let el = document.getElementById('vscode-layer');
  if (!el) {
    el = document.createElement('div');
    el.id = 'vscode-layer';
    // Over the panes' contents, under what is above them in the app (the bar of the view, the × of a pane, z-index 5
    // and 6) and under Escouade's menus, tooltips and dialogs, which are out of the root.
    el.style.cssText = 'position:fixed;inset:0;pointer-events:none;z-index:3;overflow:hidden;';
    (document.querySelector('#app > .root') ?? document.body).appendChild(el);
  }
  return el;
}

/**
 * What the view wants to know of the pointer on the workbench: it is not in the view's part of the page, so a press or
 * a hover there reaches no element of the view (its pane would not be made active, its × not shown).
 */
export interface Through {
  down?: () => void;
  hover?: (on: boolean) => void;
}
let through: Through = {};
function listenThrough(el: HTMLElement) {
  el.addEventListener('pointerdown', () => through.down?.(), true);
  el.addEventListener('pointerenter', () => through.hover?.(true));
  el.addEventListener('pointerleave', () => through.hover?.(false));
}

/** The view's place the workbench is laid over, where it was laid last, the frame that follows it. */
let shownIn: HTMLElement | null = null;
let laid = '';
let frame = 0;

function hide() {
  if (!host) return;
  host.style.visibility = 'hidden';
  host.style.left = '-10000px';
  laid = '';
}

/** Lays the workbench over its place, every frame while it is shown (a pane moved changes no size). */
function follow() {
  frame = 0;
  if (!host || !shownIn) return;
  if (!shownIn.isConnected) return hide();
  const r = shownIn.getBoundingClientRect();
  const at = [r.left, r.top, r.width, r.height].map(Math.round);
  const key = at.join();
  if (key !== laid) {
    const resized = at.slice(2).join() !== laid.split(',').slice(2).join();
    laid = key;
    Object.assign(host.style, {
      visibility: at[2] && at[3] ? 'visible' : 'hidden',
      left: `${at[0]}px`,
      top: `${at[1]}px`,
      width: `${at[2]}px`,
      height: `${at[3]}px`,
    });
    if (resized) void relayout();
  }
  frame = requestAnimationFrame(follow);
}

/** Escouade's theme as a theme extension. */
async function registerTheme() {
  const tokens = readTokens(document.querySelector('.app, #app') ?? document.documentElement);
  const dark = isDark(tokens);
  const ext = registerExtension(
    {
      name: 'escouade-theme',
      publisher: 'escouade',
      version: '1.0.0',
      engines: { vscode: '*' },
      contributes: { themes: [{ id: 'Escouade', label: 'Escouade', uiTheme: dark ? 'vs-dark' : 'vs', path: './escouade.json' }] },
    },
    ExtensionHostKind.LocalProcess,
  );
  const theme = new Blob([JSON.stringify(vscodeTheme(tokens, dark))], { type: 'application/json' });
  ext.registerFileUrl('./escouade.json', URL.createObjectURL(theme));
  await ext.whenReady();
}

const fold = /Win|Mac/i.test(navigator.userAgent);

async function start(first: Source, name: string, fontSize: number): Promise<void> {
  window.MonacoEnvironment = {
    // Absolute: a worker started from a blob has no base to read a path from.
    getWorkerUrl: (_: string, label: string) =>
      new URL(
        {
          editorWorkerService: editorWorker,
          TextMateWorker: textMateWorker,
          LocalFileSearchWorker: searchWorker,
          OutputLinkDetectionWorker: outputWorker,
        }[label] ?? editorWorker,
        location.href,
      ).href,
    getWorkerOptions: () => ({ type: 'module' }),
  };

  files = new EscouadeFileSystem<vscode.Uri>(
    api,
    {
      error: (message, code) => FileSystemProviderError.create(message, code as FileSystemProviderErrorCode),
      uri: ({ authority, path }) => vscode.Uri.from({ scheme: 'file', authority, path }),
    },
    fold,
  );
  registerFileSystemOverlay(1, files as unknown as IFileSystemProviderWithFileReadWriteCapability);
  onDiskChange((c) => void files?.diskChanged(c));
  const uri = await files.addSource(first);
  folders.push({ source: first, uri, name });
  await initFile(WORKSPACE, JSON.stringify(<IStoredWorkspace>{ folders: [{ uri: uri.toString(), name }] }, null, 2));
  await initUserConfiguration(
    JSON.stringify({
      'workbench.colorTheme': 'Escouade',
      'workbench.iconTheme': 'vs-seti',
      'workbench.startupEditor': 'none',
      'workbench.editor.enablePreview': false,
      'workbench.tips.enabled': false,
      'window.menuBarVisibility': 'hidden',
      'workbench.layoutControl.enabled': false,
      'window.commandCenter': false,
      'editor.fontFamily': "'JetBrains Mono', ui-monospace, Consolas, monospace",
      'editor.fontSize': fontSize,
      'editor.minimap.enabled': false,
      'files.autoSave': 'off',
      'extensions.ignoreRecommendations': true,
      'telemetry.telemetryLevel': 'off',
      'update.mode': 'none',
      'security.workspace.trust.enabled': false,
    }),
  );

  const themeReady = registerTheme();
  host = document.createElement('div');
  host.className = 'vscode-host';
  host.style.cssText = 'position:absolute;left:-10000px;top:0;width:1000px;height:700px;pointer-events:auto;visibility:hidden;';
  layer().appendChild(host);
  listenThrough(host);

  await initialize(
    {
      ...getLogServiceOverride(),
      ...getExtensionServiceOverride(),
      ...getModelServiceOverride(),
      ...getNotificationsServiceOverride(),
      ...getDialogsServiceOverride(),
      ...getConfigurationServiceOverride(),
      ...getKeybindingsServiceOverride(),
      ...getTextmateServiceOverride(),
      ...getThemeServiceOverride(),
      ...getLanguagesServiceOverride(),
      ...getPreferencesServiceOverride(),
      ...getOutputServiceOverride(),
      ...getMarkersServiceOverride(),
      ...getOutlineServiceOverride(),
      ...getExplorerServiceOverride(),
      ...getSearchServiceOverride(),
      ...getStorageServiceOverride(),
      ...getLifecycleServiceOverride(),
      ...getEnvironmentServiceOverride(),
      ...getWorkingCopyServiceOverride(),
      ...getFilesServiceOverride(),
      ...getWorkbenchServiceOverride(),
      ...getQuickAccessServiceOverride({ isKeybindingConfigurationVisible: () => true, shouldUseGlobalPicker: () => true }),
    },
    host,
    {
      enableWorkspaceTrust: false,
      developmentOptions: { logLevel: LogLevel.Warning },
      workspaceProvider: { trusted: true, open: async () => false, workspace: { workspaceUri: WORKSPACE } },
      productConfiguration: { nameShort: 'Escouade', nameLong: 'Escouade' },
    },
    { userHome: vscode.Uri.file('/') },
  );
  await registerExtension(
    { name: 'escouade', publisher: 'escouade', version: '1.0.0', engines: { vscode: '*' } },
    ExtensionHostKind.LocalProcess,
  ).setAsDefaultApi();
  await themeReady;
  for (const f of folderListeners) f(folders[0], vscode);
}

/**
 * The workbench, started the first time (on the source `src`, the editor's text at `fontSize`); the source's folder in
 * its workspace, named `name`.
 */
export async function workbench(src: Source, name: string, fontSize: number): Promise<void> {
  ready ??= start(src, name, fontSize).catch((e) => {
    ready = null;
    throw e;
  });
  await ready;
  if (folders.some((f) => sameSource(f.source, src))) return;
  const uri = await files!.addSource(src);
  if (folders.some((f) => sameSource(f.source, src))) return;
  const folder = { source: src, uri, name };
  folders.push(folder);
  vscode.workspace.updateWorkspaceFolders(vscode.workspace.workspaceFolders?.length ?? 0, 0, { uri, name });
  for (const f of folderListeners) f(folder, vscode);
}

/** Shows the workbench over `place` (a view's element); `pointer` tells the view of the presses and the hover on it. */
export function attach(place: HTMLElement, pointer: Through = {}) {
  if (!host) return;
  through.hover?.(false);
  shownIn = place;
  through = pointer;
  laid = '';
  if (!frame) frame = requestAnimationFrame(follow);
}

/** Hides the workbench, when `place` is still where it is shown (a view drawn again shows it in its new place first). */
export function detach(place: HTMLElement) {
  if (shownIn !== place) return;
  through.hover?.(false);
  through = {};
  shownIn = null;
  if (frame) cancelAnimationFrame(frame);
  frame = 0;
  hide();
}

async function relayout() {
  if (!host) return;
  (await getService(IWorkbenchLayoutService)).layout();
}

/** Opens the file `rel` of the source in a tab (the source's folder in the workspace first). */
export async function openFile(src: Source, rel: string) {
  const uri = files?.uriOf(src, rel);
  if (!uri) return;
  const doc = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(doc, { preview: false, preserveFocus: true });
}

/** Whether the workbench was started in this page. */
export const started = () => !!host;
