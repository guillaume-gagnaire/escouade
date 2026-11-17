// The workbench of VS Code (`@codingame/monaco-vscode-api`, pinned), the editor of a project in Escouade (ESC-444's
// study, taken in ESC-447 in place of the CodeMirror editor). Its services start once per page, in an element
// of their own (`host`) that is never destroyed nor moved in the page: it is laid over the place of the view that shows
// it (`VscodeView.svelte`, alone or in a pane) and hidden when that view goes, so that moving, closing and opening the
// pane again keep its tabs (an iframe or a webview of VS Code moved in the DOM loads again). One workbench, so one
// column of it at a time.
//
// - The files are the core's (`fs-provider.ts`): each source shown (the project's checkout, an agent's worktree) is a
//   folder of its workspace, and the changes the core's watcher tells are VS Code's file changes.
// - The theme is Escouade's (`theme.ts`), followed as it changes (`followTheme`), and its fonts (`fontSettings`); its
//   language is the one it was loaded in (`load.ts`).
// - No extension host of its own: the default extensions it carries are declarative (languages, grammars, themes), so
//   that the page needs neither `unsafe-eval` nor an iframe. The extensions of Open VSX (A5, ESC-481) come by the core,
//   checked, and only the declarative ones are laid (`useExtensions`): one with code waits for the extension host of its
//   own origin (A6). The language servers (A1, ESC-446: `onFolder` tells it each folder) and the Node extension host
//   (N2) are apart.

import {
  getService,
  IKeybindingService,
  initialize,
  IWorkbenchLayoutService,
  IWorkingCopyService,
  IWorkspaceContextService,
  IWorkspaceEditingService,
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
import getExtensionServiceOverride from '@codingame/monaco-vscode-extensions-service-override';
import getFilesServiceOverride, {
  FileSystemProviderError,
  type FileSystemProviderErrorCode,
  type IFileSystemProviderWithFileReadWriteCapability,
  initFile,
  registerFileSystemOverlay,
} from '@codingame/monaco-vscode-files-service-override';
import getKeybindingsServiceOverride, { updateUserKeybindings } from '@codingame/monaco-vscode-keybindings-service-override';
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
import getScmServiceOverride from '@codingame/monaco-vscode-scm-service-override';
import getSearchServiceOverride from '@codingame/monaco-vscode-search-service-override';
import getStorageServiceOverride from '@codingame/monaco-vscode-storage-service-override';
import getTextmateServiceOverride from '@codingame/monaco-vscode-textmate-service-override';
import getThemeServiceOverride from '@codingame/monaco-vscode-theme-service-override';
import getWorkbenchServiceOverride from '@codingame/monaco-vscode-workbench-service-override';
import getWorkingCopyServiceOverride from '@codingame/monaco-vscode-working-copy-service-override';
import * as vscode from 'vscode';
import 'vscode/localExtensionHost';
import { IPaneCompositePartService } from '@codingame/monaco-vscode-api/vscode/vs/workbench/services/panecomposite/browser/panecomposite.service';
import { VIEWLET_ID as SCM_VIEWLET_ID } from '@codingame/monaco-vscode-api/vscode/vs/workbench/contrib/scm/common/scm';

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
import { IS_MAC } from '../platform';
import { type ExtState, type FileInfo, chosenTheme, filesToLay, runs } from './extensions';
import { EscouadeFileSystem, FileType, type Source } from './fs-provider';
import { appTakes, utf16Column } from './keys';
import { closePreviews, registerPreviews } from './previews/panes';
import { previewOf } from './previews/kinds';
import { addSource, contributions, followChanges, removeProject } from './scm';
import { THEME_IDS, readTokens, themeSettings, vscodeTheme } from './theme';
import { type Unsaved, viewUnsaved } from './unsaved.svelte';

export type { Source } from './fs-provider';

/**
 * VS Code's view of the source control (its commit, push and sync buttons) is never shown (ESC-476): the service
 * override of the source control brings it with the quick diff, which does not work without the rest of it (its
 * container or its views taken out of the registry, the marks are gone too). So its entry of the activity bar is hidden
 * (`SCM_HIDDEN`), its keys are taken off (`keepAppKeys`), and opening it any other way (its palette's « Afficher »)
 * shows the explorer instead. Escouade's own commits and pushes stay.
 */
async function withoutSourceControlView() {
  const sheet = document.createElement('style');
  sheet.textContent = SCM_HIDDEN;
  document.head.append(sheet);
  const panes = await getService(IPaneCompositePartService);
  panes.onDidPaneCompositeOpen(({ composite, viewContainerLocation }) => {
    if (composite.getId() === SCM_VIEWLET_ID) void panes.openPaneComposite(EXPLORER_VIEWLET_ID, viewContainerLocation, true);
  });
}
const EXPLORER_VIEWLET_ID = 'workbench.view.explorer';
/** The sheet that hides the source control's entry of the activity bar. */
const SCM_HIDDEN = '.vscode-host .activitybar .action-item:has(.codicon-source-control-view-icon) { display: none !important; }';

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
  // Come in from the keyboard too (VS Code's F6, a dialog closed): its pane is the active one.
  el.addEventListener('focusin', () => through.down?.());
  el.addEventListener('pointerenter', () => through.hover?.(true));
  el.addEventListener('pointerleave', () => through.hover?.(false));
}

