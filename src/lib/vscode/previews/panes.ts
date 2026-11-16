// The previews of the VS Code view (ESC-473): editors of Escouade's own in VS Code's workbench, one per family of
// files (`kinds.ts`), drawn in the page by Escouade, never in a webview of VS Code (A6, the extension host on an
// origin of its own, is not there): no iframe, no script of a document, nothing loaded from the web without a click.
//
// - Each family is an editor of VS Code's editor resolver (`registerEditor`): a click in the explorer, a link of a
//   conversation (`vscode.open`), Ctrl+P open it; « Rouvrir l’éditeur avec… » (« Reopen Editor With… ») goes back to the text editor. The tables
//   (`.csv`) open as text first, their table on demand.
// - The files are read by the core, as every file of the view (`vscode.workspace.fs`, the provider of `fs-provider.ts`).
// - What weighs is loaded when a file of its family is opened, never when the view starts: pdf.js (`pdf.ts`), mammoth
//   for Word (`docx.ts`), the highlighter of code blocks (`markdown.ts` of the conversation).
// - Markdown: « Ouvrir l’aperçu » and « Ouvrir l’aperçu sur le côté » (Ctrl+Shift+V, Ctrl+K V, the button of the editor's
//   title), as VS Code names them; the preview follows the source's scroll and its typing.

import { getService, IEditorService } from '@codingame/monaco-vscode-api';
import { ExtensionHostKind, registerExtension } from '@codingame/monaco-vscode-api/extensions';
import {
  EditorInputCapabilities,
  EditorPane,
  registerEditor,
  registerEditorPane,
  RegisteredEditorPriority,
  SimpleEditorInput,
} from '@codingame/monaco-vscode-workbench-service-override';
import * as vscode from 'vscode';

import { fSize } from '../../format';
import { t } from '../../i18n';
import { api } from '../../ipc';
import { highlightWithin } from '../../markdown';
import type { Source } from '../fs-provider';
import { parseTable, separatorOf } from './csv';
import { extOf, globOf, KINDS, linkOf, MAX_PREVIEW_BYTES, NO_PREVIEW, opensInPreview, previewOf, type PreviewKind } from './kinds';
import { lineFor, renderPreview, scrollFor } from './markdown';
import './previews.css';

/** What the previews need of the workbench: where a URI is in the sources, a file of a source opened in the view. */
export interface PreviewHost {
  sourceOf(uri: vscode.Uri): { source: Source; rel: string } | null;
  uriOf(src: Source, rel: string): vscode.Uri | null;
  openFile(src: Source, rel: string, at?: { line?: number }): Promise<void>;
}

let host: PreviewHost | null = null;

const PANE = 'escouade.preview';
export const editorIdOf = (kind: PreviewKind) => `${PANE}.${kind}`;

// ---------- the input and the pane ----------

/** A file shown in its preview: its URI, its family. Read only: nothing is ever written from a preview. */
class PreviewInput extends SimpleEditorInput {
  constructor(
    resource: vscode.Uri,
    readonly kind: PreviewKind,
  ) {
    super(resource as never);
    const name = resource.path.split('/').pop() ?? '';
    // The Markdown and the table are a view of a text the user may have open beside: their tab says so.
    this.setName(kind === 'markdown' || kind === 'csv' ? t('editor.preview.titleOf', { name }) : name);
    this.setTitle(resource.path);
    this.addCapability(EditorInputCapabilities.Readonly);
  }
  override get typeId(): string {
    return PANE;
  }
  override get editorId(): string {
    return editorIdOf(this.kind);
  }
  override matches(other: unknown): boolean {
    return (
      other === this ||
      (other instanceof PreviewInput && other.kind === this.kind && other.resource?.toString() === this.resource?.toString())
    );
  }
}

/** A preview drawn in the pane: what it lets go of, and how it is brought to a line (a link of a conversation, Ctrl+P). */
interface Shown {
  dispose(): void;
  reveal?(line: number): void;
}

