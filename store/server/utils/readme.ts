import { posix } from 'node:path';
import MarkdownIt from 'markdown-it';
import sanitizeHtml from 'sanitize-html';
import { manifestOf, type Lang } from './checks';
import { imageInfo, reencode, type ImageFormat } from './images';
import type { ArchiveRead } from './tar';

// A plugin's README and documentation pages, as the store shows them (docs/STORE.md §3.2, §7): the markdown rendered,
// then cut down to a short list of tags and attributes (no script, no style, no event, no frame, no form, no class
// but a code block's language, no link but http, https and mailto); the images of the repository copied on the store
// (re-encoded, their metadata dropped) and served from it, every other image dropped: a visitor loads nothing from a
// third party (`img-src 'self'`), and nobody follows them through a README. Done once per version, from its frozen
// archive; nothing of the plugin runs.

export interface DocPage {
  /** The page's path in the plugin's folder (`docs/setup.md`, `docs/fr/01-installer.md`). */
  path: string;
  title: string;
  html: string;
  /** The language of a page of `docs/fr` or `docs/en` (ESC-366); none for a folder without them. */
  lang?: Lang;
}

export interface Docs {
  /** The README of each language: README.fr.md and README.en.md, or README.md for both. */
  readme: Partial<Record<Lang, string>>;
  /** The pages of `store.docs`, in the order of their names: those of its `fr` and `en` folders, each with its
   * language, the page showing the visitor's (the other otherwise); without them, every page, for both. */
  pages: DocPage[];
  /** The images copied, by the name they are served under (`<sha256>.png`). */
  images: string[];
}

/** How the store keeps a copied image: under its SHA-256, as the screenshots; gives the name it is served under. */
export type KeepImage = (bytes: Buffer, format: ImageFormat) => Promise<string>;

export const DOC_PAGES_MAX = 20;
export const IMAGES_MAX = 40;
export const IMAGE_BYTES = 5 * 1024 * 1024;
/** Where the store serves the copied images (server/routes/files/[name].get.ts). */
export const FILES = '/files/';

const README = /^README(?:\.(fr|en))?\.md$/i;
const LANGS = ['fr', 'en'] as const;
const REL = 'nofollow ugc noopener noreferrer';

const md = new MarkdownIt({ html: true, linkify: true, typographer: false });

