// A PDF read in a tab of the VS Code view (ESC-473), by pdf.js (Apache-2.0), loaded when the first PDF opens: its
// pages drawn as they come into view, the zoom, the search (Ctrl+F in the preview, Entrée / Maj+Entrée), the text
// selected and copied.
//
// - No script of the PDF runs: pdf.js's scripting is off (its sandbox is not even shipped), pdf.js 6 has no `eval` left
//   (and the window's policy has no `unsafe-eval`), no XFA form.
// - Nothing is loaded from the web: the bytes are the core's; the PDF's links of the web open by Escouade's link opening
//   (`guardLinks`), on a click only. Its decoders of images (JBIG2, JPEG 2000) and of colours are pdf.js's WebAssembly,
//   shipped with the app and read from it (`BinaryDataFactory`).
// - Its fonts that are not embedded are the system's (`useSystemFonts`): pdf.js's standard fonts and CMaps are not
//   shipped (1,2 Mo), a PDF in Chinese or Japanese without its fonts may show its glyphs wrong.

import * as pdfjs from 'pdfjs-dist';
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url';
import jbig2 from 'pdfjs-dist/wasm/jbig2.wasm?url';
import openjpeg from 'pdfjs-dist/wasm/openjpeg.wasm?url';
import qcms from 'pdfjs-dist/wasm/qcms_bg.wasm?url';
import 'pdfjs-dist/web/pdf_viewer.css';

import { t } from '../../i18n';

/** The viewer's components read the library from the global (`web/pdf_viewer.mjs`): set before it is loaded. */
(globalThis as { pdfjsLib?: unknown }).pdfjsLib = pdfjs;
const viewerLib = import('pdfjs-dist/web/pdf_viewer.mjs');

pdfjs.GlobalWorkerOptions.workerSrc = new URL(workerUrl, location.href).href;

const WASM: Record<string, string> = { 'jbig2.wasm': jbig2, 'openjpeg.wasm': openjpeg, 'qcms_bg.wasm': qcms };

/** pdf.js's binary data, read from the app's own files only: its WebAssembly; no CMap, no standard font. */
class AppBinaryData {
  async fetch({ kind, filename }: { kind: string; filename: string }): Promise<Uint8Array> {
    const url = kind === 'wasmUrl' ? WASM[filename] : undefined;
    if (!url) throw new Error(`no ${kind} ${filename}`);
    const r = await fetch(new URL(url, location.href));
    if (!r.ok) throw new Error(`${filename}: ${r.status}`);
    return new Uint8Array(await r.arrayBuffer());
  }
}

/** pdf.js's texts (its pages' labels for a screen reader), none: Escouade's bar says what is needed. */
const l10n = {
  getLanguage: () => document.documentElement.lang || 'fr',
  getDirection: () => 'ltr',
  get: async () => '',
  translate: async () => {},
  translateOnce: async () => {},
  destroy: async () => {},
  pause() {},
  resume() {},
};

const el = <K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Record<string, string> = {}, ...kids: (Node | string)[]) => {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  e.append(...kids);
  return e;
};

function button(label: string, title: string, onclick: () => void) {
  const b = el('button', { type: 'button', class: 'pv-btn', title, 'aria-label': title }, label);
  b.addEventListener('click', onclick);
  return b;
}