/** The view's place the workbench is laid over, where it was laid last, the frame that follows it. */
let shownIn: HTMLElement | null = null;
let laid = '';
let frame = 0;

function hide() {
  if (!host) return;
  // The keyboard is never left in a workbench no one sees: what it typed would go in a hidden file.
  if (host.contains(document.activeElement)) (document.activeElement as HTMLElement).blur();
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

/** Where the tokens are read: the root, which carries the project's tint and the theme. */
const tokenRoot = () => document.querySelector('.app, #app') ?? document.documentElement;

/**
 * Escouade's two themes, dark and light, as a theme extension: VS Code's defaults of their kind under the colours they
 * start with. The colours are then the user's customizations (`themeSettings`), laid again as the tokens change.
 */
async function registerTheme() {
  const tokens = readTokens(tokenRoot());
  const ext = registerExtension(
    {
      name: 'escouade-theme',
      publisher: 'escouade',
      version: '1.0.0',
      engines: { vscode: '*' },
      contributes: {
        themes: [
          { id: THEME_IDS.dark, label: THEME_IDS.dark, uiTheme: 'vs-dark', path: './escouade-dark.json' },
          { id: THEME_IDS.light, label: THEME_IDS.light, uiTheme: 'vs', path: './escouade-light.json' },
        ],
      },
    },
    ExtensionHostKind.LocalProcess,
  );
  for (const dark of [true, false]) {
    const theme = new Blob([JSON.stringify(vscodeTheme(tokens, dark))], { type: 'application/json' });
    ext.registerFileUrl(dark ? './escouade-dark.json' : './escouade-light.json', URL.createObjectURL(theme));
  }
  await ext.whenReady();
}

/** The editor's font: the app's code font (`--mono`), its weight and its ligatures, at `size` (the WebView's zoom over it). */
export function fontSettings(size: number, root: Element = document.documentElement) {
  const mono = getComputedStyle(root).getPropertyValue('--mono').trim();
  return {
    'editor.fontFamily': mono || "'JetBrains Mono', ui-monospace, Consolas, monospace",
    'editor.fontSize': size,
    'editor.fontWeight': 'normal',
    // As the app's code (its diffs, its terminals' font): JetBrains Mono's contextual ligatures, which the browser shows.
    'editor.fontLigatures': true,
  };
}

/** The settings of the theme and the fonts the view was given last, as written: written again only when they change. */
let laidLook = '';

/** Lays `settings` on the user's configuration, those that changed. */
async function laySettings(settings: Record<string, unknown>) {
  const cfg = vscode.workspace.getConfiguration();
  for (const [key, value] of Object.entries(settings))
    if (JSON.stringify(cfg.get(key)) !== JSON.stringify(value)) await cfg.update(key, value, vscode.ConfigurationTarget.Global);
}

/** The theme read again from the tokens, and laid if it changed: the colours of every part of the view at once. */
async function retheme() {
  if (!host) return;
  // Escouade's theme, unless the user chose one of an extension (ESC-481): that one wins, Escouade's colours off, until
  // « Revenir au thème d'Escouade ».
  const look = extensionTheme
    ? { 'workbench.colorTheme': extensionTheme, 'workbench.colorCustomizations': {}, 'editor.tokenColorCustomizations': {} }
    : themeSettings(readTokens(tokenRoot()));
  const settings = { ...look, ...fontSettings(fontSize) };
  const key = JSON.stringify(settings);
  if (key === laidLook) return;
  laidLook = key;
  await laySettings(settings);
}

/** How long `--p` glides from a project's colour to the next (src/app.css), after which the view takes it. */
const GLIDE_MS = 750;

/**
 * Follows the window's theme (`data-theme`: Réglages, the system's), the tint of the project (`--p` on the root),
 * a plugin's theme (`data-plugin-theme` and its sheet in the head), at once and once the tint has glided: no reload,
 * the tabs kept.
 */
function followTheme() {
  let soon = 0;
  let late: ReturnType<typeof setTimeout> | undefined;
  const changed = () => {
    if (!soon) soon = requestAnimationFrame(() => ((soon = 0), void retheme()));
    clearTimeout(late);
    late = setTimeout(() => void retheme(), GLIDE_MS);
  };
  new MutationObserver(changed).observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme', 'data-plugin-theme', 'style', 'class'],
  });
  new MutationObserver(changed).observe(document.head, { childList: true, subtree: true, characterData: true });
  matchMedia('(prefers-color-scheme: light)').addEventListener('change', changed);
}