/** The line an opening asks for (1-based), from its options (`vscode.open` with a selection). */
function lineAsked(options: unknown): number | undefined {
  const sel = (options as { selection?: { startLineNumber?: number } } | undefined)?.selection;
  return sel?.startLineNumber;
}

class PreviewPane extends EditorPane {
  private root!: HTMLElement;
  private shown: Shown | null = null;
  private generation = 0;

  constructor(group: never) {
    super(PANE, group);
  }

  protected override createEditor(parent: HTMLElement): void {
    this.root = el('div', { class: 'escouade-preview', tabindex: '-1' });
    parent.appendChild(this.root);
  }

  override async setInput(input: never, options: never, context: never, token: { isCancellationRequested: boolean }) {
    await super.setInput(input, options, context, token as never);
    if (token.isCancellationRequested) return;
    const mine = ++this.generation;
    this.drop();
    const pi = input as unknown as PreviewInput;
    this.root.dataset.kind = pi.kind;
    const shown = await render(pi, this.root, () => mine !== this.generation || token.isCancellationRequested);
    if (mine !== this.generation) {
      shown.dispose();
      return;
    }
    this.shown = shown;
    const line = lineAsked(options);
    if (line) shown.reveal?.(line);
  }

  override setOptions(options: never): void {
    super.setOptions(options);
    const line = lineAsked(options);
    if (line) this.shown?.reveal?.(line);
  }

  override clearInput(): void {
    this.generation++;
    this.drop();
    super.clearInput();
  }

  private drop() {
    this.shown?.dispose();
    this.shown = null;
    this.root.replaceChildren();
  }

  override layout(dimension: { width: number; height: number }): void {
    this.root.style.width = `${dimension.width}px`;
    this.root.style.height = `${dimension.height}px`;
  }

  override focus(): void {
    super.focus();
    (this.root.querySelector<HTMLElement>('[data-focus]') ?? this.root).focus();
  }

  override dispose(): void {
    this.generation++;
    this.shown?.dispose();
    super.dispose();
  }
}

// ---------- registration ----------

let registered = false;

/** The previews registered in the workbench, once it is initialized. */
export async function registerPreviews(h: PreviewHost) {
  host = h;
  if (registered) return;
  registered = true;
  registerEditorPane(PANE, t('editor.preview.what'), PreviewPane as never, [PreviewInput]);
  for (const kind of KINDS) {
    registerEditor(
      globOf(kind),
      {
        id: editorIdOf(kind),
        label: t(`editor.preview.label.${kind}`),
        priority: {
          editor: opensInPreview(kind) ? RegisteredEditorPriority.default : RegisteredEditorPriority.option,
          // A diff stays the text diff's: the preview draws one file.
          diff: RegisteredEditorPriority.option,
          merge: RegisteredEditorPriority.option,
        },
      },
      { singlePerResource: true, canSupportResource: (uri) => uri.scheme === 'file' && !!host?.sourceOf(uri as never) },
      {
        createEditorInput: ({ resource, options }) => ({
          editor: new PreviewInput(resource as never, kind) as never,
          options,
        }),
      },
    );
  }
  await registerCommands();
}

/**
 * The Markdown and table commands, with VS Code's ids and names (in Escouade's language, as the view: ESC-472), their keys
 * and their button in the editor's title.
 */
