// ESC-444 (study): the workbench of VS Code (`@codingame/monaco-vscode-api`) in the editor of Escouade. Its services
// start once per page, in an element of their own (`host`) that is never destroyed nor moved in the page: it is laid
// over the place of the view that shows it (`VscodeView.svelte`, alone or in a pane) and hidden when that view goes, so
// that moving, closing and opening the pane again keep its tabs. One workbench, so one column of it at a time.
//
// The files are the core's (`fs-provider.ts`), the theme is Escouade's (`theme.ts`), the extensions come from Open VSX,
// a language server is started by the core (`lsp.ts`), and the Node extension host of VS Code when the study asks for
// it (`vscode_host_start`).

import {
  getService,
  IExtensionService,
  initialize,
  IWorkbenchLayoutService,
  IWorkbenchThemeService,
  LogLevel,
} from '@codingame/monaco-vscode-api';
import { ExtensionHostKind, registerExtension } from '@codingame/monaco-vscode-api/extensions';
import getConfigurationServiceOverride, {
  initUserConfiguration,
  type IStoredWorkspace,
} from '@codingame/monaco-vscode-configuration-service-override';
import getDialogsServiceOverride from '@codingame/monaco-vscode-dialogs-service-override';
import getEnvironmentServiceOverride from '@codingame/monaco-vscode-environment-service-override';
import getExplorerServiceOverride from '@codingame/monaco-vscode-explorer-service-override';
import getExtensionGalleryServiceOverride from '@codingame/monaco-vscode-extension-gallery-service-override';
import getExtensionServiceOverride from '@codingame/monaco-vscode-extensions-service-override';
import getFilesServiceOverride, {
  initFile,
  registerCustomProvider,
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
import getRemoteAgentServiceOverride from '@codingame/monaco-vscode-remote-agent-service-override';
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
import extensionHostWorker from '@codingame/monaco-vscode-api/workers/extensionHost.worker?worker&url';
import textMateWorker from '@codingame/monaco-vscode-textmate-service-override/worker?worker&url';
import searchWorker from '@codingame/monaco-vscode-search-service-override/worker?worker&url';
import outputWorker from '@codingame/monaco-vscode-output-service-override/worker?worker&url';

import { invoke } from '@tauri-apps/api/core';
import { api } from '../ipc';
import { EscouadeFileSystem } from './fs-provider';
import { readTokens, vscodeTheme } from './theme';

export interface WorkbenchOptions {
  /** Starts the Node extension host of VS Code with the system's Node, and connects to it. */
  nodeHost?: boolean;
}

/** What the study measures of a start (ms since the page's start, `performance.now()`). */
export const marks: Record<string, number> = {};
const mark = (name: string) => (marks[name] = Math.round(performance.now()));

/** The element the workbench lives in, lent to the view that shows it. */
let host: HTMLElement | null = null;
let ready: Promise<HTMLElement> | null = null;
let fsProvider: EscouadeFileSystem | null = null;
/** The projects already in the workspace: their folder's URI path. */
const folders = new Map<string, string>();
/** The Node extension host the view is connected to, when there is one. */
export let nodeHost: { authority: string; node: string } | null = null;

const WORKSPACE = vscode.Uri.file('/escouade.code-workspace');
/** The scheme of the projects' files in the workbench: `file`, or the Node host's `vscode-remote`. */
let files: { scheme: string; authority?: string } = { scheme: 'file' };
/** How the workbench names the projects' files. */
export const filesScheme = () => files;

/**
 * The layer the workbench lives in: fixed over the window, never moved in the page (an iframe moved in the DOM loads
 * again: the extension host's, the webviews'). The workbench is laid over the place of the view that shows it, and
 * hidden off screen when none does.
 */
function layer(): HTMLElement {
  let el = document.getElementById('vscode-layer');
  if (!el) {
    el = document.createElement('div');
    el.id = 'vscode-layer';
    // Over the panes, under Escouade's menus, tooltips and dialogs (and the × of a pane, z-index 5).
    el.style.cssText = 'position:fixed;inset:0;pointer-events:none;z-index:3;overflow:hidden;';
    document.body.appendChild(el);
  }
  return el;
}

/** The view's place the workbench is laid over, and where it was laid last. */
let shownIn: HTMLElement | null = null;
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
  const at = `${Math.round(r.left)},${Math.round(r.top)},${Math.round(r.width)},${Math.round(r.height)}`;
  if (at !== laid) {
    const resized = at.split(',').slice(2).join() !== laid.split(',').slice(2).join();
    laid = at;
    Object.assign(host.style, {
      visibility: r.width && r.height ? 'visible' : 'hidden',
      left: `${Math.round(r.left)}px`,
      top: `${Math.round(r.top)}px`,
      width: `${Math.round(r.width)}px`,
      height: `${Math.round(r.height)}px`,
    });
    if (resized) void relayout();
  }
  frame = requestAnimationFrame(follow);
}