/** The editor's text size, « Taille du texte de l'éditeur » of the settings. */
let fontSize = 13;

/** The colour theme of an extension chosen in the view (its id), null: Escouade's. */
let extensionTheme: string | null = null;

/** An extension of Open VSX laid in the view: its version, and what takes it away again. */
interface Laid {
  version: string;
  dispose: () => Promise<void>;
}
const laidExtensions = new Map<string, Laid>();

/** What the view reads of the core to lay an extension (the tests give their own). */
export interface ExtensionFiles {
  files: (id: string) => Promise<FileInfo[]>;
  file: (id: string, path: string) => Promise<ArrayBuffer>;
}
const coreFiles: ExtensionFiles = { files: (id) => api.vsxFiles(id), file: (id, path) => api.vsxFile(id, path) };

/**
 * An installed extension laid in the view, declarative alone: its `package.json` registered, each file it reads given
 * by the core (`vsx_file`, read-only, inside its folder) as a `blob:` URL of its type, a font as a `data:` URL (the
 * window's CSP takes no other: it is not widened). No code of it runs.
 */
async function layExtension(e: ExtState, read: ExtensionFiles): Promise<Laid> {
  const all = await read.files(e.key);
  const manifest = JSON.parse(new TextDecoder().decode(await read.file(e.key, 'package.json')));
  delete manifest.main;
  delete manifest.browser;
  // Every file read first: VS Code reads some (a language's configuration) as soon as the extension is added.
  const urls: string[] = [];
  const laid: { path: string; url: string; mime: string }[] = [];
  const lay = filesToLay(all);
  for (let i = 0; i < lay.length; i += 16) {
    await Promise.all(
      lay.slice(i, i + 16).map(async (f) => {
        const bytes = await read.file(e.key, f.path);
        let url: string;
        if (f.mime.startsWith('font/')) {
          let bin = '';
          const view = new Uint8Array(bytes);
          for (let j = 0; j < view.length; j += 0x8000) bin += String.fromCharCode(...view.subarray(j, j + 0x8000));
          url = `data:${f.mime};base64,${btoa(bin)}`;
        } else {
          url = URL.createObjectURL(new Blob([bytes], { type: f.mime }));
          urls.push(url);
        }
        laid.push({ path: f.path, url, mime: f.mime.split(';')[0] });
      }),
    );
  }
  // A location of its own per version: an update laid while the version before goes away does not meet its files.
  const ext = registerExtension(manifest, ExtensionHostKind.LocalProcess, { system: false, path: `/${e.version}` });
  const registered = laid.map((f) => ext.registerFileUrl(`./${f.path}`, f.url, f.mime));
  await ext.whenReady();
  return {
    version: e.version,
    dispose: async () => {
      await ext.dispose();
      for (const r of registered) r.dispose();
      for (const u of urls) URL.revokeObjectURL(u);
    },
  };
}

/**
 * The extensions installed, as the core says them, laid in the view: those that run (declarative, on, allowed) added,
 * an updated one laid again, the others taken away; then the theme chosen (`<key>/<id>`), if its extension runs.
 */