async function registerCommands() {
  const ext = registerExtension(
    {
      name: 'escouade-previews',
      publisher: 'escouade',
      version: '1.0.0',
      engines: { vscode: '*' },
      contributes: {
        commands: [
          {
            command: 'markdown.showPreview',
            title: t('editor.preview.command.showPreview'),
            category: 'Markdown',
            icon: '$(open-preview)',
          },
          {
            command: 'markdown.showPreviewToSide',
            title: t('editor.preview.command.showPreviewToSide'),
            category: 'Markdown',
            icon: '$(open-preview)',
          },
          { command: 'escouade.showTable', title: t('editor.preview.command.showTable'), category: 'CSV', icon: '$(table)' },
        ],
        menus: {
          'editor/title': [
            { command: 'markdown.showPreviewToSide', when: 'editorLangId == markdown', group: 'navigation' },
            { command: 'escouade.showTable', when: 'resourceExtname =~ /^\\.(csv|tsv)$/i', group: 'navigation' },
          ],
        },
        keybindings: [
          { command: 'markdown.showPreview', key: 'ctrl+shift+v', mac: 'cmd+shift+v', when: 'editorLangId == markdown' },
          { command: 'markdown.showPreviewToSide', key: 'ctrl+k v', mac: 'cmd+k v', when: 'editorLangId == markdown' },
        ],
      },
    },
    ExtensionHostKind.LocalProcess,
  );
  await ext.whenReady();
  const target = (uri?: vscode.Uri) => uri ?? vscode.window.activeTextEditor?.document.uri;
  const open = (kind: PreviewKind, column: vscode.ViewColumn, preserveFocus: boolean) => (uri?: vscode.Uri) => {
    const at = target(uri);
    if (at) void vscode.commands.executeCommand('vscode.openWith', at, editorIdOf(kind), { viewColumn: column, preserveFocus });
  };
  vscode.commands.registerCommand('markdown.showPreview', open('markdown', vscode.ViewColumn.Active, false));
  vscode.commands.registerCommand('markdown.showPreviewToSide', open('markdown', vscode.ViewColumn.Beside, true));
  vscode.commands.registerCommand('escouade.showTable', open('csv', vscode.ViewColumn.Active, false));
}

/** The preview tabs of the files `mine` says, closed (a closed project's). */
export async function closePreviews(mine: (uri: vscode.Uri) => boolean) {
  const editors = await getService(IEditorService);
  const those = editors
    .getEditors(0 /* EditorsOrder.MOST_RECENTLY_ACTIVE */)
    .filter((e) => e.editor instanceof PreviewInput && !!e.editor.resource && mine(e.editor.resource as never));
  if (those.length) await editors.closeEditors(those);
}

// ---------- drawing ----------

function el<K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Record<string, string> = {}, ...kids: (Node | string)[]) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  e.append(...kids);
  return e;
}

function button(label: string, onclick: () => void, attrs: Record<string, string> = {}) {
  const b = el('button', { type: 'button', class: 'pv-btn', ...attrs }, label);
  b.addEventListener('click', onclick);
  return b;
}

/** A sentence in place of the preview (a file too big, broken, a format without preview). */
function message(root: HTMLElement, text: string, ...more: Node[]): Shown {
  root.replaceChildren(el('div', { class: 'pv-message', role: 'status' }, el('p', {}, text), ...more));
  return { dispose() {} };
}

const nothing: Shown = { dispose() {} };

/** The bytes of a file of the view, read by the core; null (and the pane says why) when too big. */
async function bytesOf(uri: vscode.Uri, root: HTMLElement): Promise<Uint8Array | null> {
  const stat = await vscode.workspace.fs.stat(uri);
  if (stat.size > MAX_PREVIEW_BYTES) {
    message(root, t('editor.preview.tooBig', { size: fSize(stat.size), max: fSize(MAX_PREVIEW_BYTES) }));
    return null;
  }
  return vscode.workspace.fs.readFile(uri);
}

async function render(input: PreviewInput, root: HTMLElement, stale: () => boolean): Promise<Shown> {
  const uri = input.resource as unknown as vscode.Uri;
  root.replaceChildren(el('div', { class: 'pv-message pv-loading' }, t('editor.preview.loading')));
  try {
    switch (input.kind) {
      case 'image':
        return await image(uri, root, stale);
      case 'audio':
      case 'video':
        return await media(uri, root, input.kind, stale);
      case 'markdown':
        return await markdown(uri, root, stale);
      case 'csv':
        return await table(uri, root, stale);
      case 'pdf': {
        const bytes = await bytesOf(uri, root);
        if (!bytes || stale()) return nothing;
        const { showPdf } = await import('./pdf');
        return await showPdf(bytes, root, stale);
      }
      case 'docx': {
        const bytes = await bytesOf(uri, root);
        if (!bytes || stale()) return nothing;
        const { docxHtml } = await import('./docx');
        const html = await docxHtml(bytes);
        if (stale()) return nothing;
        return document_(uri, root, html, 'docx');
      }
      case 'none':
        return noPreview(uri, root);
    }
  } catch (e) {
    return message(root, t('editor.preview.failed', { error: e instanceof Error ? e.message : String(e) }));
  }
}

