// What the VS Code view shows a file in (ESC-473): its preview, chosen by its extension, or the text editor. Pure of
// monaco, so that the tests run it alone; `panes.ts` registers each kind with the workbench.
//
// - `image`, `audio`, `video`, `pdf`, `docx`: opened in their preview, from the tree, a link or Ctrl+P.
// - `markdown`: in its preview too, its source beside on demand (« Ouvrir le source », the preview's own commands).
// - `csv`: the text editor first (a table is often edited as text), its table on demand (« Aperçu en tableau »).
// - `none`: a document format Escouade has no preview of (`NO_PREVIEW`): a pane says so, and why.
// - Any other file: VS Code's text editor, a binary one shown as VS Code shows it.

/** A family of preview. */
export type PreviewKind = 'image' | 'audio' | 'video' | 'pdf' | 'docx' | 'markdown' | 'csv' | 'none';

/** The MIME type of each format shown by the webview itself (an `<img>`, an `<audio>`, a `<video>` of a `blob:`). */
const MEDIA: Record<string, { kind: 'image' | 'audio' | 'video'; mime: string }> = {
  png: { kind: 'image', mime: 'image/png' },
  jpg: { kind: 'image', mime: 'image/jpeg' },
  jpeg: { kind: 'image', mime: 'image/jpeg' },
  jfif: { kind: 'image', mime: 'image/jpeg' },
  gif: { kind: 'image', mime: 'image/gif' },
  webp: { kind: 'image', mime: 'image/webp' },
  avif: { kind: 'image', mime: 'image/avif' },
  bmp: { kind: 'image', mime: 'image/bmp' },
  ico: { kind: 'image', mime: 'image/x-icon' },
  // As an image (`<img>`), never in the page: no script of it runs, nothing it names is loaded.
  svg: { kind: 'image', mime: 'image/svg+xml' },
  mp3: { kind: 'audio', mime: 'audio/mpeg' },
  wav: { kind: 'audio', mime: 'audio/wav' },
  ogg: { kind: 'audio', mime: 'audio/ogg' },
  oga: { kind: 'audio', mime: 'audio/ogg' },
  opus: { kind: 'audio', mime: 'audio/ogg' },
  m4a: { kind: 'audio', mime: 'audio/mp4' },
  aac: { kind: 'audio', mime: 'audio/aac' },
  flac: { kind: 'audio', mime: 'audio/flac' },
  mp4: { kind: 'video', mime: 'video/mp4' },
  m4v: { kind: 'video', mime: 'video/mp4' },
  webm: { kind: 'video', mime: 'video/webm' },
  ogv: { kind: 'video', mime: 'video/ogg' },
  mov: { kind: 'video', mime: 'video/quicktime' },
};

const DOCS: Record<string, PreviewKind> = {
  pdf: 'pdf',
  docx: 'docx',
  md: 'markdown',
  markdown: 'markdown',
  csv: 'csv',
  tsv: 'csv',
};

/**
 * The document formats Escouade has no preview of, and why (the key of the reason in `editor.preview.why`): they open
 * in a pane that says so, not in the text editor (they are archives or binaries, which it would show as noise).
 */
export const NO_PREVIEW: Record<string, 'office' | 'legacy' | 'sheet' | 'tiff'> = {
  odt: 'office',
  odp: 'office',
  pptx: 'office',
  rtf: 'legacy',
  doc: 'legacy',
  xls: 'legacy',
  ppt: 'legacy',
  xlsx: 'sheet',
  ods: 'sheet',
  tif: 'tiff',
  tiff: 'tiff',
};

/** The extension of `path` (its last `.` part, lower case); '' for none, or a dot file (`.env`). */
export function extOf(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? '';
  const at = name.lastIndexOf('.');
  return at > 0 ? name.slice(at + 1).toLowerCase() : '';
}

/** The preview of `path`, by its extension; null for a file the text editor shows. */
export function previewOf(path: string): { kind: PreviewKind; mime?: string } | null {
  const ext = extOf(path);
  const media = MEDIA[ext];
  if (media) return media;
  if (DOCS[ext]) return { kind: DOCS[ext] };
  if (NO_PREVIEW[ext]) return { kind: 'none' };
  return null;
}

/** Whether the preview is what the file opens in (a click, a link, Ctrl+P), rather than the text editor first. */
export const opensInPreview = (kind: PreviewKind) => kind !== 'csv';

/** The glob of the files of each kind, as VS Code's editor resolver reads it (`*.{png,jpg}`). */
export function globOf(kind: PreviewKind): string {
  const exts = [
    ...Object.entries(MEDIA)
      .filter(([, m]) => m.kind === kind)
      .map(([e]) => e),
    ...Object.entries(DOCS)
      .filter(([, k]) => k === kind)
      .map(([e]) => e),
    ...(kind === 'none' ? Object.keys(NO_PREVIEW) : []),
  ];
  // Both cases: the resolver's globs are case-sensitive on a case-sensitive file system.
  const all = [...new Set(exts.flatMap((e) => [e, e.toUpperCase()]))];
  return `**/*.{${all.join(',')}}`;
}

/** The preview kinds, in the order they are registered. */
export const KINDS: PreviewKind[] = ['image', 'audio', 'video', 'pdf', 'docx', 'markdown', 'csv', 'none'];

/** The largest file a preview reads (the core reads 64 MB at most of a file). */
export const MAX_PREVIEW_BYTES = 64 * 1024 * 1024;

/**
 * Where a link of a document leads, from the file `from` (its path in its source, `/` separated): another file of the
 * source (`rel`, with a line `#L12` or an anchor `#titre`), an anchor of the same document, the outside (an `https:`
 * or `mailto:` address, opened by Escouade's link opening), or nowhere (another scheme, a path out of the source).
 */
export type LinkTarget =
  | { to: 'file'; rel: string; line?: number; anchor?: string }
  | { to: 'anchor'; anchor: string }
  | { to: 'outside'; href: string }
  | { to: 'none' };

export function linkOf(from: string, href: string): LinkTarget {
  const h = href.trim();
  if (!h) return { to: 'none' };
  if (h.startsWith('#')) return h.length > 1 ? { to: 'anchor', anchor: decode(h.slice(1)) } : { to: 'none' };
  if (/^(https?:|mailto:)/i.test(h)) return { to: 'outside', href: h };
  // Another scheme (`javascript:`, `file:`, `data:`, an app's), a path from the root of a drive or of the host
  // (`/etc`, `C:\`, `//host`): nowhere.
  if (/^[a-z][a-z0-9+.-]*:/i.test(h) || h.startsWith('/') || h.startsWith('\\')) return { to: 'none' };
  const [pathPart, hash = ''] = h.split('#', 2);
  const rel = resolveRel(from, decode(pathPart.split('?')[0]));
  if (rel === null) return { to: 'none' };
  const line = /^L(\d+)/i.exec(hash);
  if (line) return { to: 'file', rel, line: Number(line[1]) };
  return hash ? { to: 'file', rel, anchor: decode(hash) } : { to: 'file', rel };
}

function decode(s: string): string {
  try {
    return decodeURIComponent(s);
  } catch {
    return s;
  }
}

/** `href` (relative) from the folder of `from`, in the source; null when it goes out of it. */
export function resolveRel(from: string, href: string): string | null {
  const parts = from.split('/').slice(0, -1);
  for (const p of href.replace(/\\/g, '/').split('/')) {
    if (p === '' || p === '.') continue;
    if (p === '..') {
      if (!parts.length) return null;
      parts.pop();
    } else parts.push(p);
  }
  return parts.length ? parts.join('/') : null;
}