export async function useExtensions(list: ExtState[], theme: string | null, read: ExtensionFiles = coreFiles) {
  if (!host) return;
  const wanted = new Map(list.filter(runs).map((e) => [e.key, e]));
  for (const [key, laid] of [...laidExtensions]) {
    if (wanted.get(key)?.version === laid.version) continue;
    laidExtensions.delete(key);
    await laid.dispose().catch((err) => console.warn('[vscode] extension', key, err));
  }
  for (const [key, e] of wanted) {
    if (laidExtensions.has(key)) continue;
    try {
      laidExtensions.set(key, await layExtension(e, read));
    } catch (err) {
      console.warn('[vscode] extension', key, err);
    }
  }
  const next = chosenTheme(list, theme);
  if (next !== extensionTheme) {
    extensionTheme = next;
    laidLook = '';
    await retheme();
  }
}

/** The extensions laid in the view now (their keys): what the e2e read. */
export const laidExtensionKeys = () => [...laidExtensions.keys()];

/** The projects whose folders are in the view's workspace: their `.escouade.yml` and their privacy count for the extensions. */
export const viewProjects = () => [...new Set(folders.map((f) => f.source.projectId))];

/** The colour theme the view shows now (`workbench.colorTheme`). */
export const colorTheme = () => vscode.workspace.getConfiguration('workbench').get<string>('colorTheme') ?? '';

/** « Taille du texte de l'éditeur » changed: the view takes it at once. */
export function setFontSize(size: number) {
  if (size === fontSize) return;
  fontSize = size;
  void retheme();
}

const fold = /Win|Mac/i.test(navigator.userAgent);
const stillMotion = matchMedia('(prefers-reduced-motion: reduce)');