/** A Blob URL of `bytes`, of the MIME type of `path`'s format. */
const blobOf = (bytes: Uint8Array, mime: string) => URL.createObjectURL(new Blob([bytes as BlobPart], { type: mime }));

// ----- an image -----

async function image(uri: vscode.Uri, root: HTMLElement, stale: () => boolean): Promise<Shown> {
  const bytes = await bytesOf(uri, root);
  if (!bytes || stale()) return nothing;
  const url = blobOf(bytes, previewOf(uri.path)?.mime ?? 'application/octet-stream');
  const img = el('img', { class: 'pv-image', alt: uri.path.split('/').pop() ?? '', draggable: 'false' });
  const stage = el('div', { class: 'pv-stage', 'data-focus': '', tabindex: '0' }, img);
  const info = el('span', { class: 'pv-info' });
  const zoomLabel = el('span', { class: 'pv-zoom', 'aria-live': 'polite' });
  let scale: number | 'fit' = 'fit';
  const STEPS = [0.1, 0.25, 0.33, 0.5, 0.67, 0.75, 1, 1.25, 1.5, 2, 3, 4, 6, 8, 12, 16];
  const fitScale = () => {
    const w = img.naturalWidth || 1;
    const h = img.naturalHeight || 1;
    return Math.min(1, (stage.clientWidth - 32) / w, (stage.clientHeight - 32) / h);
  };
  const apply = () => {
    const s = scale === 'fit' ? fitScale() : scale;
    img.style.width = `${Math.max(1, Math.round((img.naturalWidth || 0) * s))}px`;
    img.style.height = `${Math.max(1, Math.round((img.naturalHeight || 0) * s))}px`;
    stage.classList.toggle('fit', scale === 'fit');
    zoomLabel.textContent = `${Math.round(s * 100)} %`;
  };
  const zoom = (dir: 1 | -1) => {
    const cur = scale === 'fit' ? fitScale() : scale;
    const next = dir > 0 ? STEPS.find((s) => s > cur + 1e-6) : [...STEPS].reverse().find((s) => s < cur - 1e-6);
    scale = next ?? cur;
    apply();
  };
  const bar = el(
    'div',
    { class: 'pv-bar' },
    info,
    el('span', { class: 'pv-gap' }),
    button('−', () => zoom(-1), { 'aria-label': t('editor.preview.zoomOut'), title: t('editor.preview.zoomOut') }),
    zoomLabel,
    button('+', () => zoom(1), { 'aria-label': t('editor.preview.zoomIn'), title: t('editor.preview.zoomIn') }),
    button(t('editor.preview.fit'), () => ((scale = 'fit'), apply())),
    button(t('editor.preview.actual'), () => ((scale = 1), apply())),
  );
  root.replaceChildren(bar, stage);
  stage.addEventListener(
    'wheel',
    (e) => {
      if (!e.ctrlKey && !e.metaKey) return;
      e.preventDefault();
      zoom(e.deltaY < 0 ? 1 : -1);
    },
    { passive: false },
  );
  stage.addEventListener('keydown', (e) => {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.key === '+' || e.key === '=') zoom(1);
    else if (e.key === '-') zoom(-1);
    else if (e.key === '0') ((scale = 'fit'), apply());
    else return;
    e.preventDefault();
  });
  const resize = new ResizeObserver(() => scale === 'fit' && apply());
  await new Promise<void>((done) => {
    img.onload = () => done();
    img.onerror = () => done();
    img.src = url;
  });
  if (!img.naturalWidth) {
    URL.revokeObjectURL(url);
    return message(root, t('editor.preview.imageBroken'));
  }
  info.textContent = `${t('editor.preview.imageSize', { width: img.naturalWidth, height: img.naturalHeight })} · ${fSize(bytes.length)}`;
  apply();
  resize.observe(stage);
  return {
    dispose() {
      resize.disconnect();
      URL.revokeObjectURL(url);
    },
  };
}

