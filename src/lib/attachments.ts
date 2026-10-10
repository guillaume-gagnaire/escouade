// Files attached to a message: images, PDFs and text files, as Claude reads them.

import { fBytes } from './format';
import { t } from './i18n';
import type { Attachment } from './types';

export type AttachmentKind = 'image' | 'pdf' | 'text';

/** An attachment waiting in the composer; images keep a preview. */
export interface DraftAttachment extends Attachment {
  kind: AttachmentKind;
  /** Of the file, in bytes. */
  size: number;
  url?: string;
}

/** The only image formats the API reads. */
const IMAGE_EXTENSIONS = new Map([
  ['png', 'image/png'],
  ['jpg', 'image/jpeg'],
  ['jpeg', 'image/jpeg'],
  ['gif', 'image/gif'],
  ['webp', 'image/webp'],
]);
const IMAGE_TYPES = new Set(IMAGE_EXTENSIONS.values());

// Extensions (or whole names, for files without one) read as text: Windows often gives such
// files no type, or a wrong one (.ts is "video/mp2t").
const TEXT_EXTENSIONS = new Set(
  (
    'txt text md markdown mdx rst adoc org tex bib log csv tsv json jsonc json5 jsonl ndjson ' +
    'yaml yml toml ini cfg conf config properties env xml xsd xsl html htm xhtml svg css scss sass less ' +
    'js mjs cjs jsx ts mts cts tsx vue svelte astro py pyi rb php java kt kts scala groovy gradle go rs ' +
    'c h cc cpp cxx hpp hh cs fs fsx vb swift m mm dart lua pl pm r jl ex exs erl hs clj elm zig nim ' +
    'sql graphql gql proto prisma tf hcl http sh bash zsh fish ps1 psm1 psd1 bat cmd diff patch ' +
    'lock gitignore gitattributes editorconfig dockerfile makefile license readme srt vtt'
  ).split(' '),
);
const TEXT_TYPES = /^(text\/|application\/([\w.-]+\+)?(json|xml|yaml|x-yaml|toml|sql|javascript|x-sh)$)/;

const KB = 1024;
const MB = 1024 * KB;
// A PDF fits in what a whole message may carry; more text would fill Claude's context.
const LIMITS: Record<AttachmentKind, number> = { image: 5 * MB, pdf: 18 * MB, text: 256 * KB };
/** All the files of a message: sent in base64 (4/3 bigger), under the API's 32 MB a request. */
export const MAX_TOTAL = 18 * MB;

/** A size, rounded down: « 5 Mo », « 256 Ko ». */
export function sizeLabel(bytes: number): string {
  return bytes >= MB ? fBytes(Math.floor(bytes / MB) * MB) : t('composer.attachments.kilobytes', { n: Math.floor(bytes / KB) });
}

/** For the file dialog's filter. */
export const ACCEPT = [...IMAGE_TYPES, 'application/pdf', '.pdf', 'text/*', ...[...TEXT_EXTENSIONS].map((e) => '.' + e)].join(',');

function extension(name: string): string {
  const n = name.toLowerCase();
  return n.slice(n.lastIndexOf('.') + 1);
}

export function attachmentKind(f: File): AttachmentKind | null {
  const ext = extension(f.name);
  if (IMAGE_TYPES.has(f.type) || IMAGE_EXTENSIONS.has(ext)) return 'image';
  if (f.type === 'application/pdf' || ext === 'pdf') return 'pdf';
  if (TEXT_EXTENSIONS.has(ext) || TEXT_TYPES.test(f.type)) return 'text';
  return null;
}

function read(f: File, as: 'dataURL'): Promise<string>;
function read(f: File, as: 'arrayBuffer'): Promise<ArrayBuffer>;
function read(f: File, as: 'dataURL' | 'arrayBuffer'): Promise<string | ArrayBuffer> {
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onload = () => resolve(r.result!);
    r.onerror = () => reject(new Error(t('composer.attachments.unreadable', { name: f.name })));
    if (as === 'dataURL') r.readAsDataURL(f);
    else r.readAsArrayBuffer(f);
  });
}

/**
 * The text of a file: UTF-8, UTF-16 with its BOM (PowerShell 5), else with `legacy` Windows-1252
 * (Excel, older tools). Null for binary content.
 */
function decodeText(buf: ArrayBuffer, legacy: boolean): string | null {
  const b = new Uint8Array(buf);
  if (b[0] === 0xff && b[1] === 0xfe) return new TextDecoder('utf-16le').decode(b.subarray(2));
  if (b[0] === 0xfe && b[1] === 0xff) return new TextDecoder('utf-16be').decode(b.subarray(2));
  if (b.includes(0)) return null;
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(b);
  } catch {
    return legacy ? new TextDecoder('windows-1252').decode(b) : null;
  }
}

/** Reads a file to attach; rejects with the reason to show when it cannot be. */
export async function readAttachment(f: File): Promise<DraftAttachment> {
  const kind = attachmentKind(f);
  const unsupported = new Error(t('composer.attachments.unsupported', { name: f.name }));
  // Of no known kind and given no type by Windows (.env.local, Jenkinsfile…), it may be text.
  const maybeText = !kind && (!f.type || f.type === 'application/octet-stream') && f.size <= LIMITS.text;
  if (!kind && !maybeText) throw unsupported;
  const max = LIMITS[kind ?? 'text'];
  if (f.size > max) throw new Error(t('composer.attachments.tooBig', { name: f.name, max: sizeLabel(max) }));
  if (kind === 'text' || maybeText) {
    const data = decodeText(await read(f, 'arrayBuffer'), kind === 'text');
    if (data === null) {
      if (maybeText) throw unsupported;
      throw new Error(t('composer.attachments.notText', { name: f.name }));
    }
    return { kind: 'text', name: f.name, mediaType: 'text/plain', data, size: f.size };
  }
  const url = await read(f, 'dataURL');
  const data = url.slice(url.indexOf(',') + 1);
  if (kind === 'pdf') return { kind, name: f.name, mediaType: 'application/pdf', data, size: f.size };
  const mediaType = IMAGE_TYPES.has(f.type) ? f.type : IMAGE_EXTENSIONS.get(extension(f.name))!;
  return { kind: 'image', name: f.name || 'image', mediaType, data, size: f.size, url };
}

/** A file dropped outside a drop zone would make the WebView open it in place of the app. */
export function guardFileDrops(target: Window): () => void {
  const files = (e: DragEvent) => !!e.dataTransfer?.types.includes('Files');
  const over = (e: DragEvent) => {
    if (e.defaultPrevented || !files(e)) return;
    e.preventDefault();
    e.dataTransfer!.dropEffect = 'none';
  };
  const drop = (e: DragEvent) => {
    if (files(e)) e.preventDefault();
  };
  target.addEventListener('dragover', over);
  target.addEventListener('drop', drop);
  return () => {
    target.removeEventListener('dragover', over);
    target.removeEventListener('drop', drop);
  };
}