/** The path inside the plugin an image or a link names, from the markdown file `from`; null if it leaves the folder. */
export function insidePath(src: string, from: string, repo: string, folder = ''): string | null {
  let s = src.trim();
  // The repository's own files, named by their address on GitHub: read from the frozen archive instead.
  const own = new RegExp(
    `^https://(?:raw\\.githubusercontent\\.com/${escape(repo)}/[^/]+/|github\\.com/${escape(repo)}/(?:raw|blob)/[^/]+/)(.+)$`,
    'i',
  ).exec(s.replace(/[?#].*$/, ''));
  if (own) s = `/${own[1]}`;
  else if (/^[a-z][a-z0-9+.-]*:|^\/\//i.test(s)) return null;
  // A plugin in a folder (ESC-313): a path from the repository's root must lead into that folder.
  if (folder && s.startsWith('/')) {
    if (!s.startsWith(`/${folder}/`)) return null;
    s = s.slice(folder.length + 1);
  }
  s = s.replace(/[?#].*$/, '');
  if (!s) return null;
  try {
    s = decodeURIComponent(s);
  } catch {
    return null;
  }
  if (s.includes('\\') || s.includes('\0')) return null;
  const joined = s.startsWith('/') ? posix.normalize(s.slice(1)) : posix.join(posix.dirname(from), s);
  if (joined === '.' || joined.startsWith('../') || joined === '..' || posix.isAbsolute(joined)) return null;
  return joined;
}

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** The options of the cut: what a README may hold once rendered. `img` and `a` are rewritten by `transform`. */
function options(transform: { img: sanitizeHtml.Transformer; a: sanitizeHtml.Transformer }): sanitizeHtml.IOptions {
  return {
    allowedTags: [
      'h1',
      'h2',
      'h3',
      'h4',
      'h5',
      'h6',
      'p',
      'br',
      'hr',
      'a',
      'img',
      'ul',
      'ol',
      'li',
      'blockquote',
      'pre',
      'code',
      'em',
      'strong',
      'b',
      'i',
      'del',
      's',
      'kbd',
      'sup',
      'sub',
      'table',
      'thead',
      'tbody',
      'tr',
      'th',
      'td',
      'details',
      'summary',
    ],
    allowedAttributes: {
      a: ['href', 'title', 'rel'],
      img: ['src', 'alt', 'title', 'width', 'height', 'loading'],
      ol: ['start'],
      th: ['align'],
      td: ['align'],
      code: ['class'],
    },
    allowedClasses: { code: ['language-*'] },
    allowedSchemes: ['http', 'https', 'mailto'],
    allowedSchemesByTag: { img: [] },
    allowedSchemesAppliedToAttributes: ['href', 'src'],
    allowProtocolRelative: false,
    disallowedTagsMode: 'discard',
    // What a tag dropped held is dropped with it when it is code or style; the text of the others stays.
    nonTextTags: ['script', 'style', 'textarea', 'option', 'noscript', 'iframe', 'object', 'embed', 'svg', 'math', 'template', 'title'],
    transformTags: { img: transform.img, a: transform.a },
    exclusiveFilter: (frame) => frame.tag === 'img' && !frame.attribs.src,
  };
}

/** Renders `markdown` (the file `from` of the plugin), its images rewritten by `images` (inside path → served name). */
export function renderMarkdown(
  markdown: string,
  ctx: {
    from: string;
    repo: string;
    commit: string;
    /** The plugin's folder in the repository, '' at its root (ESC-313). */
    path?: string;
    images: Map<string, { name: string; width: number; height: number }>;
    /** The documentation pages shown on the store: a link to one of them opens its tab there (ESC-366). */
    pages?: ReadonlySet<string>;
  },
): { html: string; wanted: string[] } {
  const wanted: string[] = [];
  const raw = md.render(markdown);
  const html = sanitizeHtml(
    raw,
    options({
      img: (_tag, attribs) => {
        const path = attribs.src ? insidePath(attribs.src, ctx.from, ctx.repo, ctx.path) : null;
        if (path) wanted.push(path);
        const kept = path ? ctx.images.get(path) : undefined;
        if (!kept) return { tagName: 'img', attribs: {} };
        const out: Record<string, string> = {
          src: `${FILES}${kept.name}`,
          width: String(kept.width),
          height: String(kept.height),
          loading: 'lazy',
        };
        if (attribs.alt) out.alt = attribs.alt;
        if (attribs.title) out.title = attribs.title;
        return { tagName: 'img', attribs: out };
      },
      a: (_tag, attribs) => {
        const href = (attribs.href ?? '').trim();
        const out: Record<string, string> = {};
        if (attribs.title) out.title = attribs.title;
        if (/^(https?:|mailto:)/i.test(href)) out.href = href;
        else if (href && !/^[a-z][a-z0-9+.-]*:|^\/\//i.test(href) && !href.startsWith('#')) {
          // A file of the repository: on GitHub, at the commit the store froze.
          const path = insidePath(href, ctx.from, ctx.repo, ctx.path);
          // Another page of its documentation, or its README: on the store, in the plugin's own tabs (ESC-366).
          if (path && ctx.pages?.has(path)) {
            out.href = `?tab=docs&page=${encodeURIComponent(path)}`;
            return { tagName: 'a', attribs: out };
          }
          if (path && ctx.pages && !path.includes('/') && README.test(path)) {
            out.href = '?';
            return { tagName: 'a', attribs: out };
          }
          const full = ctx.path && path ? `${ctx.path}/${path}` : path;
          if (full) out.href = `https://github.com/${ctx.repo}/blob/${ctx.commit}/${full.split('/').map(encodeURIComponent).join('/')}`;
        }
        if (out.href) out.rel = REL;
        return { tagName: 'a', attribs: out };
      },
    }),
  );
  return { html, wanted };
}

/** The first heading of a markdown page (not a `#` of a code block), or its file name without its order (`01-`). */
export function titleOf(markdown: string, path: string): string {
  const tokens = md.parse(markdown, {});
  const at = tokens.findIndex((t) => t.type === 'heading_open');
  const inline = at >= 0 ? (tokens[at + 1]?.children ?? []) : [];
  const heading = inline
    .map((c) => (c.type === 'text' || c.type === 'code_inline' ? c.content : ''))
    .join('')
    .trim();
  const name = posix.basename(path).replace(/\.md$/i, '');
  return heading || name.replace(/^\d+[-_.\s]+/, '').trim() || name;
}

/** The folder `store.docs` names (`.` for the plugin's own), or null if it names none inside the plugin. */
export function docsFolder(manifest: Record<string, unknown> | null): string | null {
  const store = (manifest?.store ?? {}) as { docs?: unknown };
  if (typeof store.docs !== 'string' || !store.docs.trim() || store.docs.includes('\\')) return null;
  const dir = posix.normalize(store.docs.trim().replace(/\/+$/, '') || '.');
  return dir === '..' || dir.startsWith('../') || posix.isAbsolute(dir) ? null : dir;
}

/**
 * The documentation pages of `store.docs` (docs/STORE.md §3.2), in the order of their names, `DOC_PAGES_MAX` per
 * language: with a `fr` or an `en` folder in it, the pages under them, each with its language (the others are not
 * read); without, its markdown pages for both languages, as before ESC-366. A README is never a page.
 */
export function docPages(read: ArchiveRead, manifest: Record<string, unknown> | null): { path: string; lang?: Lang }[] {
  const dir = docsFolder(manifest);
  if (!dir) return [];
  const base = dir === '.' ? '' : `${dir}/`;
  const all = [...read.files.keys()].filter((p) => /\.md$/i.test(p) && p.startsWith(base) && !README.test(p)).sort();
  const under = (l: Lang) => all.filter((p) => p.startsWith(`${base}${l}/`));
  if (LANGS.some((l) => under(l).length))
    return LANGS.flatMap((l) =>
      under(l)
        .slice(0, DOC_PAGES_MAX)
        .map((path) => ({ path, lang: l })),
    );
  return all
    .filter((p) => dir !== '.' || !p.includes('/'))
    .slice(0, DOC_PAGES_MAX)
    .map((path) => ({ path }));
}

/**
 * Renders the README and the documentation of a version from its archive: each page twice, the first time to learn
 * which images it names, the second with them copied.
 */
export async function renderDocs(read: ArchiveRead, ctx: { repo: string; commit: string; path?: string; keep: KeepImage }): Promise<Docs> {
  const readmes = [...read.files.keys()].filter((p) => !p.includes('/') && README.test(p));
  const byLang = (l: Lang) => readmes.find((p) => README.exec(p)![1]?.toLowerCase() === l) ?? readmes.find((p) => !README.exec(p)![1]);
  const listed = docPages(read, manifestOf(read) as Record<string, unknown> | null);
  const pages = listed.map((p) => p.path);
  const shown = new Set(pages);
  const sources = [...new Set([byLang('fr'), byLang('en'), ...pages].filter((p): p is string => !!p))];
  const text = (p: string) => read.files.get(p)!.toString('utf8');

  // 1. Which images the pages name, inside the plugin.
  const none = new Map();
  const wanted = new Set<string>();
  for (const p of sources)
    for (const w of renderMarkdown(text(p), { from: p, repo: ctx.repo, commit: ctx.commit, path: ctx.path, images: none }).wanted)
      wanted.add(w);

  // 2. Each one copied once: a PNG, JPEG or WebP of the archive, re-encoded; the others dropped.
  const images = new Map<string, { name: string; width: number; height: number }>();
  for (const path of [...wanted].slice(0, IMAGES_MAX)) {
    const bytes = read.files.get(path);
    if (!bytes || bytes.length > IMAGE_BYTES) continue;
    const info = imageInfo(bytes);
    if (!info) continue;
    try {
      const out = await reencode(bytes, info.format);
      const size = imageInfo(out) ?? info;
      images.set(path, { name: await ctx.keep(out, size.format), width: size.width, height: size.height });
    } catch {
      // An image sharp cannot read is dropped, as one of another format.
    }
  }

  // 3. The pages, their images in place.
  const render = (p: string) =>
    renderMarkdown(text(p), { from: p, repo: ctx.repo, commit: ctx.commit, path: ctx.path, images, pages: shown }).html;
  const readme: Docs['readme'] = {};
  for (const l of LANGS) {
    const p = byLang(l);
    if (p) readme[l] = render(p);
  }
  return {
    readme,
    pages: listed.map(({ path, lang }) => ({ path, title: titleOf(text(path), path), html: render(path), ...(lang ? { lang } : {}) })),
    images: [...new Set([...images.values()].map((i) => i.name))],
  };
}