// ----- audio and video -----

async function media(uri: vscode.Uri, root: HTMLElement, kind: 'audio' | 'video', stale: () => boolean): Promise<Shown> {
  const bytes = await bytesOf(uri, root);
  if (!bytes || stale()) return nothing;
  const url = blobOf(bytes, previewOf(uri.path)?.mime ?? 'application/octet-stream');
  // Never played by itself; its controls the webview's.
  const player = el(kind, { class: `pv-${kind}`, controls: '', preload: 'metadata', 'data-focus': '' });
  player.addEventListener('error', () => message(root, t('editor.preview.mediaBroken')), { once: true });
  root.replaceChildren(
    el('div', { class: 'pv-bar' }, el('span', { class: 'pv-info' }, fSize(bytes.length))),
    el('div', { class: 'pv-stage fit' }, player),
  );
  player.src = url;
  return {
    dispose() {
      player.pause();
      player.removeAttribute('src');
      player.load();
      URL.revokeObjectURL(url);
    },
  };
}

// ----- a document's HTML: Markdown, Word -----

/** The images of a document shown: read by the core from the source, those of the web on a click only. */
class Images {
  private urls = new Map<string, string>();
  private remote = new Map<string, string>();
  constructor(
    private readonly uri: vscode.Uri,
    private readonly stale: () => boolean,
  ) {}

  fill(body: HTMLElement) {
    const at = host?.sourceOf(this.uri);
    for (const img of body.querySelectorAll<HTMLImageElement>('img[data-local-src]')) {
      const src = img.dataset.localSrc ?? '';
      const link = at ? linkOf(at.rel, src) : { to: 'none' as const };
      const kind = link.to === 'file' ? previewOf(link.rel) : null;
      if (link.to !== 'file' || kind?.kind !== 'image' || !at) {
        off(img, t('editor.preview.imageMissing', { src }));
        continue;
      }
      const known = this.urls.get(link.rel);
      if (known) {
        img.src = known;
        continue;
      }
      const target = host?.uriOf(at.source, link.rel);
      if (!target) {
        off(img, t('editor.preview.imageMissing', { src }));
        continue;
      }
      vscode.workspace.fs.readFile(target).then(
        (bytes) => {
          if (this.stale()) return;
          const url = this.urls.get(link.rel) ?? blobOf(bytes, kind.mime ?? 'application/octet-stream');
          this.urls.set(link.rel, url);
          img.src = url;
        },
        () => off(img, t('editor.preview.imageMissing', { src })),
      );
    }
    for (const img of body.querySelectorAll<HTMLImageElement>('img[data-remote-src]')) {
      const src = img.dataset.remoteSrc ?? '';
      const shown = this.remote.get(src);
      if (shown) {
        img.src = shown;
        continue;
      }
      let hostName = '';
      try {
        hostName = new URL(src).hostname;
      } catch {
        // Its address shown whole.
      }
      const show = el('button', { type: 'button', class: 'pv-btn', 'data-show-img': '' }, t('editor.preview.remoteShow'));
      const label = el(
        'span',
        { class: 'pv-img-off', 'data-remote': src, title: src },
        img.alt ? `${img.alt} · ` : '',
        t('editor.preview.remoteOf', { host: hostName || src }),
        ' ',
        show,
      );
      show.addEventListener('click', async (e) => {
        e.preventDefault();
        e.stopPropagation();
        show.disabled = true;
        // The core downloads it, now only (as the conversation's « Afficher l'image »): the window loads nothing from the web.
        const url = await api.remoteImage(src).catch(() => null);
        if (!url || this.stale()) {
          show.disabled = false;
          return;
        }
        this.remote.set(src, url);
        img.src = url;
        label.replaceWith(img);
      });
      img.replaceWith(label);
    }
    for (const img of body.querySelectorAll<HTMLImageElement>('img[data-img-off]'))
      off(img, t('editor.preview.imageOff', { src: img.dataset.imgOff ?? '' }));
  }

