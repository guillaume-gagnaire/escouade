// A PDF read in place (the box « À toi », ESC-510), by pdf.js (Apache-2.0), loaded when the first PDF opens: a page drawn
// on a canvas at the width asked, its text read to look for a word. The same rules as the editor's preview of the 2.3
// (release/2.3, `src/lib/vscode/previews/pdf.ts`), out of any view:
//
// - No script of the PDF runs: only pdf.js's core is used (its viewer and its scripting sandbox are not), pdf.js 6 has no
//   `eval` left (and the window's policy has no `unsafe-eval`), no XFA form. Its forms are drawn as they are, never
//   filled; its links are not live (a page is a picture).
// - Nothing is loaded from the web: the bytes are the core's copy of the file; pdf.js's worker and its WebAssembly
//   decoders (JBIG2, JPEG 2000, colours) are files of the app, read from it (`AppBinaryData`). The window's policy is
//   left as it is: the worker is a script of the app's own origin.
// - Its fonts that are not embedded are the system's (`useSystemFonts`): pdf.js's standard fonts and CMaps are not
//   shipped (1,2 Mo), a PDF in Chinese or Japanese without its fonts may show its glyphs wrong.

import * as pdfjs from 'pdfjs-dist';
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url';
import jbig2 from 'pdfjs-dist/wasm/jbig2.wasm?url';
import openjpeg from 'pdfjs-dist/wasm/openjpeg.wasm?url';
import qcms from 'pdfjs-dist/wasm/qcms_bg.wasm?url';

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

/** A PDF open: its pages drawn one at a time. */
export interface PdfDoc {
  pages: number;
  /** The page `n` (from 1) drawn on `canvas`, `width` CSS pixels wide (sharp on a high-density screen). */
  draw(n: number, canvas: HTMLCanvasElement, width: number): Promise<void>;
  /** The pages (from 1) whose text holds `query`, case and accents aside. */
  find(query: string): Promise<number[]>;
  dispose(): void;
}

/** Why a PDF does not show: a password. */
export class PdfPasswordError extends Error {
  constructor() {
    super('password');
    this.name = 'PdfPasswordError';
  }
}

/** Folded for a search: lower case, without accents. */
export const folded = (s: string) => s.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();

/** The PDF of `bytes`, or `PdfPasswordError`, or pdf.js's error for a file it cannot read. */
export async function openPdf(bytes: Uint8Array): Promise<PdfDoc> {
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
  } as Parameters<typeof pdfjs.getDocument>[0]);
  let doc: pdfjs.PDFDocumentProxy;
  try {
    doc = await task.promise;
  } catch (e) {
    await task.destroy();
    if ((e as { name?: string }).name === 'PasswordException') throw new PdfPasswordError();
    throw e;
  }
  const texts = new Map<number, string>();
  // The drawing under way on each canvas: a new one on the same canvas cancels it, never another canvas's.
  const drawing = new Map<HTMLCanvasElement, { cancel(): void }>();
  return {
    pages: doc.numPages,
    async draw(n, canvas, width) {
      drawing.get(canvas)?.cancel();
      const page = await doc.getPage(n);
      const base = page.getViewport({ scale: 1 });
      const ratio = Math.min(3, window.devicePixelRatio || 1);
      const viewport = page.getViewport({ scale: (width / base.width) * ratio });
      canvas.width = Math.floor(viewport.width);
      canvas.height = Math.floor(viewport.height);
      canvas.style.width = `${Math.floor(viewport.width / ratio)}px`;
      canvas.style.aspectRatio = `${viewport.width} / ${viewport.height}`;
      const run = page.render({
        canvas,
        viewport,
        // The forms drawn as they are, never filled: a preview reads.
        annotationMode: pdfjs.AnnotationMode.ENABLE,
      } as Parameters<typeof page.render>[0]);
      drawing.set(canvas, run);
      try {
        await run.promise;
      } catch (e) {
        if ((e as { name?: string }).name !== 'RenderingCancelledException') throw e;
      } finally {
        if (drawing.get(canvas) === run) drawing.delete(canvas);
      }
    },
    async find(query) {
      const q = folded(query.trim());
      if (!q) return [];
      const hits: number[] = [];
      for (let n = 1; n <= doc.numPages; n++) {
        let text = texts.get(n);
        if (text === undefined) {
          const content = await (await doc.getPage(n)).getTextContent();
          text = folded(content.items.map((i) => ('str' in i ? i.str : '')).join(' '));
          texts.set(n, text);
        }
        if (text.includes(q)) hits.push(n);
      }
      return hits;
    },
    dispose() {
      for (const run of drawing.values()) run.cancel();
      drawing.clear();
      void task.destroy();
    },
  };
}