/** The PDF of `bytes` shown in `root`; what lets it go. */
export async function showPdf(bytes: Uint8Array, root: HTMLElement, stale: () => boolean) {
  const { EventBus, PDFViewer, PDFLinkService, PDFFindController, LinkTarget } = await viewerLib;
  const task = pdfjs.getDocument({
    // A copy: pdf.js gives the buffer to its worker.
    data: bytes.slice(),
    enableXfa: false,
    useSystemFonts: true,
    useWorkerFetch: false,
    // Only a mark for pdf.js (it wants an address ending in a slash): `AppBinaryData` reads the app's files.
    wasmUrl: new URL('/pdfjs/wasm/', location.href).href,
    BinaryDataFactory: AppBinaryData,
    stopAtErrors: false,
  });
  let doc: pdfjs.PDFDocumentProxy;
  try {
    doc = await task.promise;
  } catch (e) {
    await task.destroy();
    if ((e as { name?: string }).name === 'PasswordException') throw new Error(t('editor.preview.pdf.password'));
    throw e;
  }
  if (stale()) {
    await task.destroy();
    return { dispose() {} };
  }

  const eventBus = new EventBus();
  const linkService = new PDFLinkService({ eventBus, externalLinkTarget: LinkTarget.NONE, externalLinkRel: 'noopener noreferrer' });
  const findController = new PDFFindController({ eventBus, linkService });
  const viewerEl = el('div', { class: 'pdfViewer' });
  const container = el('div', { class: 'pv-pdf-scroll', 'data-focus': '', tabindex: '0' }, viewerEl);

  // The bar: the page, the zoom, the search.
  const pageInput = el('input', { class: 'pv-page-input', type: 'text', inputmode: 'numeric', 'aria-label': t('editor.preview.pdf.page') });
  const pages = el('span', { class: 'pv-info' }, t('editor.preview.pdf.of', { count: doc.numPages }));
  const zoomLabel = el('span', { class: 'pv-zoom', 'aria-live': 'polite' });
  const find = el('input', {
    class: 'pv-find',
    type: 'search',
    placeholder: t('editor.preview.pdf.find'),
    'aria-label': t('editor.preview.pdf.find'),
  });
  const found = el('span', { class: 'pv-found', 'aria-live': 'polite' });
  const zoomSel = el(
    'select',
    { class: 'pv-select', 'aria-label': t('editor.preview.pdf.zoom') },
    el('option', { value: 'page-width' }, t('editor.preview.pdf.fitWidth')),
    el('option', { value: 'page-fit' }, t('editor.preview.pdf.fitPage')),
    el('option', { value: '1' }, '100 %'),
  );
  const bar = el(
    'div',
    { class: 'pv-bar' },
    el('label', { class: 'pv-pages' }, t('editor.preview.pdf.page'), ' ', pageInput, ' ', pages),
    el('span', { class: 'pv-gap' }),
    button('−', t('editor.preview.zoomOut'), () => viewer.decreaseScale()),
    zoomLabel,
    button('+', t('editor.preview.zoomIn'), () => viewer.increaseScale()),
    zoomSel,
    el('span', { class: 'pv-sep' }),
    find,
    found,
    button('↑', t('editor.preview.pdf.findPrev'), () => search(true, 'again')),
    button('↓', t('editor.preview.pdf.findNext'), () => search(false, 'again')),
  );
  root.replaceChildren(bar, el('div', { class: 'pv-pdf' }, container));

  const viewer = new PDFViewer({
    container,
    viewer: viewerEl,
    eventBus,
    linkService,
    findController,
    l10n: l10n as never,
    textLayerMode: 1,
    // The forms drawn as they are, never filled: a preview reads.
    annotationMode: pdfjs.AnnotationMode.ENABLE,
    annotationEditorMode: pdfjs.AnnotationEditorType.DISABLE,
    enableAutoLinking: true,
    supportsPinchToZoom: true,
  });
  linkService.setViewer(viewer);
  viewer.setDocument(doc);
  linkService.setDocument(doc, null);

  eventBus.on('pagesinit', () => {
    viewer.currentScaleValue = 'page-width';
  });
  eventBus.on('pagechanging', ({ pageNumber }: { pageNumber: number }) => {
    pageInput.value = String(pageNumber);
  });
  eventBus.on('scalechanging', ({ scale, presetValue }: { scale: number; presetValue?: string }) => {
    zoomLabel.textContent = `${Math.round(scale * 100)} %`;
    zoomSel.value = presetValue && ['page-width', 'page-fit'].includes(presetValue) ? presetValue : Math.abs(scale - 1) < 1e-3 ? '1' : '';
  });
  pageInput.value = '1';
  pageInput.addEventListener('change', () => {
    const n = Math.round(Number(pageInput.value));
    if (n >= 1 && n <= doc.numPages) viewer.currentPageNumber = n;
    else pageInput.value = String(viewer.currentPageNumber);
  });
  zoomSel.addEventListener('change', () => {
    if (zoomSel.value) viewer.currentScaleValue = zoomSel.value;
  });

  const search = (previous: boolean, type: '' | 'again') => {
    eventBus.dispatch('find', {
      source: null,
      type,
      query: find.value,
      caseSensitive: false,
      entireWord: false,
      highlightAll: true,
      findPrevious: previous,
      matchDiacritics: false,
    });
  };
  const counted = ({ matchesCount }: { matchesCount?: { current: number; total: number } }) => {
    if (!find.value || !matchesCount) found.textContent = '';
    else if (!matchesCount.total) found.textContent = t('editor.preview.pdf.noMatch');
    else found.textContent = t('editor.preview.pdf.matchAt', { current: matchesCount.current, total: matchesCount.total });
  };
  eventBus.on('updatefindmatchescount', counted);
  eventBus.on('updatefindcontrolstate', counted);
  find.addEventListener('input', () => search(false, ''));
  find.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      search(e.shiftKey, 'again');
    } else if (e.key === 'Escape') {
      e.preventDefault();
      find.value = '';
      search(false, '');
      container.focus();
    }
    e.stopPropagation();
  });

  // Ctrl+F, the zoom keys: the preview's, not VS Code's (it would look for a text editor).
  const keys = (e: KeyboardEvent) => {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && !e.altKey && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      e.stopPropagation();
      find.focus();
      find.select();
    } else if (e.key === 'F3') {
      e.preventDefault();
      e.stopPropagation();
      search(e.shiftKey, 'again');
    }
  };
  root.addEventListener('keydown', keys);
  const wheel = (e: WheelEvent) => {
    if (!e.ctrlKey && !e.metaKey) return;
    e.preventDefault();
    if (e.deltaY < 0) viewer.increaseScale();
    else viewer.decreaseScale();
  };
  container.addEventListener('wheel', wheel, { passive: false });

  return {
    dispose() {
      root.removeEventListener('keydown', keys);
      viewer.cleanup();
      viewer.setDocument(null as never);
      linkService.setDocument(null, null);
      void task.destroy();
    },
  };
}