  dispose() {
    for (const url of this.urls.values()) URL.revokeObjectURL(url);
    this.urls.clear();
  }
}

/** An image not shown: its text and why, in its place. */
function off(img: HTMLImageElement, why: string) {
  img.replaceWith(el('span', { class: 'pv-img-off', title: why }, img.alt || why));
}

/** A click on a link of a document: a file of the source opened in the view, an anchor scrolled to; the web is `guardLinks`'. */
function followLinks(body: HTMLElement, uri: vscode.Uri) {
  body.addEventListener('click', (e) => {
    const a = (e.target as Element | null)?.closest?.('a[href]');
    if (!a) return;
    e.preventDefault();
    const at = host?.sourceOf(uri);
    if (!at) return;
    const link = linkOf(at.rel, a.getAttribute('href') ?? '');
    if (link.to === 'anchor') {
      const id = link.anchor.toLowerCase();
      const target =
        body.querySelector(`[id="user-content-${CSS.escape(link.anchor)}"]`) ?? body.querySelector(`[id="user-content-${CSS.escape(id)}"]`);
      target?.scrollIntoView({ block: 'start' });
    } else if (link.to === 'file') void host?.openFile(at.source, link.rel, { line: link.line });
  });
}

function document_(uri: vscode.Uri, root: HTMLElement, html: string, kind: 'docx'): Shown {
  const body = el('div', { class: `pv-doc pv-${kind}` });
  body.innerHTML = html;
  const images = new Images(uri, () => !body.isConnected);
  images.fill(body);
  followLinks(body, uri);
  root.replaceChildren(
    el('div', { class: 'pv-bar' }, el('span', { class: 'pv-info' }, t('editor.preview.readOnly'))),
    el('div', { class: 'pv-scroll', 'data-focus': '', tabindex: '0' }, el('div', { class: 'pv-page' }, body)),
  );
  return { dispose: () => images.dispose() };
}

// ----- Markdown -----

async function markdown(uri: vscode.Uri, root: HTMLElement, stale: () => boolean): Promise<Shown> {
  const doc = await vscode.workspace.openTextDocument(uri);
  if (stale()) return nothing;
  const body = el('div', { class: 'pv-doc pv-markdown' });
  const scroller = el('div', { class: 'pv-scroll', 'data-focus': '', tabindex: '0' }, body);
  const images = new Images(uri, () => !body.isConnected);
  const draw = (text: string) => {
    body.innerHTML = renderPreview(text);
    images.fill(body);
    void highlightWithin(body);
  };
  const blocks = () =>
    [...body.querySelectorAll<HTMLElement>('[data-line]')].map((b) => ({ line: Number(b.dataset.line), top: b.offsetTop }));
  let following = false;
  const toLine = (line: number) => {
    following = true;
    scroller.scrollTop = scrollFor(line, blocks());
    requestAnimationFrame(() => (following = false));
  };
  const same = (d: vscode.TextDocument) => d.uri.toString() === uri.toString();
  const source = () => vscode.window.visibleTextEditors.find((e) => same(e.document));
  root.replaceChildren(
    el(
      'div',
      { class: 'pv-bar' },
      el('span', { class: 'pv-info' }, t('editor.preview.markdownOf', { name: uri.path.split('/').pop() ?? '' })),
      el('span', { class: 'pv-gap' }),
      button(t('editor.preview.source'), () => void vscode.commands.executeCommand('vscode.openWith', uri, 'default')),
      button(
        t('editor.preview.sourceBeside'),
        () => void vscode.commands.executeCommand('vscode.openWith', uri, 'default', { viewColumn: vscode.ViewColumn.Beside }),
      ),
    ),
    scroller,
  );
  draw(doc.getText());
  followLinks(body, uri);
  let timer = 0;
  const subs = [
    // Typing in the source: drawn again (once a frame at most), its scroll kept.
    vscode.workspace.onDidChangeTextDocument((e) => {
      if (!same(e.document) || timer) return;
      timer = window.setTimeout(() => {
        timer = 0;
        const top = scroller.scrollTop;
        draw(e.document.getText());
        scroller.scrollTop = top;
      }, 60);
    }),
    // The source scrolled: the preview follows it.
    vscode.window.onDidChangeTextEditorVisibleRanges((e) => {
      if (!same(e.textEditor.document) || !e.visibleRanges[0]) return;
      toLine(e.visibleRanges[0].start.line);
    }),
  ];
  // The preview scrolled by hand: the source beside follows it, when it is shown.
  const onScroll = () => {
    if (following) return;
    const editor = source();
    if (!editor) return;
    const line = Math.floor(lineFor(scroller.scrollTop, blocks()));
    editor.revealRange(new vscode.Range(line, 0, line, 0), vscode.TextEditorRevealType.AtTop);
  };
  scroller.addEventListener('scroll', onScroll, { passive: true });
  const beside = source();
  if (beside?.visibleRanges[0]) requestAnimationFrame(() => toLine(beside.visibleRanges[0].start.line));
  return {
    dispose() {
      window.clearTimeout(timer);
      for (const s of subs) s.dispose();
      images.dispose();
    },
    reveal: (line) => requestAnimationFrame(() => toLine(line - 1)),
  };
}