/** Escouade's theme as a theme extension, and the settings of the view. */
async function registerTheme() {
  const tokens = readTokens(document.querySelector('.app, #app') ?? document.documentElement);
  // Dark when the panel is: the theme Escouade shows, whether chosen or the system's.
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(tokens.panel.slice(i, i + 2), 16));
  const dark = 0.2126 * r + 0.7152 * g + 0.0722 * b < 128;
  const theme = vscodeTheme(tokens, dark);
  const ext = registerExtension(
    {
      name: 'escouade-theme',
      publisher: 'escouade',
      version: '0.0.1',
      engines: { vscode: '*' },
      contributes: { themes: [{ id: 'Escouade', label: 'Escouade', uiTheme: dark ? 'vs-dark' : 'vs', path: './escouade.json' }] },
    },
    ExtensionHostKind.LocalProcess,
  );
  ext.registerFileUrl('./escouade.json', URL.createObjectURL(new Blob([JSON.stringify(theme)], { type: 'application/json' })));
  await ext.whenReady();
  return ext;
}

async function start(projectId: string, opts: WorkbenchOptions): Promise<HTMLElement> {
  mark('start');
  window.MonacoEnvironment = {
    // Absolute: the extension host's worker is started from a blob of its iframe, where a path means nothing.
    getWorkerUrl: (_: string, label: string) =>
      new URL(
        {
          editorWorkerService: editorWorker,
          extensionHostWorkerMain: extensionHostWorker,
          TextMateWorker: textMateWorker,
          LocalFileSearchWorker: searchWorker,
          OutputLinkDetectionWorker: outputWorker,
        }[label] ?? editorWorker,
        location.href,
      ).href,
    getWorkerOptions: () => ({ type: 'module' }),
  } as typeof window.MonacoEnvironment;

  let remote: { authority: string; token: string } | undefined;
  if (opts.nodeHost) {
    try {
      const h = await invoke<{ authority: string; token: string; node: string }>('vscode_host_start');
      remote = h;
      nodeHost = { authority: h.authority, node: h.node };
      mark('nodeHost');
    } catch (e) {
      console.warn('[vscode] Node extension host not started:', e);
    }
  }

  // With the Node extension host, the workspace is the server's (`vscode-remote:`): its extensions see its files as
  // their own (`file:` on their side). Those files are still read and written by the core, by the same provider.
  if (remote) {
    files = { scheme: 'vscode-remote', authority: remote.authority };
    fsProvider = new EscouadeFileSystem('vscode-remote');
    registerCustomProvider('vscode-remote', fsProvider);
  } else {
    fsProvider = new EscouadeFileSystem('file');
    registerFileSystemOverlay(1, fsProvider);
  }
  const root = await rootOf(projectId);
  folders.set(projectId, root.path);
  fsProvider.addRoot(projectId, root.path);
  await initFile(WORKSPACE, JSON.stringify(<IStoredWorkspace>{ folders: [{ uri: root.toString() }] }, null, 2));
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
      'editor.fontSize': 13,
      'editor.minimap.enabled': false,
      'files.autoSave': 'off',
      'extensions.ignoreRecommendations': true,
      'telemetry.telemetryLevel': 'off',
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
      ...getExtensionServiceOverride({ enableWorkerExtensionHost: true }),
      ...getExtensionGalleryServiceOverride({ webOnly: !remote }),
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
      ...getRemoteAgentServiceOverride({ scanRemoteExtensions: true }),
      ...getFilesServiceOverride(),
      ...getWorkbenchServiceOverride(),
      ...getQuickAccessServiceOverride({ isKeybindingConfigurationVisible: () => true, shouldUseGlobalPicker: () => true }),
    },
    host,
    {
      remoteAuthority: remote?.authority,
      connectionToken: remote?.token,
      enableWorkspaceTrust: false,
      developmentOptions: { logLevel: LogLevel.Warning },
      workspaceProvider: { trusted: true, open: async () => false, workspace: { workspaceUri: WORKSPACE } },
      productConfiguration: {
        nameShort: 'Escouade',
        nameLong: 'Escouade',
        extensionsGallery: {
          serviceUrl: 'https://open-vsx.org/vscode/gallery',
          resourceUrlTemplate: 'https://open-vsx.org/vscode/unpkg/{publisher}/{name}/{version}/{path}',
          extensionUrlTemplate: 'https://open-vsx.org/vscode/gallery/{publisher}/{name}/latest',
          controlUrl: '',
          nlsBaseUrl: '',
        },
      },
    },
    { userHome: vscode.Uri.file('/') },
  );
  mark('services');
  await registerExtension(
    { name: 'escouade', publisher: 'escouade', version: '0.0.1', engines: { vscode: '*' } },
    ExtensionHostKind.LocalProcess,
  ).setAsDefaultApi();
  await themeReady;
  mark('ready');
  return host;
}