async function start(first: Source, name: string, size: number): Promise<void> {
  fontSize = size;
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
  withFolderFault(files);
  registerFileSystemOverlay(1, files as unknown as IFileSystemProviderWithFileReadWriteCapability);
  onDiskChange((c) => void files?.diskChanged(c));
  const uri = await files.addSource(first);
  folders.push({ source: first, uri, name });
  await initFile(WORKSPACE, JSON.stringify(<IStoredWorkspace>{ folders: [{ uri: uri.toString(), name }] }, null, 2));
  await initUserConfiguration(
    JSON.stringify({
      'workbench.iconTheme': 'vs-seti',
      'workbench.startupEditor': 'none',
      'workbench.editor.enablePreview': false,
      'workbench.tips.enabled': false,
      'window.menuBarVisibility': 'hidden',
      'workbench.layoutControl.enabled': false,
      'window.commandCenter': false,
      'editor.minimap.enabled': false,
      // The caret stays still with prefers-reduced-motion, as the rest of the app (followed below).
      'editor.cursorBlinking': stillMotion.matches ? 'solid' : 'blink',
      'files.autoSave': 'off',
      'extensions.ignoreRecommendations': true,
      'telemetry.telemetryLevel': 'off',
      'update.mode': 'none',
      'security.workspace.trust.enabled': false,
      ...themeSettings(readTokens(tokenRoot())),
      ...fontSettings(fontSize),
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
      // VS Code's quick diff (the marks of the changes from the base), without its view of the source control (ESC-476).
      ...getScmServiceOverride(),
      ...getFilesServiceOverride(),
      ...getWorkbenchServiceOverride(),
      ...getQuickAccessServiceOverride({ isKeybindingConfigurationVisible: () => true, shouldUseGlobalPicker: () => true }),
    },
    host,
    {
      enableWorkspaceTrust: false,
      developmentOptions: { logLevel: LogLevel.Warning },
      workspaceProvider: { trusted: true, open: async () => false, workspace: { workspaceUri: WORKSPACE } },
      productConfiguration: {
        nameShort: 'Escouade',
        nameLong: 'Escouade',
        // The page's own extension reads the diff information of an editor (ESC-476: the lines changed, in the view's bar).
        extensionEnabledApiProposals: { 'escouade.escouade': ['textEditorDiffInformation'] },
      },
    },
    { userHome: vscode.Uri.file('/') },
  );
  await registerExtension(
    {
      name: 'escouade',
      publisher: 'escouade',
      version: '1.0.0',
      engines: { vscode: '*' },
      // The diff information of an editor: the lines its quick diff found changed, counted in the view's bar (ESC-476).
      enabledApiProposals: ['textEditorDiffInformation'],
      contributes: contributions(),
    },
    ExtensionHostKind.LocalProcess,
  ).setAsDefaultApi();
  followChanges(files, fold);
  await withoutSourceControlView();
  addSource(first, uri, name);
  await themeReady;
  // The previews (ESC-473): their editors registered, what they draw loaded when a file of theirs opens.
  await registerPreviews({
    sourceOf: (uri) => files?.sourceOf(uri) ?? null,
    uriOf: (src, rel) => files?.uriOf(src, rel) ?? null,
    openFile: (src, rel, at) => openFile(src, rel, at),
  });
  followTheme();
  void retheme();
  await keepAppKeys();
  stillMotion.addEventListener('change', () => {
    void vscode.workspace
      .getConfiguration('editor')
      .update('cursorBlinking', stillMotion.matches ? 'solid' : 'blink', vscode.ConfigurationTarget.Global);
  });
  await followUnsaved();
  // A folder of a source the workspace lost afterwards (its file read again, a change on its way): brought back.
  (await getService(IWorkspaceContextService)).onDidChangeWorkspaceFolders(() => void reconcileFolders());
  for (const f of folderListeners) f(folders[0], vscode);
}

/**
 * The keys Escouade keeps in the view (`keys.ts`) taken off VS Code's keybindings: they never reach it, and its menus
 * and its palette show no key that does something else. Its palette still runs those commands by their names.
 */
async function keepAppKeys() {
  const keys = await getService(IKeybindingService);
  const off = keys
    .getKeybindings()
    .filter(
      (k) =>
        k.command &&
        !k.command.startsWith('-') &&
        // The view of the source control's key goes too: the view is never shown (`withoutSourceControlView`).
        (k.command === SCM_VIEWLET_ID || appTakes(k.resolvedKeybinding?.getUserSettingsLabel(), k.command, IS_MAC)),
    )
    .map((k) => ({ key: k.resolvedKeybinding!.getUserSettingsLabel(), command: `-${k.command}`, when: k.when?.serialize() }));
  await updateUserKeybindings(JSON.stringify(off));
}

/** The files modified and not saved, told to the app as they change (`unsaved.svelte.ts`), and what it asks of them. */
async function followUnsaved() {
  const copies = await getService(IWorkingCopyService);
  const tell = () => {
    const list: Unsaved[] = [];
    for (const c of copies.dirtyWorkingCopies) {
      const at = files?.sourceOf(c.resource);
      if (at)
        list.push({ projectId: at.source.projectId, source: at.source.agentId ?? 'project', path: at.rel, uri: c.resource.toString() });
    }
    viewUnsaved.set(list);
  };
  copies.onDidChangeDirty(tell);
  copies.onDidRegister(tell);
  copies.onDidUnregister(tell);
  tell();
  const those = (uris: string[]) => copies.dirtyWorkingCopies.filter((c) => uris.includes(c.resource.toString()));
  viewUnsaved.serve({
    save: async (uris) => {
      for (const c of those(uris)) await c.save().catch(() => false);
      return those(uris).length === 0;
    },
    revert: async (uris) => {
      for (const c of those(uris)) await c.revert().catch(() => {});
    },
    forget: forgetProject,
  });
}

/** A folder's URI as the workspace is compared with it: the drive's letter in any case, no slash at its end. */
const folderKey = (uri: { toString(): string }) => uri.toString().toLowerCase().replace(/\/+$/, '');

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/**
 * A fault the e2e inject (`localStorage['escouade.test.vscodeFolders']`, JSON `{ drop, delay }`): the first `drop`
 * reads of the root of a source being added to the workspace answer, after `delay` ms, that it is not a folder. VS Code
 * leaves a folder that is not one out of the workspace without a word (`doUpdateFolders`): what a late or wrong answer of
 * a loaded machine does. Nothing set, nothing done.
 */
export const FOLDER_FAULTS_KEY = 'escouade.test.vscodeFolders';
function folderFault(): { drop: boolean; delay: number } {
  try {
    const f = JSON.parse(localStorage.getItem(FOLDER_FAULTS_KEY) ?? 'null') as { drop?: number; delay?: number } | null;
    if (!f) return { drop: false, delay: 0 };
    const drop = (f.drop ?? 0) > 0;
    if (drop) localStorage.setItem(FOLDER_FAULTS_KEY, JSON.stringify({ ...f, drop: (f.drop ?? 0) - 1 }));
    return { drop, delay: f.delay ?? 0 };
  } catch {
    return { drop: false, delay: 0 };
  }
}

/** The provider's `stat`, with the fault above on the root of a source the workspace does not show yet. */
function withFolderFault(fs: EscouadeFileSystem<vscode.Uri>) {
  const stat = fs.stat.bind(fs);
  fs.stat = async (uri) => {
    const st = await stat(uri);
    if (st.type !== FileType.Directory || fs.sourceOf(uri)?.rel !== '' || localStorage.getItem(FOLDER_FAULTS_KEY) === null) return st;
    const context = await getService(IWorkspaceContextService);
    if (context.getWorkspace().folders.some((f) => folderKey(f.uri) === folderKey(uri))) return st;
    const fault = folderFault();
    if (fault.delay) await sleep(fault.delay);
    return fault.drop ? { ...st, type: FileType.File } : st;
  };
}

/** How many rounds the workspace is brought to its sources before it is said not to come (about 20 s). */
const ROUNDS = 25;

let reconciling: Promise<void> | null = null;
let reconcileAgain = false;

/**
 * The workspace brought to the folders of the sources shown (`folders`, the reference), as the explorer shows them:
 * the main side's list (`IWorkspaceContextService`), never the extension host's, which says a folder asked for at once,
 * before the main side took it, and forgets it without a word when it is refused (ESC-481). Each round adds what is
 * missing by the main side's editing service, awaited, then waits for the workspace to change or for a delay that grows;
 * after `ROUNDS`, what is still missing is said in the console, with the last refusal, and the round ends. One at a
 * time; a call during it makes one more.
 */
function reconcileFolders(): Promise<void> {
  if (reconciling) {
    reconcileAgain = true;
    return reconciling;
  }
  reconciling = (async () => {
    const context = await getService(IWorkspaceContextService);
    const editing = await getService(IWorkspaceEditingService);
    const missing = () => {
      const shown = new Set(context.getWorkspace().folders.map((f) => folderKey(f.uri)));
      return folders.filter((f) => !shown.has(folderKey(f.uri)));
    };
    let refusal = '';
    for (let round = 0; round < ROUNDS; round++) {
      reconcileAgain = false;
      const lacking = missing();
      if (!lacking.length) return;
      try {
        await editing.addFolders(
          lacking.map((f) => ({ uri: f.uri, name: f.name })),
          true,
        );
      } catch (e) {
        refusal = String(e);
      }
      if (!missing().length) return;
      // The main side tells the change once the workspace file is read again: that, or a delay that grows.
      await new Promise<void>((done) => {
        const sub = context.onDidChangeWorkspaceFolders(() => {
          sub.dispose();
          done();
        });
        setTimeout(
          () => {
            sub.dispose();
            done();
          },
          Math.min(100 * 2 ** round, 1000),
        );
      });
    }
    const still = missing();
    if (still.length)
      console.error(
        `[vscode] workspace: ${still.map((f) => f.name).join(', ')} not in the explorer after ${ROUNDS} rounds${refusal ? ` (last: ${refusal})` : ''}`,
      );
  })().finally(() => {
    reconciling = null;
    if (reconcileAgain) void reconcileFolders();
  });
  return reconciling;
}

/** Whether the explorer shows the folder of `uri`. */
async function inExplorer(uri: vscode.Uri): Promise<boolean> {
  const context = await getService(IWorkspaceContextService);
  return context.getWorkspace().folders.some((f) => folderKey(f.uri) === folderKey(uri));
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
  if (folders.some((f) => sameSource(f.source, src))) {
    // Asked again (the view drawn again, the source picked again): still not shown, it is brought there once more.
    const there = folders.find((f) => sameSource(f.source, src))!;
    if (!(await inExplorer(there.uri))) await reconcileFolders();
    return;
  }
  // The source's files: a tree the core may be slow to give under load, asked again before it is said.
  let uri: vscode.Uri | null = null;
  for (let i = 0; !uri; i++) {
    try {
      uri = await files!.addSource(src);
    } catch (e) {
      if (i >= 2) throw e;
      await sleep(500 * (i + 1));
    }
  }
  if (folders.some((f) => sameSource(f.source, src))) return;
  const folder = { source: src, uri, name };
  folders.push(folder);
  // Added by the main side, not through the extension host (whose list says a folder before the main side took it):
  // the explorer shows it when this returns, or the console says why not.
  await reconcileFolders();
  if (!(await inExplorer(uri))) await reconcileFolders();
  addSource(src, uri, name);
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

/** Once the workbench is laid out over its place: a hidden element takes no focus. */
async function shown(): Promise<boolean> {
  for (let i = 0; i < 30; i++) {
    if (host && shownIn && host.style.visibility === 'visible') return true;
    await new Promise((r) => requestAnimationFrame(r));
  }
  return false;
}

/**
 * Opens the file `rel` of the source in a tab (the source's folder in the workspace first), at `line` and `col` when
 * given (1-based, the column in characters), the keyboard in it.
 */
export async function openFile(src: Source, rel: string, at: { line?: number; col?: number } = {}) {
  const uri = files?.uriOf(src, rel);
  if (!uri) return;
  let selection: vscode.Range | undefined;
  // A file shown in its preview has no text to put a caret in; a Markdown one is brought to its line by its preview.
  const kind = previewOf(rel)?.kind;
  if (at.line && (!kind || kind === 'markdown' || kind === 'csv')) {
    // A binary file has no line: it opens at its top.
    const doc = await Promise.resolve(vscode.workspace.openTextDocument(uri)).catch(() => null);
    if (doc) {
      const line = Math.min(Math.max(at.line, 1), doc.lineCount) - 1;
      const pos = new vscode.Position(line, utf16Column(doc.lineAt(line).text, at.col ?? 1));
      selection = new vscode.Range(pos, pos);
    }
  }
  const focus = await shown();
  await vscode.commands.executeCommand('vscode.open', uri, <vscode.TextDocumentShowOptions>{
    preview: false,
    preserveFocus: !focus,
    selection,
  });
  const editor = vscode.window.activeTextEditor;
  if (selection && editor?.document.uri.toString() === uri.toString())
    editor.revealRange(selection, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
}

/** The keyboard in the workbench: its editor shown, else its explorer. */
export async function focus() {
  if (!(await shown())) return;
  const editors = vscode.window.tabGroups.all.some((g) => g.tabs.length);
  await vscode.commands.executeCommand(editors ? 'workbench.action.focusActiveEditorGroup' : 'workbench.explorer.fileView.focus');
}

/** Whether the workbench has a tab open. */
export const hasTabs = () => vscode.window.tabGroups.all.some((g) => g.tabs.length);

/**
 * A closed project's folders leave the workspace, their tabs closed: their changes were saved or given up when the
 * close was asked (`close-unsaved.ts`).
 */
export async function forgetProject(projectId: string) {
  if (!host) return;
  const mine = (uri: vscode.Uri | undefined) => !!uri && files?.sourceOf(uri)?.source.projectId === projectId;
  const tabs = vscode.window.tabGroups.all
    .flatMap((g) => g.tabs)
    .filter(
      (t) =>
        (t.input instanceof vscode.TabInputText && mine(t.input.uri)) ||
        // Its comparisons with the base (the file, or its base for a file deleted).
        (t.input instanceof vscode.TabInputTextDiff && mine(t.input.original.with({ scheme: 'file', query: '' }))),
    );
  if (tabs.length) await vscode.window.tabGroups.close(tabs, true);
  await closePreviews(mine);
  // Out of the reference first: the workspace is not brought back to them.
  const gone = folders.filter((f) => f.source.projectId === projectId);
  for (const f of gone) folders.splice(folders.indexOf(f), 1);
  if (gone.length)
    await (
      await getService(IWorkspaceEditingService)
    )
      .removeFolders(
        gone.map((f) => f.uri),
        true,
      )
      .catch((e) => console.error('[vscode] workspace: the folders of a project closed stay', e));
  removeProject(projectId);
}

/** Whether the workbench was started in this page. */
export const started = () => !!host;

/** The tabs of the view, of the files of a source (its project's, an agent's), the one shown last. */
export function openTabs(): { projectId: string; source: string; path: string }[] {
  const tab = (t: vscode.Tab | undefined) => {
    const at = t?.input instanceof vscode.TabInputText ? files?.sourceOf(t.input.uri) : undefined;
    return at ? { projectId: at.source.projectId, source: at.source.agentId ?? 'project', path: at.rel } : null;
  };
  const active = vscode.window.tabGroups.activeTabGroup?.activeTab;
  const all = vscode.window.tabGroups.all.flatMap((g) => g.tabs).filter((t) => t !== active);
  return [...all, active].map(tab).filter((t) => t !== null);
}