// ----- a table -----

async function table(uri: vscode.Uri, root: HTMLElement, stale: () => boolean): Promise<Shown> {
  const doc = await vscode.workspace.openTextDocument(uri);
  if (stale()) return nothing;
  const scroller = el('div', { class: 'pv-scroll', 'data-focus': '', tabindex: '0' });
  const info = el('span', { class: 'pv-info' });
  const draw = (text: string) => {
    const { rows, more } = parseTable(text, separatorOf(text, extOf(uri.path) === 'tsv'));
    const [head, ...rest] = rows;
    const tableEl = el('table', { class: 'pv-table' });
    if (head) tableEl.append(el('thead', {}, el('tr', {}, ...head.map((c) => el('th', {}, c)))));
    tableEl.append(el('tbody', {}, ...rest.map((r) => el('tr', {}, ...r.map((c) => el('td', {}, c))))));
    scroller.replaceChildren(tableEl);
    if (more) scroller.append(el('p', { class: 'pv-more' }, t('editor.preview.moreRows', { count: more })));
    info.textContent = t('editor.preview.rows', { count: Math.max(0, rows.length - 1) + more });
  };
  root.replaceChildren(
    el(
      'div',
      { class: 'pv-bar' },
      info,
      el('span', { class: 'pv-gap' }),
      button(t('editor.preview.source'), () => void vscode.commands.executeCommand('vscode.openWith', uri, 'default')),
    ),
    scroller,
  );
  draw(doc.getText());
  const sub = vscode.workspace.onDidChangeTextDocument((e) => {
    if (e.document.uri.toString() === uri.toString()) draw(e.document.getText());
  });
  return { dispose: () => sub.dispose() };
}

// ----- no preview -----

function noPreview(uri: vscode.Uri, root: HTMLElement): Shown {
  const ext = extOf(uri.path);
  const why = NO_PREVIEW[ext] ?? 'office';
  root.replaceChildren(
    el(
      'div',
      { class: 'pv-message', role: 'status' },
      el('p', { class: 'pv-title' }, t('editor.preview.none.title', { ext })),
      el('p', {}, t(`editor.preview.none.${why}`)),
      button(t('editor.preview.none.openAsText'), () => void vscode.commands.executeCommand('vscode.openWith', uri, 'default'), {
        'data-focus': '',
      }),
    ),
  );
  return nothing;
}