async function rootOf(projectId: string) {
  const tree = await api.fsTree(projectId, null);
  return vscode.Uri.from({ ...files, path: vscode.Uri.file(tree.root).path });
}

/** The workbench, started on `projectId` the first time; that project's folder in its workspace. */
export async function workbench(projectId: string, opts: WorkbenchOptions = {}): Promise<HTMLElement> {
  ready ??= start(projectId, opts);
  const el = await ready;
  if (!folders.has(projectId)) {
    const root = await rootOf(projectId);
    folders.set(projectId, root.path);
    fsProvider!.addRoot(projectId, root.path);
    vscode.workspace.updateWorkspaceFolders(vscode.workspace.workspaceFolders?.length ?? 0, 0, { uri: root });
  }
  return el;
}

/** Shows the workbench over `place` (a view's element). */
export async function attach(place: HTMLElement, pointer: Through = {}) {
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

export async function relayout() {
  if (!host) return;
  const layout = await getService(IWorkbenchLayoutService);
  layout.layout();
}

/** Opens the file `rel` of the project in an editor tab. */
export async function openFile(projectId: string, rel: string, opts: { preview?: boolean } = {}) {
  const root = folders.get(projectId);
  if (!root) return;
  const uri = vscode.Uri.from({ ...files, path: `${root}/${rel}` });
  const doc = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(doc, { preview: opts.preview ?? false, preserveFocus: true });
}

/** The URI path of the project's folder in the workspace. */
export const folderOf = (projectId: string) => folders.get(projectId) ?? null;

/** What the provider's calls to the core took. */
export const coreTimings = () => fsProvider?.timings ?? [];

/** Whether the workbench was started in this page. */
export const started = () => !!host;

/**
 * A second column of the editor, without a second workbench: a code editor of its own in `el`, on the same model as the
 * workbench's tab of `rel` (its text, its changes, its diagnostics, the theme are shared). It has no tabs nor tree of
 * VS Code's: those would be Escouade's. Returns what closes it.
 */
export async function standaloneEditor(rel: string, el: HTMLElement, projectId = [...folders.keys()][0]) {
  const monaco = await import('monaco-editor');
  const root = folders.get(projectId);
  if (!root) throw new Error('project not open');
  const uri = vscode.Uri.from({ ...files, path: `${root}/${rel}` });
  // The workbench's document: its model, the one the tab edits.
  const doc = await vscode.workspace.openTextDocument(uri);
  const model = monaco.editor.getModel(monaco.Uri.parse(doc.uri.toString()));
  if (!model) throw new Error('no model');
  const editor = monaco.editor.create(el, { model, automaticLayout: true, minimap: { enabled: false } });
  return () => editor.dispose();
}

/** For the study's tests and bench: the extension API of the view, and what it measured. */
(globalThis as Record<string, unknown>).__escouadeVscode = {
  vscode,
  marks,
  openFile,
  folderOf,
  coreTimings,
  standaloneEditor,
  /** The extensions the workbench runs, in which host, activated or not, with their errors. */
  async extensions() {
    const service = await getService(IExtensionService);
    const status = service.getExtensionsStatus();
    return service.extensions.map((e) => {
      const st = status[e.identifier.value];
      return {
        id: e.identifier.value.toLowerCase(),
        builtin: e.isBuiltin,
        browser: !!e.browser,
        node: !!e.main,
        activated: !!st?.activationTimes,
        errors: (st?.runtimeErrors ?? []).map(String),
        messages: (st?.messages ?? []).map((m) => m.message),
      };
    });
  },
  async colorThemes() {
    const themes = await getService(IWorkbenchThemeService);
    return (await themes.getColorThemes()).map((t) => t.label);
  },
  get nodeHost() {
    return nodeHost;
  },
};
