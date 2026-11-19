// Files attached to a message: images, PDFs and text files go in the message, as Claude reads them; any other file, or
// one too big for the message, goes by its path (a copy the backend keeps, or the file itself).

import { onServer } from './connection.svelte';
import { local } from './transport';
import { fBytes } from './format';
import { t } from './i18n';
import { api } from './ipc';
import type { AttachedFile, Attachment, Settings } from './types';

/** What the message itself can carry of a file. */
export type MessageKind = 'image' | 'pdf' | 'text';
export type AttachmentKind = MessageKind | 'file';

/** An attachment waiting in the composer; images keep a preview. */
export interface DraftAttachment extends Attachment {
  kind: AttachmentKind;
  /** Of the file, in bytes. */
  size: number;
  url?: string;
  /** A file by its path (`kind: 'file'`): where it is once copied or found; its content is never in `data`. */
  file?: AttachedFile;
  /** A file by its path: big enough for the settings to warn of it. */
  warn?: boolean;
  /** A file by its path, being copied: `progress` is the part done (0 to 1), `cancel` stops it. */
  copying?: boolean;
  progress?: number;
  cancel?: () => void;
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
const LIMITS = { image: 5 * MB, pdf: 18 * MB, text: 256 * KB };
/** The most an image weighs to go in a message, or to be an image of a ticket. */
export const MAX_IMAGE = LIMITS.image;
/** All the files of a message: sent in base64 (4/3 bigger), under the API's 32 MB a request. */
export const MAX_TOTAL = 18 * MB;

/** A size, rounded down: « 5 Mo », « 256 Ko ». */
export function sizeLabel(bytes: number): string {
  return bytes >= MB ? fBytes(Math.floor(bytes / MB) * MB) : t('composer.attachments.kilobytes', { n: Math.floor(bytes / KB) });
}

/** The filter of a file dialog that takes images alone (a ticket's form does not use it any more: it takes every file). */
export const IMAGE_ACCEPT = [...IMAGE_TYPES].join(',');

function extension(name: string): string {
  const n = name.toLowerCase();
  return n.slice(n.lastIndexOf('.') + 1);
}

/** What a chip draws for a file, from its extension. */
export type FileCategory = 'archive' | 'sheet' | 'document' | 'video' | 'audio' | 'code' | 'generic';

const words = (list: string) => new Set(list.split(' '));
const ARCHIVES = words('zip rar 7z tar gz tgz bz2 xz zst lz cab iso dmg jar war apk');
const SHEETS = words('xlsx xls xlsm xlsb ods numbers csv tsv');
const DOCUMENTS = words('pdf doc docx odt rtf pages ppt pptx pps ppsx odp key epub txt text md markdown mdx rst adoc org tex log srt vtt');
const VIDEOS = words('mp4 m4v mov avi mkv webm wmv flv mpg mpeg ogv 3gp');
const SOUNDS = words('mp3 wav flac ogg oga m4a aac wma opus aiff mid midi');

export function fileCategory(name: string): FileCategory {
  const ext = extension(name);
  if (ARCHIVES.has(ext)) return 'archive';
  if (SHEETS.has(ext)) return 'sheet';
  if (DOCUMENTS.has(ext)) return 'document';
  if (VIDEOS.has(ext)) return 'video';
  if (SOUNDS.has(ext)) return 'audio';
  return TEXT_EXTENSIONS.has(ext) ? 'code' : 'generic';
}

/** The media type of an image the API reads, from its type or else its name; null for any other file. */
export function imageType(f: File): string | null {
  return IMAGE_TYPES.has(f.type) ? f.type : (IMAGE_EXTENSIONS.get(extension(f.name)) ?? null);
}

/** What the message can carry of a file, by its type or name; null for a file only its path can carry. */
export function attachmentKind(f: File): MessageKind | null {
  const ext = extension(f.name);
  if (imageType(f)) return 'image';
  if (f.type === 'application/pdf' || ext === 'pdf') return 'pdf';
  if (TEXT_EXTENSIONS.has(ext) || TEXT_TYPES.test(f.type)) return 'text';
  return null;
}

// `name` is the file's, for the one of a slice of it.
function read(b: Blob, as: 'dataURL', name?: string): Promise<string>;
function read(b: Blob, as: 'arrayBuffer', name?: string): Promise<ArrayBuffer>;
function read(b: Blob, as: 'dataURL' | 'arrayBuffer', name = (b as File).name): Promise<string | ArrayBuffer> {
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onload = () => resolve(r.result!);
    r.onerror = () => reject(new Error(t('composer.attachments.unreadable', { name })));
    if (as === 'dataURL') r.readAsDataURL(b);
    else r.readAsArrayBuffer(b);
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

/** Of no known kind and given no type by Windows (.env.local, Jenkinsfile…), a file may be text: reading it says. */
function maybeText(f: File): boolean {
  return !attachmentKind(f) && (!f.type || f.type === 'application/octet-stream') && f.size <= LIMITS.text;
}

/**
 * Reads a file as the message carries it: the text itself, the base64 of a PDF or an image (with its preview). Null for a
 * « text » that is none (binary content), which only its path can carry.
 */
export async function readForMessage(f: File, kind: MessageKind): Promise<DraftAttachment | null> {
  if (kind === 'text') {
    // The Windows-1252 reading is for the files known as text: any other would be taken for text by mistake.
    const data = decodeText(await read(f, 'arrayBuffer'), attachmentKind(f) === 'text');
    return data === null ? null : { kind, name: f.name, mediaType: 'text/plain', data, size: f.size };
  }
  const url = await read(f, 'dataURL');
  const data = url.slice(url.indexOf(',') + 1);
  if (kind === 'pdf') return { kind, name: f.name, mediaType: 'application/pdf', data, size: f.size };
  return { kind, name: f.name || 'image', mediaType: imageType(f)!, data, size: f.size, url };
}

/** Reads an image, a PDF or a text file for the message alone (a ticket's); rejects with the reason to show when it cannot be. */
export async function readAttachment(f: File): Promise<DraftAttachment> {
  const kind = attachmentKind(f);
  const unsupported = new Error(t('composer.attachments.unsupported', { name: f.name }));
  const maybe = maybeText(f);
  if (!kind && !maybe) throw unsupported;
  const max = LIMITS[kind ?? 'text'];
  if (f.size > max) throw new Error(t('composer.attachments.tooBig', { name: f.name, max: sizeLabel(max) }));
  const a = await readForMessage(f, kind ?? 'text');
  if (a) return a;
  if (maybe) throw unsupported;
  throw new Error(t('composer.attachments.notText', { name: f.name }));
}

/** What is done with a file: where it goes, whether the settings warn of its size, or why it is refused. */
export type Plan = { kind: AttachmentKind; warn: boolean } | { refusal: string };

export type AttachSettings = Pick<Settings, 'attachMaxMb' | 'attachWarnMb' | 'attachBigText'>;

/**
 * Where a file goes: in the message (as an image, a PDF or text) when Claude reads it, it is within the limit of its kind
 * and the message has room for it (`messageBytes` is what it carries already), else by its path. A text file over its
 * limit is refused when `attachBigText` says so. A text that turns out not to be one is still to be sent by its path
 * (`planByPath`).
 */
export function planAttachment(f: File, { settings, messageBytes }: { settings: AttachSettings; messageBytes: number }): Plan {
  const kind = attachmentKind(f);
  const inMessage = kind ?? (maybeText(f) ? 'text' : null);
  if (inMessage && f.size <= LIMITS[inMessage] && messageBytes + f.size <= MAX_TOTAL) return { kind: inMessage, warn: false };
  return planByPath(f, settings);
}

/**
 * A file sent by its path (wherever it is: the message, a ticket, the terminal): a text over what the message carries is
 * refused when the settings say so, any file over the maximum size is (none at 0), with a warning over the threshold (never
 * at 0).
 */
export function planByPath(f: File, settings: AttachSettings): Plan {
  if (attachmentKind(f) === 'text' && f.size > LIMITS.text && settings.attachBigText === 'refuse') {
    return { refusal: t('composer.attachments.tooBig', { name: f.name, max: sizeLabel(LIMITS.text) }) };
  }
  const max = settings.attachMaxMb * MB;
  if (max > 0 && f.size > max) return { refusal: t('composer.attachments.tooBig', { name: f.name, max: sizeLabel(max) }) };
  return { kind: 'file', warn: settings.attachWarnMb > 0 && f.size > settings.attachWarnMb * MB };
}

/** What the message already carries: the files sent by their path are not in it. */
export function inMessageBytes(files: { kind: AttachmentKind; size: number }[]): number {
  return files.reduce((n, f) => (f.kind === 'file' ? n : n + f.size), 0);
}

/** What a file is copied in: the most one request carries (the backend takes up to this much of the raw bytes). */
export const CHUNK = 4 * MB;

export interface CopyOptions {
  /** Cancelling it erases what was written. */
  signal?: AbortSignal;
  /** Bytes written so far. */
  onProgress?: (done: number) => void;
}

function stopped(signal?: AbortSignal) {
  if (signal?.aborted) throw new DOMException('Aborted', 'AbortError');
}

/**
 * A copy of the file that the backend keeps for `owner` (an agent's id, or « draft »), sent by chunks so a big file never
 * sits whole in memory. What is half written is erased when it fails or is cancelled.
 */
export async function copyToDisk(f: File, owner: string, { signal, onProgress }: CopyOptions = {}): Promise<AttachedFile> {
  const handle = await api.attachOpen(owner, f.name, f.size);
  let copy: AttachedFile;
  try {
    for (let at = 0; at < f.size; at += CHUNK) {
      stopped(signal);
      const bytes = new Uint8Array(await read(f.slice(at, at + CHUNK), 'arrayBuffer', f.name));
      stopped(signal);
      await api.attachChunk(handle, bytes);
      onProgress?.(Math.min(at + CHUNK, f.size));
    }
    stopped(signal);
    copy = await api.attachFinish(handle);
  } catch (e) {
    await api.attachCancel(handle).catch(() => {});
    throw e;
  }
  // Cancelled as it was finishing: the copy exists, nothing keeps it.
  if (signal?.aborted) {
    await api.attachDiscard(owner, copy.path).catch(() => {});
    stopped(signal);
  }
  return copy;
}

/**
 * A file attached by its path: where it is when the host knows it (`origin` is the id it holds for it, see `file-origin.ts`;
 * the backend finds the path from it, the page never gives one), nothing copied; else a copy.
 */
export async function attachByPath(
  f: File,
  owner: string,
  { origin, ...copy }: CopyOptions & { origin?: string | null } = {},
): Promise<AttachedFile> {
  stopped(copy.signal);
  return origin ? api.attachOrigin(owner, origin) : copyToDisk(f, owner, copy);
}

/** A file's content in base64, as the backend takes it. */
export async function base64(f: File): Promise<string> {
  const url = await read(f, 'dataURL');
  return url.slice(url.indexOf(',') + 1);
}

/**
 * The image of the system's clipboard, for a paste WebKitGTK (Linux) leaves without it (WebKit bug
 * 218519): the app reads it itself. Null without one, and always elsewhere (the WebView gives it).
 * One read at a time (ESC-239): a paste made while one goes on gets null, the image going to the first.
 */
export async function clipboardImage(): Promise<File | null> {
  if (readingClipboard) return null;
  readingClipboard = true;
  try {
    // On a server (ESC-538), the clipboard is this computer's: read here, then sent as any image pasted.
    const png = onServer() ? await local<ArrayBuffer>('clipboard_image') : await api.clipboardImage();
    return png.byteLength ? new File([png], 'image.png', { type: 'image/png' }) : null;
  } finally {
    readingClipboard = false;
  }
}

let readingClipboard = false;

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
